//! Helpers for studio mode, where each section of the video is a pre-rendered film (Remotion)
//! dropped in as a full-frame `video` beat:
//!
//! - `slots`: the sections in OUTPUT time, frame-exact, with the beat line that places a film there
//! - `timing`: the kept words of a slot (or any stretch) in the slot's own frame clock
//! - `check-render`: mechanical checks of a rendered film against its slot
//! - `sheet-video`: contact sheets of a render (transparency shown over the canvas)
//! - `canvas`: one frame of the dot-grid canvas, for matching it pixel for pixel

use crate::Project;
use anyhow::{anyhow, bail, Context, Result};
use cutroom_core::compose::Seg;
use cutroom_core::resolve::{clock, OutWord, Plan};
use cutroom_core::spec::LayoutMode;
use cutroom_gfx::{Gfx, Pixmap, Theme};
use serde::Serialize;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------------------------- slots

#[derive(Serialize, Clone, Debug)]
pub struct Slots {
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub duration_ms: f64,
    pub frames: u64,
    /// Changes whenever the cuts, the chapter cards or the frame rate change: a film rendered for
    /// an older hash no longer fits its slot.
    pub cuts_hash: String,
    /// Output pixels `[x, y, w, h]`: where cutroom draws the panel, the camera tile and the
    /// badge. Films leave the tile and badge areas clear.
    pub panel_rect: [i32; 4],
    pub camera_rect: [i32; 4],
    pub badge_rect: Option<[i32; 4]>,
    pub canvas: Canvas,
    pub slots: Vec<Slot>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Slot {
    /// `intro`, then `ch01`, `ch02`... (the chapter files' names).
    pub id: String,
    /// Chapter number (0 for the intro).
    pub n: usize,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Output window (ms), frame-aligned: `start_ms` = `frame0 / fps`.
    pub start_ms: f64,
    pub end_ms: f64,
    /// First output frame and the exact number of frames a film must have.
    pub frame0: u64,
    pub frames: u64,
    /// First and last kept word in the slot.
    pub first_word: Option<String>,
    pub last_word: Option<String>,
    pub words: usize,
    /// What the raw recording has in each stretch of the slot (slot-clock frames).
    pub layout: Vec<SlotLayout>,
    /// The beat that places a film here, frame-exact (paste into a beats file; set `src`).
    pub beat: serde_json::Value,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SlotLayout {
    pub f0: u64,
    pub f1: u64,
    /// Layout cutroom would draw without a film: `panel`, `camera`, `screen`.
    pub mode: &'static str,
    /// The recording has a screen capture / a camera here.
    pub screen: bool,
    pub camera: bool,
}

/// The dot-grid canvas, so a film can draw the same one (see docs/CANVAS.md).
#[derive(Serialize, Clone, Debug)]
pub struct Canvas {
    pub bg: String,
    pub dot: String,
    /// Grid pitch and dot radius in output px.
    pub pitch_px: f64,
    pub dot_radius_px: f64,
    /// Drift speed (px/s, output px) down-right; dots sit at (k*pitch + off, m*pitch + off) with
    /// off = drift_px_s * t_s, t = OUTPUT time (0 at the start of the video).
    pub drift_px_s: f64,
    /// Drift is quantized to this fraction of a pixel.
    pub phase_step_px: f64,
}

pub fn canvas_info(plan: &Plan) -> Canvas {
    canvas_info_for(&plan.style, plan.height)
}

pub fn canvas_info_for(style: &std::collections::BTreeMap<String, String>, height: u32) -> Canvas {
    let th = Theme::from_style(style);
    let scale = height as f64 / 1080.0;
    let hex = |c: [u8; 3]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
    Canvas {
        bg: hex(th.canvas),
        dot: hex(th.dot),
        pitch_px: (th.dot_pitch as f64 * scale).round().max(4.0),
        dot_radius_px: 1.6 * scale,
        drift_px_s: th.drift_px_s as f64 * scale,
        phase_step_px: 1.0 / 8.0,
    }
}

/// FNV-1a over the time map, the cuts and the frame rate (stable across builds and machines).
pub fn cuts_hash(plan: &Plan) -> String {
    let v = serde_json::json!({"map": plan.map, "cuts": plan.cuts, "fps": plan.fps, "cards": plan.chapters.iter().map(|c| c.card).collect::<Vec<_>>()});
    let s = v.to_string();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn rect_px(r: [f64; 4], w: u32, h: u32) -> [i32; 4] {
    let x = (r[0] * w as f64).round() as i32;
    let y = (r[1] * h as f64).round() as i32;
    let x1 = ((r[0] + r[2]) * w as f64).round() as i32;
    let y1 = ((r[1] + r[3]) * h as f64).round() as i32;
    [x, y, (x1 - x) & !1, (y1 - y) & !1]
}

pub fn slots(p: &Project, segs: &[Seg], src_pattern: &str) -> Slots {
    let plan = &p.plan;
    let fps = plan.fps;
    let fd = 1000.0 / fps;
    let dur = plan.duration_ms();
    let (_, total) = cutroom_render::window_frames(0.0, dur, fps);
    // (id, n, title, group, start, end)
    let mut raw: Vec<(String, usize, String, Option<String>, f64, f64)> = vec![];
    let starts: Vec<f64> = plan.chapters.iter().map(|c| c.at_ms).collect();
    if let Some(&first) = starts.first() {
        if first >= fd {
            raw.push(("intro".into(), 0, plan.output.intro.clone(), None, 0.0, first));
        }
    } else {
        raw.push(("intro".into(), 0, plan.output.intro.clone(), None, 0.0, dur));
    }
    for (i, c) in plan.chapters.iter().enumerate() {
        let s = c.card.map(|x| x.1).unwrap_or(c.at_ms);
        let e = starts.get(i + 1).copied().unwrap_or(dur);
        raw.push((format!("ch{:02}", c.n), c.n, c.title.clone(), c.group.as_ref().map(|g| g.title.clone()), s, e));
    }
    let out_slots = raw
        .into_iter()
        .filter_map(|(id, n, title, group, s, e)| {
            let (k0, k1) = cutroom_render::window_frames(s, e, fps);
            if k1 <= k0 {
                return None;
            }
            let start_ms = k0 as f64 * fd;
            let end_ms = k1 as f64 * fd;
            let words: Vec<&OutWord> = plan.words.iter().filter(|w| w.start_ms >= start_ms && w.start_ms < end_ms).collect();
            let layout = slot_layout(segs, k0, k1, fd);
            // a hair before the first frame, so float rounding can never drop it (see docs)
            let at_s = (start_ms - 0.001) / 1000.0;
            let hold_ms = (k1 - k0) as f64 * fd;
            let beat = serde_json::json!({
                "at": format!("o:{at_s:.7}"),
                "lead": 0,
                "hold": format!("{hold_ms:.4}ms"),
                "kind": "video",
                "src": src_pattern.replace("{id}", &id),
                "fit": "frame",
                "alpha": false,
                "fade": false,
            });
            Some(Slot {
                id,
                n,
                title,
                group,
                start_ms,
                end_ms,
                frame0: k0,
                frames: k1 - k0,
                first_word: words.first().map(|w| format!("w{}", w.id)),
                last_word: words.last().map(|w| format!("w{}", w.id)),
                words: words.len(),
                layout,
                beat,
            })
        })
        .collect();
    let (panel, tile) = (rect_px(plan.layout.panel, plan.width, plan.height), rect_px(plan.layout.camera, plan.width, plan.height));
    Slots {
        fps,
        width: plan.width,
        height: plan.height,
        duration_ms: dur,
        frames: total,
        cuts_hash: cuts_hash(plan),
        panel_rect: panel,
        camera_rect: tile,
        badge_rect: plan.output.badge.then(|| cutroom_render::frame::badge_rect(tile, plan.height)),
        canvas: canvas_info(plan),
        slots: out_slots,
    }
}

fn slot_layout(segs: &[Seg], k0: u64, k1: u64, fd: f64) -> Vec<SlotLayout> {
    let mut out: Vec<SlotLayout> = vec![];
    for s in segs {
        let (a, b) = cutroom_render::window_frames(s.out0, s.out1, 1000.0 / fd);
        let (a, b) = (a.max(k0), b.min(k1));
        if b <= a || s.card.is_some() {
            continue;
        }
        let mode = match s.requested {
            LayoutMode::Panel if s.empty_panel => "camera",
            LayoutMode::Panel => "panel",
            LayoutMode::Camera => "camera",
            LayoutMode::Screen => "screen",
        };
        let e = SlotLayout { f0: a - k0, f1: b - k0, mode, screen: s.screen.is_some(), camera: s.camera.is_some() };
        match out.last_mut() {
            Some(l) if l.f1 == e.f0 && l.mode == e.mode && l.screen == e.screen && l.camera == e.camera => l.f1 = e.f1,
            _ => out.push(e),
        }
    }
    out
}

pub fn slots_text(s: &Slots) -> String {
    let mut o = format!("{} slots | {} fps | {} frames ({}) | cuts {}\n", s.slots.len(), s.fps, s.frames, clock(s.duration_ms), s.cuts_hash);
    for x in &s.slots {
        let screen: u64 = x.layout.iter().filter(|l| l.screen).map(|l| l.f1 - l.f0).sum();
        o.push_str(&format!(
            "{:<6} {:>8}-{:<8} f{:>6} +{:>5} frames  {:>3} words {}-{}  screen {:>3}%  {}{}\n",
            x.id,
            clock(x.start_ms),
            clock(x.end_ms),
            x.frame0,
            x.frames,
            x.words,
            x.first_word.as_deref().unwrap_or("-"),
            x.last_word.as_deref().unwrap_or("-"),
            screen * 100 / x.frames.max(1),
            x.title,
            x.group.as_ref().map(|g| format!("  [{g}]")).unwrap_or_default()
        ));
    }
    o
}

// ------------------------------------------------------------------------------------ timing

#[derive(Serialize, Clone, Debug)]
pub struct Timing {
    pub slot: Option<String>,
    /// Output ms of slot frame 0, and the clock: `f = round((ms - slot0_ms) * fps / 1000)`.
    pub slot0_ms: f64,
    pub fps: f64,
    pub frames: u64,
    pub words: Vec<TWord>,
    pub sentences: Vec<TSentence>,
}

#[derive(Serialize, Clone, Debug)]
pub struct TWord {
    pub id: usize,
    pub text: String,
    pub ms: f64,
    pub end_ms: f64,
    pub f: i64,
    pub f_end: i64,
    /// What cutroom shows there without a film (`panel`, `camera`, `screen`), and whether the
    /// recording has a screen capture at that moment.
    pub view: &'static str,
    pub screen: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct TSentence {
    pub from: String,
    pub to: String,
    pub f: i64,
    pub f_end: i64,
    pub text: String,
}

pub fn timing(p: &Project, segs: &[Seg], slot: Option<&Slot>, range: Option<(f64, f64)>) -> Result<Timing> {
    let plan = &p.plan;
    let fps = plan.fps;
    let fd = 1000.0 / fps;
    let (s0, s1, frames, id) = match (slot, range) {
        (Some(s), _) => (s.start_ms, s.end_ms, s.frames, Some(s.id.clone())),
        (None, Some((a, b))) => {
            let (k0, k1) = cutroom_render::window_frames(a, b, fps);
            (k0 as f64 * fd, k1 as f64 * fd, k1.saturating_sub(k0), None)
        }
        _ => bail!("give --slot or --from/--to"),
    };
    let f = |ms: f64| ((ms - s0) * fps / 1000.0).round() as i64;
    let words: Vec<TWord> = plan
        .words
        .iter()
        .filter(|w| w.start_ms >= s0 && w.start_ms < s1)
        .map(|w| {
            let mid = (w.start_ms + w.end_ms) / 2.0;
            let seg = segs.iter().find(|s| s.out0 <= mid && mid < s.out1);
            let view = match seg {
                Some(s) if s.card.is_some() => "card",
                Some(s) => match s.mode {
                    LayoutMode::Panel => "panel",
                    LayoutMode::Camera => "camera",
                    LayoutMode::Screen => "screen",
                },
                None => "camera",
            };
            TWord { id: w.id, text: w.text.clone(), ms: round2(w.start_ms), end_ms: round2(w.end_ms), f: f(w.start_ms), f_end: f(w.end_ms), view, screen: seg.is_some_and(|s| s.screen.is_some()) }
        })
        .collect();
    let mut sentences = vec![];
    let mut a = 0;
    for i in 0..words.len() {
        let end = words[i].text.trim_end_matches(['"', '\'', ')', '\u{201D}']).ends_with(['.', '?', '!']);
        let pause = words.get(i + 1).is_some_and(|n| n.ms - words[i].end_ms > 700.0);
        if end || pause || i + 1 == words.len() {
            sentences.push(TSentence {
                from: format!("w{}", words[a].id),
                to: format!("w{}", words[i].id),
                f: words[a].f,
                f_end: words[i].f_end,
                text: words[a..=i].iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "),
            });
            a = i + 1;
        }
    }
    Ok(Timing { slot: id, slot0_ms: round2(s0), fps, frames, words, sentences })
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn timing_text(t: &Timing, per_word: bool) -> String {
    let mut o = format!(
        "{}slot0 {} ms | {} fps | {} frames | f = round((ms - {}) * {} / 1000)\n",
        t.slot.as_ref().map(|s| format!("{s} | ")).unwrap_or_default(),
        t.slot0_ms,
        t.fps,
        t.frames,
        t.slot0_ms,
        t.fps
    );
    if per_word {
        for w in &t.words {
            o.push_str(&format!("f{:05}-{:05}  w{:<6} {:<7}{} {}\n", w.f, w.f_end, w.id, w.view, if w.screen { "+scr" } else { "    " }, w.text));
        }
    } else {
        for s in &t.sentences {
            let view = t.words.iter().find(|w| format!("w{}", w.id) == s.from).map(|w| w.view).unwrap_or("");
            o.push_str(&format!("f{:05}-{:05}  {}-{}  [{}]  {}\n", s.f, s.f_end, s.from, s.to, view, s.text));
        }
    }
    o
}

// ----------------------------------------------------------------------------- render frames

/// Decode `video` at `w x h`, every frame, straight RGBA. Returns (frames, fps, has_alpha).
fn decode_all(video: &Path, w: u32, h: u32) -> Result<(Vec<Vec<u8>>, f64, bool)> {
    let (_, _, fps, _) = cutroom_render::feed::probe(video)?;
    let alpha = cutroom_render::feed::probe_alpha(video).unwrap_or(false);
    let o = std::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(video)
        .args(["-an", "-vf", &format!("scale={w}:{h}:flags=area,format=rgba"), "-f", "rawvideo", "-"])
        .output()
        .context("decoding with ffmpeg")?;
    if !o.status.success() {
        bail!("ffmpeg could not decode {}: {}", video.display(), String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or(""));
    }
    let n = (w * h * 4) as usize;
    Ok((o.stdout.chunks_exact(n).map(|c| c.to_vec()).collect(), fps, alpha))
}

#[derive(Serialize, Clone, Debug)]
pub struct RenderCheck {
    pub render: String,
    pub slot: Option<String>,
    pub frames: u64,
    pub expected_frames: Option<u64>,
    pub fps: f64,
    pub alpha: bool,
    /// Share of the frame an overlay covers (alpha > 6%): mean and max over all frames.
    pub alpha_cover_mean: Option<f64>,
    pub alpha_cover_max: Option<f64>,
    pub findings: Vec<RFinding>,
    pub ok: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct RFinding {
    /// error | warn | info
    pub sev: &'static str,
    /// frame_count | empty_panel | black | static | camera_rect | badge_rect | alpha_cover | beat_lead | fps
    pub kind: &'static str,
    /// Slot-clock frames (inclusive start, exclusive end).
    pub f0: u64,
    pub f1: u64,
    pub msg: String,
}

/// Analysis size: 1/8 of 1080p keeps every check meaningful (the dot grid blurs away) and decodes
/// a minute of film in about a second.
const AW: u32 = 240;
const AH: u32 = 136;

pub struct CheckOpts<'a> {
    pub slot: Option<&'a Slot>,
    pub plan: Option<&'a Plan>,
    pub job: Option<PathBuf>,
    pub timing: Option<&'a Timing>,
}

pub fn check_render(video: &Path, o: &CheckOpts) -> Result<RenderCheck> {
    let (frames, fps, alpha) = decode_all(video, AW, AH)?;
    let n = frames.len() as u64;
    let mut f: Vec<RFinding> = vec![];
    let fdur = |k: u64| k as f64 / fps.max(1.0);
    if let Some(s) = o.slot {
        let d = n as i64 - s.frames as i64;
        if d != 0 {
            f.push(RFinding {
                sev: if d.abs() > 2 { "error" } else { "warn" },
                kind: "frame_count",
                f0: 0,
                f1: n,
                msg: format!("{n} frames, the slot has {}: {}", s.frames, if d < 0 { format!("{} short (the last frame would hold)", -d) } else { format!("{d} too many (dropped)") }),
            });
        }
    }
    if let Some(p) = o.plan {
        if (fps - p.fps).abs() > 0.01 {
            f.push(RFinding { sev: "error", kind: "fps", f0: 0, f1: n, msg: format!("{fps:.3} fps, the edit runs at {}", p.fps) });
        }
    }
    let plan_rects = o.plan.map(|p| {
        let tile = rect_px(p.layout.camera, p.width, p.height);
        let badge = p.output.badge.then(|| cutroom_render::frame::badge_rect(tile, p.height));
        let sc = |r: [i32; 4]| [r[0] as f32 * AW as f32 / p.width as f32, r[1] as f32 * AH as f32 / p.height as f32, r[2] as f32 * AW as f32 / p.width as f32, r[3] as f32 * AH as f32 / p.height as f32];
        (sc(rect_px(p.layout.panel, p.width, p.height)), sc(tile), badge.map(sc), Theme::from_style(&p.style).canvas)
    });
    let (panel, tile, badge, bg) = plan_rects.unwrap_or(([2.4, 2.4, 184.2, 131.2], [191.2, 28.9, 44.2, 78.2], Some([191.2, 4.0, 44.2, 22.0]), [0x12; 3]));
    let bg_l = luma(bg[0], bg[1], bg[2]);
    // content: a pixel that differs from the canvas (opaque films) or is visible (overlays)
    let content = |px: &[u8]| -> bool {
        if alpha {
            px[3] > 16
        } else {
            (luma(px[0], px[1], px[2]) - bg_l).abs() > 14.0
        }
    };
    let frac = |fr: &[u8], r: [f32; 4], inset: f32| -> f64 {
        let (x0, y0) = ((r[0] + inset).max(0.0) as u32, (r[1] + inset).max(0.0) as u32);
        let (x1, y1) = (((r[0] + r[2] - inset) as u32).min(AW), ((r[1] + r[3] - inset) as u32).min(AH));
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let mut c = 0u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * AW + x) * 4) as usize;
                if content(&fr[i..i + 4]) {
                    c += 1;
                }
            }
        }
        c as f64 / ((x1 - x0) * (y1 - y0)) as f64
    };
    let mut empty = vec![false; n as usize];
    let mut black = vec![false; n as usize];
    let mut still = vec![false; n as usize];
    let mut in_tile = vec![false; n as usize];
    let mut in_badge = vec![false; n as usize];
    let mut covers: Vec<f64> = vec![];
    for (k, fr) in frames.iter().enumerate() {
        let full = [0.0, 0.0, AW as f32, AH as f32];
        if !alpha {
            empty[k] = frac(fr, panel, 1.0) < 0.004;
            let mean: f64 = fr.as_chunks::<4>().0.iter().map(|p| luma(p[0], p[1], p[2])).sum::<f64>() / (AW * AH) as f64;
            black[k] = mean < 8.0;
        } else {
            covers.push(frac(fr, full, 0.0));
        }
        // a black dip is reported once, as black
        if !black[k] {
            in_tile[k] = frac(fr, tile, 1.0) > 0.02;
            if let Some(b) = badge {
                in_badge[k] = frac(fr, b, 1.0) > 0.02;
            }
        }
    }
    // A hold: every frame stays within a hair of the frame the hold began on. Comparing with the
    // start (not the previous frame) catches slow push-ins and small highlights appearing, which
    // change too little from one frame to the next.
    let mut anchor = 0usize;
    for k in 1..frames.len() {
        let (fr, a) = (&frames[k], &frames[anchor]);
        let changed = fr.as_chunks::<4>().0.iter().zip(a.as_chunks::<4>().0.iter()).filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| (*x as i32 - *y as i32).abs() > 10)).count();
        if (changed as f64) / ((AW * AH) as f64) < 0.002 {
            still[k] = true;
        } else {
            anchor = k;
        }
    }
    let runs = |v: &[bool], min: u64| -> Vec<(u64, u64)> {
        let mut out = vec![];
        let mut a: Option<u64> = None;
        for (k, &x) in v.iter().enumerate().chain(std::iter::once((v.len(), &false))) {
            match (x, a) {
                (true, None) => a = Some(k as u64),
                (false, Some(s)) => {
                    if k as u64 - s >= min {
                        out.push((s, k as u64));
                    }
                    a = None;
                }
                _ => {}
            }
        }
        out
    };
    let sec = |s: f64| (s * fps).round() as u64;
    for (a, b) in runs(&empty, sec(1.0)) {
        f.push(RFinding { sev: "warn", kind: "empty_panel", f0: a, f1: b, msg: format!("panel shows only the canvas for {:.1}s", fdur(b - a)) });
    }
    for (a, b) in runs(&black, 2) {
        f.push(RFinding { sev: "error", kind: "black", f0: a, f1: b, msg: format!("black for {} frames", b - a) });
    }
    for (a, b) in runs(&still, sec(6.0)) {
        f.push(RFinding { sev: "warn", kind: "static", f0: a.saturating_sub(1), f1: b, msg: format!("nothing moves for {:.1}s (beats change every 2.5-6 s)", fdur(b - a + 1)) });
    }
    for (a, b) in runs(&in_tile, 1) {
        f.push(RFinding { sev: "error", kind: "camera_rect", f0: a, f1: b, msg: format!("content under the camera tile for {} frames (it will be hidden)", b - a) });
    }
    for (a, b) in runs(&in_badge, 1) {
        f.push(RFinding { sev: "error", kind: "badge_rect", f0: a, f1: b, msg: format!("content under the badge for {} frames (it will be hidden)", b - a) });
    }
    let (cmean, cmax) = if alpha && !covers.is_empty() {
        let m = covers.iter().sum::<f64>() / covers.len() as f64;
        let mx = covers.iter().cloned().fold(0.0, f64::max);
        let big: Vec<bool> = covers.iter().map(|c| *c > 0.6).collect();
        for (a, b) in runs(&big, 1) {
            f.push(RFinding { sev: "warn", kind: "alpha_cover", f0: a, f1: b, msg: format!("the overlay covers over 60% of the frame for {} frames: the screen under it is hidden", b - a) });
        }
        (Some(round4(m)), Some(round4(mx)))
    } else {
        (None, None)
    };
    if let (Some(job), Some(t)) = (&o.job, o.timing) {
        f.extend(beat_leads(job, t)?);
    }
    let ok = !f.iter().any(|x| x.sev == "error");
    Ok(RenderCheck { render: video.display().to_string(), slot: o.slot.map(|s| s.id.clone()), frames: n, expected_frames: o.slot.map(|s| s.frames), fps, alpha, alpha_cover_mean: cmean, alpha_cover_max: cmax, findings: f, ok })
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

fn luma(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
}

/// `job.json` `beats`: `[{"f": 120, "word": "w812"}, ...]` (also `frame`, `w`, `at`). A beat should
/// land 2-4 frames before the word it is keyed to.
fn beat_leads(job: &Path, t: &Timing) -> Result<Vec<RFinding>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(job).with_context(|| format!("reading {}", job.display()))?)?;
    let Some(beats) = v.get("beats").and_then(|b| b.as_array()) else { return Ok(vec![]) };
    let mut out = vec![];
    for b in beats {
        let fr = b.get("f").or_else(|| b.get("frame")).and_then(|x| x.as_i64());
        let word = b.get("word").or_else(|| b.get("w")).or_else(|| b.get("at"));
        let id = match word {
            Some(serde_json::Value::String(s)) => s.trim_start_matches('w').parse::<usize>().ok(),
            Some(serde_json::Value::Number(n)) => n.as_u64().map(|x| x as usize),
            _ => None,
        };
        let (Some(fr), Some(id)) = (fr, id) else { continue };
        let Some(w) = t.words.iter().find(|w| w.id == id) else {
            out.push(RFinding { sev: "warn", kind: "beat_lead", f0: fr.max(0) as u64, f1: fr.max(0) as u64 + 1, msg: format!("beat at f{fr} is keyed to w{id}, which is not in this slot (cut?)") });
            continue;
        };
        let lead = w.f - fr;
        if !(1..=6).contains(&lead) {
            out.push(RFinding {
                sev: "warn",
                kind: "beat_lead",
                f0: fr.max(0) as u64,
                f1: fr.max(0) as u64 + 1,
                msg: format!("beat at f{fr} lands {} the word w{id} {:?} (f{}); aim for 2-4 frames before it", if lead > 0 { format!("{lead} frames before") } else { format!("{} frames after", -lead) }, w.text, w.f),
            });
        }
    }
    Ok(out)
}

pub fn check_text(c: &RenderCheck) -> String {
    let mut o = format!(
        "## check-render: {}\n\n- {} frames{} at {:.3} fps, {}\n",
        Path::new(&c.render).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        c.frames,
        c.expected_frames.map(|e| format!(" (slot: {e})")).unwrap_or_default(),
        c.fps,
        if c.alpha { "transparent overlay" } else { "opaque film" }
    );
    if let (Some(m), Some(x)) = (c.alpha_cover_mean, c.alpha_cover_max) {
        o.push_str(&format!("- covers {:.0}% of the frame on average, {:.0}% at most\n", m * 100.0, x * 100.0));
    }
    o.push_str(&format!("- {}\n\n", if c.ok { "no errors" } else { "ERRORS: fix before placing" }));
    for x in &c.findings {
        o.push_str(&format!("- **{}** {} f{}-{}: {}\n", x.sev, x.kind, x.f0, x.f1, x.msg));
    }
    o
}

// ------------------------------------------------------------------------------ sheet-video

/// Frames of `video` at `times` (seconds) tiled into one image, transparency shown over the
/// canvas, each labelled with its time and frame.
pub fn sheet_video(video: &Path, times: &[f64], cols: u32, thumb_w: u32, theme: Theme) -> Result<Pixmap> {
    let (vw, vh, fps, _) = cutroom_render::feed::probe(video)?;
    let thumb_h = ((thumb_w as f64 * vh as f64 / vw as f64).round() as u32) & !1;
    let alpha = cutroom_render::feed::probe_alpha(video).unwrap_or(false);
    let gfx = Gfx::new(theme, thumb_w, thumb_h, Path::new("."))?;
    let thumbs: Vec<Result<Pixmap>> = std::thread::scope(|s| {
        let hs: Vec<_> = times
            .iter()
            .map(|&t| {
                let gfx = &gfx;
                s.spawn(move || -> Result<Pixmap> {
                    let o = std::process::Command::new("ffmpeg")
                        .args(["-nostdin", "-v", "error", "-ss", &format!("{:.4}", (t - 0.25 / fps).max(0.0)), "-i"])
                        .arg(video)
                        .args(["-frames:v", "1", "-an", "-vf", &format!("scale={thumb_w}:{thumb_h}:flags=bicubic,format=rgba"), "-f", "rawvideo", "-"])
                        .output()?;
                    let mut pm = Pixmap::new(thumb_w, thumb_h).ok_or_else(|| anyhow!("thumb size"))?;
                    gfx.canvas(&mut pm, t * 1000.0);
                    if o.stdout.len() == (thumb_w * thumb_h * 4) as usize {
                        let mut fr = o.stdout;
                        if alpha {
                            for p in fr.as_chunks_mut::<4>().0.iter_mut() {
                                let a = p[3] as u32;
                                for c in &mut p[..3] {
                                    *c = ((*c as u32 * a + 127) / 255) as u8;
                                }
                            }
                        }
                        cutroom_render::blit::blend_over(&mut pm, 0, 0, &fr, thumb_w, thumb_h, 1.0, None);
                    }
                    let k = (t * fps).round() as i64;
                    gfx.label(&mut pm, 6.0, 6.0, (thumb_h as f32 * 0.075).max(10.0), &format!("{} · f{k}", clock(t * 1000.0)));
                    Ok(pm)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(anyhow!("thumb thread panicked")))).collect()
    });
    let rows = (times.len() as u32).div_ceil(cols.max(1));
    let gap = 6;
    let mut pm = Pixmap::new(cols * (thumb_w + gap) + gap, rows * (thumb_h + gap) + gap).ok_or_else(|| anyhow!("sheet too large"))?;
    for px in pm.data_mut().as_chunks_mut::<4>().0.iter_mut() {
        px.copy_from_slice(&[40, 40, 40, 255]);
    }
    for (i, t) in thumbs.into_iter().enumerate() {
        let t = t?;
        let x = gap + (i as u32 % cols) * (thumb_w + gap);
        let y = gap + (i as u32 / cols) * (thumb_h + gap);
        cutroom_render::blit::blit_rounded(&mut pm, x as i32, y as i32, t.data(), thumb_w, thumb_h, None);
    }
    Ok(pm)
}

/// Times for `--every N` seconds over a video.
pub fn every(video: &Path, n: f64) -> Result<Vec<f64>> {
    let (_, _, fps, dur) = cutroom_render::feed::probe(video)?;
    let mut out = vec![];
    let mut t = 0.0;
    let last = dur / 1000.0 - 1.0 / fps;
    while t <= last + 1e-9 {
        out.push(t);
        t += n.max(0.05);
    }
    if out.last().is_some_and(|l| last - l > n * 0.5) {
        out.push(last);
    }
    Ok(out)
}

/// `--at`: seconds, `f<frame>`, or `m:ss.s`.
pub fn parse_times(s: &str, fps: f64) -> Result<Vec<f64>> {
    s.split(',')
        .map(|x| {
            let x = x.trim();
            if let Some(k) = x.strip_prefix('f') {
                return Ok(k.parse::<f64>()? / fps);
            }
            if let Some((m, sec)) = x.split_once(':') {
                return Ok(m.parse::<f64>()? * 60.0 + sec.parse::<f64>()?);
            }
            Ok(x.parse::<f64>()?)
        })
        .collect()
}

// ------------------------------------------------------------------------------------ canvas

pub fn canvas_png(theme: Theme, w: u32, h: u32, t_ms: f64, out: &Path) -> Result<()> {
    let gfx = Gfx::new(theme, w, h, Path::new("."))?;
    let mut pm = Pixmap::new(w, h).ok_or_else(|| anyhow!("bad size"))?;
    gfx.canvas(&mut pm, t_ms);
    cutroom_render::save_png(&pm, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cutroom_core::model::{Clip, Media, Source, Track, Word};

    fn project(edit: &str) -> (crate::Project, PathBuf) {
        let text = "Hello and welcome. Tip one is to talk. Tip two is to plan. Tip three is to check everything twice before you ship.";
        let words: Vec<Word> = text.split_whitespace().enumerate().map(|(i, t)| Word { text: t.into(), start_ms: 1000.0 + i as f64 * 500.0, end_ms: 1000.0 + i as f64 * 500.0 + 350.0 }).collect();
        let dur = 1000.0 + words.len() as f64 * 500.0 + 1000.0;
        let src = Source {
            version: 1,
            name: "t".into(),
            duration_ms: dur,
            media: vec![
                Media { id: "cam".into(), path: "cam.mp4".into(), width: 1920, height: 1080, fps: 30.0, duration_ms: dur },
                Media { id: "scr".into(), path: "scr.mp4".into(), width: 2560, height: 1440, fps: 30.0, duration_ms: dur },
            ],
            clips: vec![
                Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: dur, media_in_ms: 0.0, take: 0 },
                Clip { track: Track::Screen, media: "scr".into(), start_ms: 6000.0, dur_ms: 3000.0, media_in_ms: 0.0, take: 0 },
            ],
            words,
            ..Default::default()
        };
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let k = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cutroom-studio-test-{k}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        src.save(&dir.join("source.json")).unwrap();
        std::fs::write(dir.join("edit.yaml"), edit).unwrap();
        (crate::open(&dir, None).unwrap(), dir)
    }

    const EDIT: &str = "source: source.json\noutput: {chapter_cards: insert, card_ms: 1500, kicker: Tip}\ncuts:\n  - {from: w1, to: w2}\nchapters:\n  - {at: w3, title: Talk, group: Easy}\n  - {at: w8, title: Plan, group: Easy}\n  - {at: w13, title: Check, group: Hard}\n";

    #[test]
    fn slots_are_frame_exact_and_cover_the_video() {
        let (p, _) = project(EDIT);
        let segs = cutroom_core::compose::segments(&p.plan, &p.src);
        let s = slots(&p, &segs, "films/{id}.mp4");
        let ids: Vec<&str> = s.slots.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["intro", "ch01", "ch02", "ch03"]);
        let fd = 1000.0 / s.fps;
        for x in &s.slots {
            assert_eq!(x.start_ms, x.frame0 as f64 * fd);
            assert_eq!(x.frames, ((x.end_ms - x.start_ms) / fd).round() as u64);
            // the beat line resolves to exactly the slot's frames
            let at: f64 = x.beat["at"].as_str().unwrap().trim_start_matches("o:").parse::<f64>().unwrap() * 1000.0;
            let hold = cutroom_core::spec::parse_dur_ms(x.beat["hold"].as_str().unwrap()).unwrap();
            assert_eq!(cutroom_render::window_frames(at, at + hold, s.fps), (x.frame0, x.frame0 + x.frames), "{}", x.id);
            assert_eq!(x.beat["src"], format!("films/{}.mp4", x.id));
        }
        // the cards sit between slots: ch01's slot starts where its card ends
        let card = p.plan.chapters[0].card.unwrap();
        assert!((s.slots[1].start_ms - card.1).abs() < fd);
        assert_eq!(s.slots[1].group.as_deref(), Some("Easy"));
        // ch02 has the screen recording for part of it
        assert!(s.slots.iter().any(|x| x.layout.iter().any(|l| l.screen)));
        assert_eq!(s.slots.last().unwrap().end_ms, s.frames as f64 * fd);
    }

    #[test]
    fn cuts_hash_follows_the_cuts() {
        let (a, _) = project(EDIT);
        let (b, _) = project(&EDIT.replace("  - {from: w1, to: w2}\n", "  - {from: w1, to: w1}\n"));
        assert_ne!(cuts_hash(&a.plan), cuts_hash(&b.plan));
        let (c, _) = project(&format!("{EDIT}beats:\n  - {{at: w4, kind: title, text: Hi}}\n"));
        assert_eq!(cuts_hash(&a.plan), cuts_hash(&c.plan), "beats do not move the slots");
    }

    #[test]
    fn timing_is_in_the_slot_clock() {
        let (p, _) = project(EDIT);
        let segs = cutroom_core::compose::segments(&p.plan, &p.src);
        let s = slots(&p, &segs, "");
        let t = timing(&p, &segs, Some(&s.slots[1]), None).unwrap();
        assert_eq!(t.words[0].id, 3);
        assert!(t.words[0].f >= 0 && t.words[0].f < 10, "the first word lands at the start of its slot: f{}", t.words[0].f);
        for w in &t.words {
            assert_eq!(w.f, ((w.ms - t.slot0_ms) * t.fps / 1000.0).round() as i64);
        }
        assert_eq!(t.sentences[0].text, "Tip one is to talk.");
    }

    #[test]
    fn times_parse() {
        assert_eq!(parse_times("1.5, f30, 1:02.5", 30.0).unwrap(), vec![1.5, 1.0, 62.5]);
        assert!(parse_times("x", 30.0).is_err());
    }

    #[test]
    fn beat_leads_are_checked() {
        let t = Timing {
            slot: None,
            slot0_ms: 0.0,
            fps: 30.0,
            frames: 300,
            words: vec![TWord { id: 7, text: "go".into(), ms: 1000.0, end_ms: 1200.0, f: 30, f_end: 36, view: "panel", screen: false }],
            sentences: vec![],
        };
        let job = std::env::temp_dir().join(format!("cutroom-job-{}.json", std::process::id()));
        std::fs::write(&job, r#"{"beats": [{"f": 27, "word": "w7"}, {"frame": 33, "w": 7}, {"f": 10, "word": "w99"}]}"#).unwrap();
        let f = beat_leads(&job, &t).unwrap();
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f[0].msg.contains("after the word"));
        assert!(f[1].msg.contains("not in this slot"));
    }
}
