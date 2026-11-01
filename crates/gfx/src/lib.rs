//! cutroom-gfx: frame-pure motion graphics. Every pixel is a function of (beat, output time).
//!
//! API CONTRACT (the renderer in crates/render calls exactly these; keep the signatures):
//! design units are 1920x1080 "stage pixels"; `Gfx::scale` = frame height / 1080.
//!
//! Stage beats lay out in panel space: the panel is 1920 units wide (1 unit = panel width /
//! 1920 px, ~0.77 frame px in the default layout) and as tall as its aspect gives. Overlays,
//! the chapter card, the badge and captions use frame design units (`scale`).
//!
//! Text in `title`, `bullets` and `quote` beats may mark accent words with stars:
//! `"Connect *Claude* to Poppy"`.
//!
//! Everything static (glyphs, text runs, beat layouts, surfaces, shadows, scaled images, the
//! dot strips) is cached behind mutexes, so one `Gfx` can be shared by render threads.

mod anim;
mod cache;
mod canvas;
mod chrome;
mod media;
mod numfmt;
mod overlay;
mod pal;
mod raster;
mod stage;
mod text;

use cutroom_core::resolve::{OutWord, ResolvedBeat, ResolvedChapter};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
pub use tiny_skia::{Pixmap, Rect};

use cache::Cache;
use pal::Pal;
use raster::{IRect, Img, Shadow, Sprite};

/// Colours and sizes. Defaults = Samin's dark Borumi look (#121212 canvas, 40px dot grid
/// drifting ~2.5 px/s, terracotta accent #E2774E, Inter).
#[derive(Clone, Debug)]
pub struct Theme {
    pub canvas: [u8; 3],
    pub dot: [u8; 3],
    pub dot_pitch: f32,
    pub drift_px_s: f32,
    pub accent: [u8; 3],
    pub ink: [u8; 3],
    pub muted: [u8; 3],
    pub card: [u8; 3],
}

impl Theme {
    /// Defaults with overrides from the spec's `style:` map (hex colours: accent, canvas, dot,
    /// ink, muted, card; numbers: dot_pitch, drift).
    pub fn from_style(style: &BTreeMap<String, String>) -> Theme {
        let mut t = Theme { canvas: [0x12; 3], dot: [0x2c; 3], dot_pitch: 40.0, drift_px_s: 2.5, accent: [0xE2, 0x77, 0x4E], ink: [0xF4, 0xF1, 0xEC], muted: [0xA3, 0x9C, 0x91], card: [0x1E, 0x1D, 0x1B] };
        for (k, v) in style {
            let key = k.trim().to_ascii_lowercase();
            let slot = match key.as_str() {
                "accent" => Some(&mut t.accent),
                "canvas" | "background" | "bg" => Some(&mut t.canvas),
                "dot" | "dots" => Some(&mut t.dot),
                "ink" | "text" => Some(&mut t.ink),
                "muted" => Some(&mut t.muted),
                "card" => Some(&mut t.card),
                _ => None,
            };
            if let Some(slot) = slot {
                if let Some(c) = pal::parse_hex(v) {
                    *slot = c;
                }
                continue;
            }
            let num = v.trim().trim_end_matches("px/s").trim_end_matches("px").trim().parse::<f32>().ok().filter(|n| n.is_finite());
            match (key.as_str(), num) {
                ("dot_pitch" | "pitch", Some(n)) if n >= 4.0 => t.dot_pitch = n,
                ("drift" | "drift_px_s", Some(n)) => t.drift_px_s = n,
                _ => {}
            }
        }
        t
    }
}

/// Maps SCREEN-normalized coordinates (0..1 of the screen recording) to frame pixels, and the
/// pixel rect the screen is visible in (the panel). Accounts for fit and zoom.
#[derive(Clone, Copy, Debug)]
pub struct ScreenMap {
    pub x: f32,
    pub y: f32,
    pub sx: f32,
    pub sy: f32,
    pub clip: Rect,
}

impl ScreenMap {
    pub fn px(&self, nx: f64, ny: f64) -> (f32, f32) {
        (self.x + nx as f32 * self.sx, self.y + ny as f32 * self.sy)
    }
}

pub struct Gfx {
    pub theme: Theme,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    inner: Inner,
}

/// (frame width, pitch px, radius x100, canvas, dot).
type StripKey = (u32, u32, u32, [u8; 3], [u8; 3]);

/// Fonts and caches. Nothing here changes what a frame looks like; it only makes it faster.
pub(crate) struct Inner {
    asset_dir: PathBuf,
    text: text::Text,
    strips: Cache<StripKey, canvas::Strips>,
    dots: Cache<(u32, u8, u8), Sprite>,
    shadows: Cache<(u32, u32, u32, u32, i32), Shadow>,
    /// Static layers (panel surface, chips, badge, glows) by a content hash.
    layers: Cache<u64, Img>,
    /// Per-beat layouts by a content hash.
    layouts: Cache<u64, stage::Layout>,
    /// Pre-scaled images by (path, w, h).
    images: Cache<(PathBuf, u32, u32), Option<Img>>,
}

impl Gfx {
    /// `asset_dir`: where relative image paths in beats resolve (the spec folder).
    pub fn new(theme: Theme, width: u32, height: u32, asset_dir: &Path) -> anyhow::Result<Gfx> {
        let inner = Inner {
            asset_dir: asset_dir.to_path_buf(),
            text: text::Text::new()?,
            strips: Cache::new(4),
            dots: Cache::new(256),
            shadows: Cache::new(32),
            layers: Cache::new(64),
            layouts: Cache::new(48),
            images: Cache::new(24),
        };
        Ok(Gfx { theme, width, height, scale: height as f32 / 1080.0, inner })
    }

    /// Full-frame canvas: dark background + drifting dot grid at output time `t_ms`.
    pub fn canvas(&self, pm: &mut Pixmap, t_ms: f64) {
        let th = &self.theme;
        let pitch = (th.dot_pitch * self.scale).round().max(4.0) as u32;
        let radius = 1.6 * self.scale;
        let key = (pm.width(), pitch, (radius * 100.0) as u32, th.canvas, th.dot);
        let strips = self.inner.strips.get_or(key, || canvas::build_strips(pm.width() as usize, pitch as usize, radius, th.canvas, th.dot));
        canvas::draw_strips(pm, &strips, self.drift(t_ms));
    }

    /// Soft drop shadow under a rounded slot (panel or camera tile), drawn before the video blit.
    pub fn slot_shadow(&self, pm: &mut Pixmap, rect: Rect, radius: f32) {
        let r = IRect::round(rect);
        let sh = self.shadow(r.w() as u32, r.h() as u32, radius, 20.0 * self.scale, 0.0);
        let dy = (20.0 * self.scale).round() as i32;
        let skip = IRect::new(r.x0 + sh.inner.x0, r.y0 + sh.inner.y0, r.x0 + sh.inner.x1, r.y0 + sh.inner.y1);
        raster::blit_mask_skip(pm, &sh.sprite, r.x0, r.y0 + dy, [0, 0, 0], 0.39, IRect::of(pm), Some(skip));
    }

    /// Corner radius (px) used for the panel and camera tile.
    pub fn slot_radius(&self) -> f32 {
        22.0 * self.scale
    }

    /// A stage beat filling the panel `panel` (px). Draws its own panel background.
    /// `media`: for `video` beats, the decoded frame (any size; gfx fits it); None otherwise.
    pub fn stage(&self, pm: &mut Pixmap, panel: Rect, beat: &ResolvedBeat, t_ms: f64, media: Option<&Pixmap>) {
        stage::draw(self, pm, panel, beat, t_ms, media);
    }

    /// Size (px) gfx wants `video` beat frames decoded at, for a given panel.
    pub fn stage_media_size(&self, panel: Rect) -> (u32, u32) {
        ((panel.width() * 0.86) as u32 & !1, (panel.height() * 0.78) as u32 & !1)
    }

    /// An overlay beat (ring, highlight, arrow, callout, lower) on top of the frame.
    pub fn overlay(&self, pm: &mut Pixmap, map: &ScreenMap, beat: &ResolvedBeat, t_ms: f64) {
        overlay::draw(self, pm, map, beat, t_ms);
    }

    /// Full-frame chapter card (step `ch.n` of `total`).
    pub fn chapter_card(&self, pm: &mut Pixmap, ch: &ResolvedChapter, total: usize, t_ms: f64) {
        chrome::chapter_card(self, pm, ch, total, t_ms);
    }

    /// Persistent step badge in `rect` (px; above the camera tile).
    pub fn badge(&self, pm: &mut Pixmap, rect: Rect, ch: &ResolvedChapter, total: usize, t_ms: f64) {
        chrome::badge(self, pm, rect, ch, total, t_ms);
    }

    /// Burned-in captions (bottom of the panel or frame).
    pub fn captions(&self, pm: &mut Pixmap, area: Rect, words: &[OutWord], t_ms: f64) {
        chrome::captions(self, pm, area, words, t_ms);
    }

    // ---- shared helpers for the modules ----

    pub(crate) fn pal(&self) -> Pal {
        Pal::from_theme(&self.theme)
    }

    pub(crate) fn text(&self) -> &text::Text {
        &self.inner.text
    }

    pub(crate) fn inner(&self) -> &Inner {
        &self.inner
    }

    /// Canvas drift (frame px) at output time `t_ms`.
    pub(crate) fn drift(&self, t_ms: f64) -> f32 {
        // Wrap well inside f32 precision; the grid is periodic in the pitch anyway.
        let pitch = (self.theme.dot_pitch * self.scale).round().max(4.0) as f64;
        let off = self.theme.drift_px_s as f64 * self.scale as f64 * t_ms / 1000.0;
        off.rem_euclid(pitch * 1000.0) as f32
    }

    pub(crate) fn shadow(&self, w: u32, h: u32, radius: f32, blur: f32, spread: f32) -> Arc<Shadow> {
        let key = (w, h, (radius * 4.0) as u32, (blur * 4.0) as u32, (spread * 4.0) as i32);
        self.inner.shadows.get_or(key, || raster::make_shadow(w, h, radius, blur, spread))
    }

    /// The 16 subpixel variants of a dot stamp of `radius` px.
    pub(crate) fn dot_sprites(&self, radius: f32) -> Vec<Arc<Sprite>> {
        let rq = (radius * 64.0) as u32;
        (0..16u8).map(|i| self.inner.dots.get_or((rq, i % 4, i / 4), || canvas::dot_sprite(radius, i % 4, i / 4))).collect()
    }

    pub(crate) fn layer(&self, key: u64, build: impl FnOnce() -> Img) -> Arc<Img> {
        self.inner.layers.get_or(key, build)
    }

    /// An image file scaled to `w` x `h` (None if it cannot be read).
    pub(crate) fn image(&self, src: &str, w: u32, h: u32) -> Arc<Option<Img>> {
        let path = self.resolve_path(src);
        self.inner.images.get_or((path.clone(), w, h), || media::load_scaled(&path, w, h))
    }

    pub(crate) fn resolve_path(&self, src: &str) -> PathBuf {
        let p = Path::new(src);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.inner.asset_dir.join(p)
        }
    }
}

/// A stable 64-bit hash of anything hashable (fixed keys, so identical across runs).
pub(crate) fn hash_of<T: std::hash::Hash>(v: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gfx_is_shareable() {
        fn assert_sync<T: Send + Sync>() {}
        assert_sync::<Gfx>();
    }

    #[test]
    fn theme_overrides() {
        let mut m = BTreeMap::new();
        m.insert("accent".to_string(), "#3366FF".to_string());
        m.insert("canvas".to_string(), "000".to_string());
        m.insert("ink".to_string(), "not a colour".to_string());
        m.insert("dot_pitch".to_string(), "32".to_string());
        m.insert("drift".to_string(), "0".to_string());
        m.insert("unknown".to_string(), "#ffffff".to_string());
        let t = Theme::from_style(&m);
        assert_eq!(t.accent, [0x33, 0x66, 0xFF]);
        assert_eq!(t.canvas, [0, 0, 0]);
        assert_eq!(t.ink, [0xF4, 0xF1, 0xEC], "bad values keep the default");
        assert_eq!(t.dot_pitch, 32.0);
        assert_eq!(t.drift_px_s, 0.0);
        assert_eq!(t.card, [0x1E, 0x1D, 0x1B]);
        let d = Theme::from_style(&BTreeMap::new());
        assert_eq!((d.canvas, d.dot, d.dot_pitch, d.drift_px_s), ([0x12; 3], [0x2c; 3], 40.0, 2.5));
    }
}
