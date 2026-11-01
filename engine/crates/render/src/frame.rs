//! The compositor: draws output frame k from the shared segment list (`cutroom_core::compose`).

use crate::blit::{self, RoundMask};
use crate::feed::{cover, Feed, FeedKey, Geometry};
use anyhow::Result;
use cutroom_core::compose::Seg;
use cutroom_core::model::{Clip, Source};
use cutroom_core::resolve::ResolvedBeat;
use cutroom_core::spec::{BeatKind, Fit, Lane, LayoutMode, RectN, VideoFit};
use cutroom_core::Plan;
use cutroom_gfx::{Gfx, Pixmap, Rect, ScreenMap, Theme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Shared, read-only render inputs.
pub struct Ctx<'a> {
    pub plan: &'a Plan,
    pub src: &'a Source,
    pub segs: &'a [Seg],
    pub dir: &'a Path,
    /// media id or beat file path -> (width, height)
    pub dims: HashMap<String, (u32, u32)>,
    /// `video` beat file path (keyed like `dims`) -> transparency and length.
    pub clips: HashMap<String, ClipInfo>,
}

/// What the renderer knows about a `video` beat's file.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClipInfo {
    /// The file carries transparency (used when the beat does not say `alpha:`).
    pub alpha: bool,
    pub dur_ms: f64,
}

#[derive(Default, Clone, Debug)]
pub struct Stats {
    pub frames: u64,
    pub compose_ns: u128,
    pub decode_ns: u128,
    pub feeds_opened: u64,
}

const ZOOM_EASE_MS: f64 = 450.0;
const MAX_SKIP_MS: f64 = 2500.0;

fn px_rect(r: &RectN, w: u32, h: u32) -> [i32; 4] {
    let x = (r[0] * w as f64).round() as i32;
    let y = (r[1] * h as f64).round() as i32;
    let x1 = ((r[0] + r[2]) * w as f64).round() as i32;
    let y1 = ((r[1] + r[3]) * h as f64).round() as i32;
    // even sizes for the decoders
    [x, y, (x1 - x) & !1, (y1 - y) & !1]
}

fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    // ease in-out cubic
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

pub struct Compositor<'a> {
    ctx: &'a Ctx<'a>,
    pub gfx: Gfx,
    pub pm: Pixmap,
    w: u32,
    h: u32,
    panel: [i32; 4],
    tile: [i32; 4],
    panel_mask: RoundMask,
    tile_mask: RoundMask,
    screen: Option<Feed>,
    camera: Option<Feed>,
    stage_video: Option<Feed>,
    /// Decoder for the `fit: frame` video beat on screen.
    frame_video: Option<Feed>,
    zbuf: Vec<u8>,
    seg_i: usize,
    active: Vec<usize>,
    next_beat: usize,
    last_t: f64,
    pub stats: Stats,
    /// This worker decodes big screen recordings on the hardware engines.
    pub hw: bool,
}

impl<'a> Compositor<'a> {
    pub fn new(ctx: &'a Ctx<'a>) -> Result<Compositor<'a>> {
        let plan = ctx.plan;
        let (w, h) = (plan.width, plan.height);
        let gfx = Gfx::new(Theme::from_style(&plan.style), w, h, ctx.dir)?;
        let panel = px_rect(&plan.layout.panel, w, h);
        let tile = px_rect(&plan.layout.camera, w, h);
        let r = gfx.slot_radius();
        Ok(Compositor {
            panel_mask: RoundMask::new(panel[2] as u32, panel[3] as u32, r),
            tile_mask: RoundMask::new(tile[2] as u32, tile[3] as u32, r),
            ctx,
            gfx,
            pm: Pixmap::new(w, h).expect("frame size"),
            w,
            h,
            panel,
            tile,
            screen: None,
            camera: None,
            stage_video: None,
            frame_video: None,
            zbuf: vec![],
            seg_i: 0,
            active: vec![],
            next_beat: 0,
            last_t: f64::NEG_INFINITY,
            stats: Stats::default(),
            hw: false,
        })
    }

    fn seg_at(&mut self, t: f64) -> &'a Seg {
        let segs = self.ctx.segs;
        if self.seg_i >= segs.len() || segs[self.seg_i].out0 > t {
            self.seg_i = segs.partition_point(|s| s.out1 <= t).min(segs.len().saturating_sub(1));
        }
        while self.seg_i + 1 < segs.len() && segs[self.seg_i].out1 <= t {
            self.seg_i += 1;
        }
        &segs[self.seg_i]
    }

    fn update_active(&mut self, t: f64) {
        let beats = &self.ctx.plan.beats;
        if t < self.last_t {
            self.active.clear();
            self.next_beat = 0;
        }
        self.last_t = t;
        while self.next_beat < beats.len() && beats[self.next_beat].start_ms <= t {
            self.active.push(self.next_beat);
            self.next_beat += 1;
        }
        self.active.retain(|&i| beats[i].end_ms > t && beats[i].start_ms <= t);
    }

    fn dims(&self, key: &str) -> (u32, u32) {
        self.ctx.dims.get(key).copied().unwrap_or((1920, 1080))
    }

    /// Pull the frame of `clip` at source time `s` from feed `slot` with geometry `geo`.
    #[allow(clippy::too_many_arguments)]
    fn pull(slot: &mut Option<Feed>, stats: &mut Stats, path: PathBuf, geo: Geometry, contain: bool, looped: bool, hw: bool, media_ms: f64, remaining_ms: f64, fps: f64) -> Result<()> {
        let key = FeedKey { path, geo, contain, looped, hw, alpha: false };
        Self::pull_key(slot, stats, key, media_ms, media_ms, remaining_ms, fps)
    }

    /// [`Self::pull`] with a full key; a reopened feed decodes from `seek_ms` of the file.
    fn pull_key(slot: &mut Option<Feed>, stats: &mut Stats, key: FeedKey, media_ms: f64, seek_ms: f64, remaining_ms: f64, fps: f64) -> Result<()> {
        let reuse = match slot {
            Some(f) if f.key == key => f.advance_to(media_ms, MAX_SKIP_MS),
            _ => false,
        };
        if !reuse {
            let frames = (remaining_ms * fps / 1000.0).ceil().max(1.0) as u64 + 2;
            let looped = key.looped;
            *slot = Some(Feed::open_at(key, media_ms, seek_ms, fps, if looped { u64::MAX / 4 } else { frames })?);
            stats.feeds_opened += 1;
        }
        Ok(())
    }

    /// Draw output frame `k`. The result is in `self.pm`.
    pub fn frame(&mut self, k: u64) -> Result<()> {
        let plan = self.ctx.plan;
        let src = self.ctx.src;
        let fps = plan.fps;
        let t = k as f64 * 1000.0 / fps;
        let seg = self.seg_at(t);
        self.update_active(t);
        let t0 = Instant::now();
        let mut dec = 0u128;
        self.gfx.canvas(&mut self.pm, t);

        let s = seg.src_at(t);
        let card_insert = seg.card.is_some() && seg.src0.is_none();
        if card_insert {
            let ch = &plan.chapters[seg.card.unwrap()];
            self.gfx.chapter_card(&mut self.pm, ch, plan.chapters.len(), t);
            self.stats.frames += 1;
            self.stats.compose_ns += t0.elapsed().as_nanos();
            return Ok(());
        }
        let stage = seg.stage.map(|i| &plan.beats[i]);
        // ---- scene beats with `place: frame`: drawn over the whole frame, no panel slot ----
        let full_frame_stage = stage.is_some_and(|b| b.kind.full_frame());
        let screen_clip: Option<&Clip> = seg.screen.map(|i| &src.clips[i]);
        let cam_clip: Option<&Clip> = seg.camera.map(|i| &src.clips[i]);

        // ---- the screen slot ----
        let (slot, show_screen) = match seg.mode {
            LayoutMode::Panel => (self.panel, stage.is_none() && screen_clip.is_some()),
            LayoutMode::Screen => ([0, 0, self.w as i32, self.h as i32], screen_clip.is_some() && stage.is_none()),
            LayoutMode::Camera => ([0, 0, 0, 0], false),
        };
        let mut map: Option<ScreenMap> = None;
        if seg.mode == LayoutMode::Panel && !full_frame_stage {
            self.gfx.slot_shadow(&mut self.pm, rect(self.panel), self.gfx.slot_radius());
        }
        if let (true, Some(c), Some(s)) = (show_screen, screen_clip, s) {
            let media = src.media(&c.media).unwrap();
            let (sw, sh) = self.dims(&c.media);
            let (zoom_rect, zoom_p) = self.zoom_at(t);
            let contain = plan.layout.fit == Fit::Contain || seg.mode == LayoutMode::Screen;
            let (dw, dh) = (slot[2] as u32, slot[3] as u32);
            let zf = if zoom_rect.is_some() { 2.0f64.min(sw as f64 / dw as f64).max(1.0) } else { 1.0 };
            let geo = if contain && zoom_rect.is_none() { Geometry { crop: [0, 0, sw & !1, sh & !1], w: dw, h: dh } } else { cover(sw, sh, None, ((dw as f64 * zf) as u32) & !1, ((dh as f64 * zf) as u32) & !1) };
            let m = c.media_at(s);
            let d0 = Instant::now();
            Self::pull(&mut self.screen, &mut self.stats, media.path.clone(), geo.clone(), contain && zoom_rect.is_none(), false, self.hw && sw * sh > 4_000_000, m, c.end_ms() - s, fps)?;
            let frame = self.screen.as_mut().unwrap().next_frame();
            dec += d0.elapsed().as_nanos();
            let mask = if seg.mode == LayoutMode::Panel { Some(&self.panel_mask) } else { None };
            match zoom_rect {
                None => {
                    blit::blit_rounded(&mut self.pm, slot[0], slot[1], frame, dw, dh, mask);
                    // screen-normalized -> px
                    map = Some(if contain {
                        let sc = (dw as f32 / sw as f32).min(dh as f32 / sh as f32);
                        let (fw, fh) = (sw as f32 * sc, sh as f32 * sc);
                        ScreenMap { x: slot[0] as f32 + (dw as f32 - fw) / 2.0, y: slot[1] as f32 + (dh as f32 - fh) / 2.0, sx: fw, sy: fh, clip: rect(slot) }
                    } else {
                        let sc = dw as f32 / geo.crop[2] as f32;
                        ScreenMap { x: slot[0] as f32 - geo.crop[0] as f32 * sc, y: slot[1] as f32 - geo.crop[1] as f32 * sc, sx: sw as f32 * sc, sy: sh as f32 * sc, clip: rect(slot) }
                    });
                }
                Some(z) => {
                    // decoded frame covers the source crop `geo.crop` at geo.w x geo.h; zoom into z
                    let k = geo.w as f64 / geo.crop[2] as f64; // decoded px per source px
                    let full = [0.0, 0.0, geo.w as f64, geo.h as f64];
                    let zr = aspect_fit([(z[0] * sw as f64 - geo.crop[0] as f64) * k, (z[1] * sh as f64 - geo.crop[1] as f64) * k, z[2] * sw as f64 * k, z[3] * sh as f64 * k], dw as f64 / dh as f64, full);
                    let p = ease(zoom_p);
                    let vr = [full[0] + (zr[0] - full[0]) * p, full[1] + (zr[1] - full[1]) * p, full[2] + (zr[2] - full[2]) * p, full[3] + (zr[3] - full[3]) * p];
                    let mut out = std::mem::take(&mut self.zbuf);
                    blit::resample(frame, geo.w, geo.h, vr.map(|v| v as f32), dw, dh, &mut out);
                    blit::blit_rounded(&mut self.pm, slot[0], slot[1], &out, dw, dh, mask);
                    self.zbuf = out;
                    // map: source px -> decoded px (k) -> view (dw / vr.w)
                    let v = dw as f64 / vr[2];
                    map = Some(ScreenMap {
                        x: (slot[0] as f64 + (-(geo.crop[0] as f64) * k - vr[0]) * v) as f32,
                        y: (slot[1] as f64 + (-(geo.crop[1] as f64) * k - vr[1]) * v) as f32,
                        sx: (sw as f64 * k * v) as f32,
                        sy: (sh as f64 * k * v) as f32,
                        clip: rect(slot),
                    });
                }
            }
            // privacy blur
            if let Some(mp) = map {
                let block = ((self.w as f32) * 0.011).max(8.0) as u32;
                for &i in &self.active {
                    if let BeatKind::Blur { rects } = &plan.beats[i].kind {
                        for r in rects {
                            let (x0, y0) = mp.px(r[0], r[1]);
                            let (x1, y1) = mp.px(r[0] + r[2], r[1] + r[3]);
                            let cx0 = x0.max(slot[0] as f32);
                            let cy0 = y0.max(slot[1] as f32);
                            let cx1 = x1.min((slot[0] + slot[2]) as f32);
                            let cy1 = y1.min((slot[1] + slot[3]) as f32);
                            if cx1 > cx0 && cy1 > cy0 {
                                // blocks of half the rect height: a line of text becomes two rows of flat squares
                                let b = block.max(((cy1 - cy0) * 0.5) as u32);
                                blit::mosaic(&mut self.pm, [cx0 as i32, cy0 as i32, (cx1 - cx0).ceil() as i32, (cy1 - cy0).ceil() as i32], b);
                            }
                        }
                    }
                }
            }
        } else if self.screen.is_some() && !show_screen {
            // keep the decoder only if the screen comes back soon (advance_to decides)
        }

        // ---- stage beat in the panel ----
        if let (Some(b), LayoutMode::Panel | LayoutMode::Screen) = (stage, seg.mode) {
            let panel = if seg.mode == LayoutMode::Panel && !full_frame_stage { rect(self.panel) } else { rect([0, 0, self.w as i32, self.h as i32]) };
            if let BeatKind::Video { fit: VideoFit::Panel, .. } = &b.kind {
                dec += self.video_panel(b, seg.mode, t)?;
            } else {
                let mut media: Option<Pixmap> = None;
                if let Some(p) = self.play(b, t) {
                    // `fit: card`: decoded letterboxed at the card size; gfx draws the card
                    let (sw, sh) = self.dims(&p.key);
                    let (mw, mh) = self.gfx.stage_media_size(panel);
                    let geo = Geometry { crop: [0, 0, sw & !1, sh & !1], w: mw, h: mh };
                    let key = FeedKey { path: p.path, geo, contain: true, looped: p.looped, hw: false, alpha: p.alpha };
                    let d0 = Instant::now();
                    Self::pull_key(&mut self.stage_video, &mut self.stats, key, p.media_ms, p.seek_ms, p.remaining_ms, fps)?;
                    let f = self.stage_video.as_mut().unwrap().next_frame();
                    dec += d0.elapsed().as_nanos();
                    let mut px = Pixmap::new(mw, mh).unwrap();
                    px.data_mut().copy_from_slice(f);
                    media = Some(px);
                }
                self.gfx.stage(&mut self.pm, panel, b, t, media.as_ref());
            }
        }

        // ---- full-frame video beat (fit: frame): over the screen and stage, under the camera ----
        dec += self.video_frame(t)?;

        // ---- camera ----
        if let (Some(c), Some(s)) = (cam_clip, s) {
            let media = src.media(&c.media).unwrap();
            let (cw, ch) = self.dims(&c.media);
            let m = c.media_at(s);
            let (target, mask) = match seg.mode {
                LayoutMode::Panel => (Some(self.tile), true),
                LayoutMode::Camera => (Some([0, 0, self.w as i32, self.h as i32]), false),
                LayoutMode::Screen => (None, false),
            };
            if let Some(tr) = target {
                let region = if mask { plan.layout.camera_crop } else { None };
                let geo = cover(cw, ch, region, tr[2] as u32, tr[3] as u32);
                if mask {
                    self.gfx.slot_shadow(&mut self.pm, rect(tr), self.gfx.slot_radius());
                }
                let d0 = Instant::now();
                Self::pull(&mut self.camera, &mut self.stats, media.path.clone(), geo, false, false, false, m, c.end_ms() - s, fps)?;
                let f = self.camera.as_mut().unwrap().next_frame();
                dec += d0.elapsed().as_nanos();
                blit::blit_rounded(&mut self.pm, tr[0], tr[1], f, tr[2] as u32, tr[3] as u32, if mask { Some(&self.tile_mask) } else { None });
            }
        }

        // ---- badge ----
        if plan.output.badge && seg.mode == LayoutMode::Panel {
            if let Some(ch) = plan.chapter_at(t) {
                // card badge (step, title, progress) filling the space above the camera tile
                let gap = (self.h as f32 * 0.02).round();
                let bh = (self.tile[1] as f32 - gap - self.h as f32 * 0.065).clamp(self.h as f32 * 0.05, self.h as f32 * 0.13).round();
                let r = Rect::from_xywh(self.tile[0] as f32, self.tile[1] as f32 - bh - gap, self.tile[2] as f32, bh).unwrap();
                self.gfx.badge(&mut self.pm, r, ch, plan.chapters.len(), t);
            }
        }

        // ---- overlays ----
        for &i in &self.active {
            let b = &plan.beats[i];
            if b.lane != Lane::Overlay || matches!(b.kind, BeatKind::Video { .. }) {
                continue;
            }
            let needs_screen = matches!(b.kind, BeatKind::Ring { .. } | BeatKind::Highlight { .. } | BeatKind::Arrow { .. });
            let fallback = ScreenMap { x: slot[0] as f32, y: slot[1] as f32, sx: slot[2].max(1) as f32, sy: slot[3].max(1) as f32, clip: rect(if slot[2] > 0 { slot } else { [0, 0, self.w as i32, self.h as i32] }) };
            match (needs_screen, map) {
                (true, None) => continue,
                (_, Some(m)) => self.gfx.overlay(&mut self.pm, &m, b, t),
                (false, None) => self.gfx.overlay(&mut self.pm, &fallback, b, t),
            }
        }

        // ---- captions ----
        if plan.output.captions {
            let lo = plan.words.partition_point(|w| w.end_ms < t - 8000.0);
            let hi = plan.words.partition_point(|w| w.start_ms < t + 8000.0);
            let area = if seg.mode == LayoutMode::Panel { rect(self.panel) } else { rect([0, 0, self.w as i32, self.h as i32]) };
            self.gfx.captions(&mut self.pm, area, &plan.words[lo..hi], t);
        }

        // ---- overlay chapter card ----
        if let Some(ci) = seg.card {
            self.gfx.chapter_card(&mut self.pm, &plan.chapters[ci], plan.chapters.len(), t);
        }
        let _ = (&self.screen, &self.stage_video, &self.frame_video);
        self.stats.frames += 1;
        self.stats.decode_ns += dec;
        self.stats.compose_ns += t0.elapsed().as_nanos() - dec;
        Ok(())
    }

    /// Active zoom rect and its eased progress (0..1) at output time t.
    fn zoom_at(&self, t: f64) -> (Option<RectN>, f64) {
        for &i in &self.active {
            let b = &self.ctx.plan.beats[i];
            if let BeatKind::Zoom { rect } = b.kind {
                let p = ((t - b.start_ms) / ZOOM_EASE_MS).min((b.end_ms - t) / ZOOM_EASE_MS).clamp(0.0, 1.0);
                return (Some(rect), p);
            }
        }
        (None, 0.0)
    }
}

// ------------------------------------------------------------------------- video beats
// `fit: card | panel | frame`, `alpha`, `loop`. Card clips go through gfx (the card look);
// panel and frame clips are composited here, premultiplied, with a short fade at each end.

/// Fade in and out of `panel` / `frame` clips (ms).
const CLIP_FADE_MS: f64 = 180.0;

/// One video beat at one output instant.
struct Play {
    path: PathBuf,
    /// `dims` / `clips` key.
    key: String,
    alpha: bool,
    looped: bool,
    /// Clip time the frame is for (caller time; keeps counting through loops).
    media_ms: f64,
    /// Where a (re)opened decoder starts in the file.
    seek_ms: f64,
    remaining_ms: f64,
    opacity: f32,
}

impl<'a> Compositor<'a> {
    /// The clip, timing and blend of video beat `b` at `t` (None for other kinds).
    fn play(&self, b: &ResolvedBeat, t: f64) -> Option<Play> {
        let BeatKind::Video { src: file, from, fit, alpha, looped, .. } = &b.kind else { return None };
        let path = self.ctx.dir.join(file);
        let key = path.display().to_string();
        let info = self.ctx.clips.get(&key).copied().unwrap_or_default();
        let looped = looped.unwrap_or(fit.loops_by_default());
        let media_ms = from.unwrap_or(0.0) * 1000.0 + (t - b.start_ms);
        let frame_ms = 1000.0 / self.ctx.plan.fps;
        let seek_ms = match (looped, info.dur_ms > 0.0) {
            (true, true) => media_ms.rem_euclid(info.dur_ms),
            // past the end: decode the last frames and hold them
            (false, true) => media_ms.min((info.dur_ms - 2.0 * frame_ms).max(0.0)),
            _ => media_ms,
        };
        let opacity = if *fit == VideoFit::Card { 1.0 } else { ((t - b.start_ms).min(b.end_ms - t) / CLIP_FADE_MS).clamp(0.0, 1.0) as f32 };
        Some(Play { path, key, alpha: alpha.unwrap_or(info.alpha), looped, media_ms, seek_ms, remaining_ms: b.end_ms - t, opacity })
    }

    /// `fit: panel`: the clip fills the panel slot (cover), rounded like the screen recording.
    /// Returns the decode wait (ns).
    fn video_panel(&mut self, b: &ResolvedBeat, mode: LayoutMode, t: f64) -> Result<u128> {
        let Some(p) = self.play(b, t) else { return Ok(0) };
        let full = mode != LayoutMode::Panel;
        let r = if full { [0, 0, self.w as i32, self.h as i32] } else { self.panel };
        let (sw, sh) = self.dims(&p.key);
        let geo = cover(sw, sh, None, r[2] as u32, r[3] as u32);
        if p.alpha || p.opacity < 1.0 {
            self.gfx.panel_surface(&mut self.pm, rect(r), t);
        }
        let (w, h) = (geo.w, geo.h);
        let key = FeedKey { path: p.path, geo, contain: false, looped: p.looped, hw: false, alpha: p.alpha };
        let d0 = Instant::now();
        Self::pull_key(&mut self.stage_video, &mut self.stats, key, p.media_ms, p.seek_ms, p.remaining_ms, self.ctx.plan.fps)?;
        let f = self.stage_video.as_mut().unwrap().next_frame();
        let dec = d0.elapsed().as_nanos();
        let mask = if full { None } else { Some(&self.panel_mask) };
        if p.alpha || p.opacity < 1.0 {
            blit::blend_over(&mut self.pm, r[0], r[1], f, w, h, p.opacity, mask);
        } else {
            blit::blit_rounded(&mut self.pm, r[0], r[1], f, w, h, mask);
        }
        Ok(dec)
    }

    /// `fit: frame`: the latest-starting full-frame clip on screen, over everything drawn so far.
    /// Returns the decode wait (ns).
    fn video_frame(&mut self, t: f64) -> Result<u128> {
        let plan = self.ctx.plan;
        let Some(b) = self.active.iter().rev().map(|&i| &plan.beats[i]).find(|b| matches!(b.kind, BeatKind::Video { fit: VideoFit::Frame, .. })) else {
            return Ok(0);
        };
        let Some(p) = self.play(b, t) else { return Ok(0) };
        let (sw, sh) = self.dims(&p.key);
        let geo = cover(sw, sh, None, self.w, self.h);
        let (w, h) = (geo.w, geo.h);
        let key = FeedKey { path: p.path, geo, contain: false, looped: p.looped, hw: false, alpha: p.alpha };
        let d0 = Instant::now();
        Self::pull_key(&mut self.frame_video, &mut self.stats, key, p.media_ms, p.seek_ms, p.remaining_ms, plan.fps)?;
        let f = self.frame_video.as_mut().unwrap().next_frame();
        let dec = d0.elapsed().as_nanos();
        if p.alpha || p.opacity < 1.0 {
            blit::blend_over(&mut self.pm, 0, 0, f, w, h, p.opacity, None);
        } else {
            blit::blit_rounded(&mut self.pm, 0, 0, f, w, h, None);
        }
        Ok(dec)
    }
}

fn rect(r: [i32; 4]) -> Rect {
    Rect::from_xywh(r[0] as f32, r[1] as f32, r[2].max(1) as f32, r[3].max(1) as f32).unwrap()
}

/// Grow `r` to aspect `a` (w/h) around its centre, then keep it inside `bounds`.
fn aspect_fit(r: [f64; 4], a: f64, bounds: [f64; 4]) -> [f64; 4] {
    let (mut w, mut h) = (r[2], r[3]);
    if w / h > a {
        h = w / a;
    } else {
        w = h * a;
    }
    w = w.min(bounds[2]);
    h = h.min(bounds[3]);
    let cx = r[0] + r[2] / 2.0;
    let cy = r[1] + r[3] / 2.0;
    let x = (cx - w / 2.0).clamp(bounds[0], bounds[0] + bounds[2] - w);
    let y = (cy - h / 2.0).clamp(bounds[1], bounds[1] + bounds[3] - h);
    [x, y, w, h]
}
