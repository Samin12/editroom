//! Text: embedded fonts (Inter, JetBrains Mono; SIL OFL), shaping with rustybuzz (kerning,
//! tabular figures), rasterizing with ab_glyph through a glyph cache, simple word-wrap layout
//! with auto-shrink, and cached run sprites.

use crate::cache::Cache;
use crate::pal::Rgb;
use crate::raster::{blit_mask, IRect, Sprite};
use ab_glyph::{point, Font as _, FontRef, GlyphId, PxScale};
use std::sync::Arc;
use tiny_skia::Pixmap;

static INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
static INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
static INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
static JB_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontId {
    Regular = 0,
    SemiBold = 1,
    Bold = 2,
    Mono = 3,
}

struct Face {
    ab: FontRef<'static>,
    hb: rustybuzz::Face<'static>,
    upem: f32,
    asc: f32,
    desc: f32,
    height: f32,
}

/// Shaped text in font units: glyph ids with pen positions.
pub struct Shaped {
    pub glyphs: Vec<(u16, f32, f32)>,
    pub adv: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub font: FontId,
    /// px per em.
    pub size: f32,
    /// Extra letter spacing (em).
    pub ls: f32,
    /// Line height (multiple of size).
    pub lh: f32,
    /// Tabular figures.
    pub tnum: bool,
}

impl Style {
    pub fn new(font: FontId, size: f32) -> Style {
        Style { font, size: q(size), ls: 0.0, lh: 1.2, tnum: false }
    }
    pub fn ls(mut self, ls: f32) -> Style {
        self.ls = ls;
        self
    }
    pub fn lh(mut self, lh: f32) -> Style {
        self.lh = lh;
        self
    }
    pub fn tnum(mut self) -> Style {
        self.tnum = true;
        self
    }
    pub fn sized(mut self, size: f32) -> Style {
        self.size = q(size);
        self
    }
}

/// Sizes are quantized to quarter pixels so layout and raster agree and caches stay small.
pub fn q(size: f32) -> f32 {
    (size.max(1.0) * 4.0).round() / 4.0
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RunKey {
    font: FontId,
    size_q: u32,
    ls_q: i32,
    tnum: bool,
    text: String,
}

/// One input token for layout: a word, whether it is accented, whether a line break precedes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Tok {
    pub text: String,
    pub tag: u8,
    pub brk: bool,
}

/// Split text into words. With `markup`, `*words*` are tagged 1 (accent) and the stars dropped.
/// `\n` forces a line break.
pub fn tokens(text: &str, markup: bool) -> Vec<Tok> {
    let mut out = vec![];
    let mut accent = false;
    for (li, line) in text.split('\n').enumerate() {
        let mut first = true;
        for w in line.split_whitespace() {
            let mut w = w;
            let mut tag_on = accent;
            let mut close = false;
            if markup {
                if let Some(rest) = w.strip_prefix('*') {
                    w = rest;
                    tag_on = true;
                    accent = true;
                }
                if let Some(rest) = w.strip_suffix('*') {
                    w = rest;
                    close = true;
                }
            }
            if !w.is_empty() {
                out.push(Tok { text: w.to_string(), tag: tag_on as u8, brk: li > 0 && first });
                first = false;
            }
            if close {
                accent = false;
            }
        }
    }
    out
}

/// A laid-out block: words placed on lines (x relative to the block's left, px).
#[derive(Clone, Debug)]
pub struct Block {
    pub words: Vec<LWord>,
    pub line_w: Vec<f32>,
    pub style: Style,
    pub lines: usize,
    /// Widest line.
    pub w: f32,
    asc: f32,
    desc: f32,
}

#[derive(Clone, Debug)]
pub struct LWord {
    pub text: String,
    pub x: f32,
    pub line: usize,
    pub tag: u8,
}

impl Block {
    pub fn line_h(&self) -> f32 {
        self.style.lh * self.style.size
    }
    pub fn h(&self) -> f32 {
        self.lines.max(1) as f32 * self.line_h()
    }
    /// Baseline of `line` below the block top (CSS half-leading).
    pub fn baseline(&self, line: usize) -> f32 {
        line as f32 * self.line_h() + (self.line_h() - (self.asc + self.desc)) * 0.5 + self.asc
    }
    /// Cap height (approx) above the baseline, px.
    pub fn cap(&self) -> f32 {
        0.727 * self.style.size
    }
    /// Centre every line inside `width`.
    pub fn center(&mut self, width: f32) {
        for w in &mut self.words {
            w.x += (width - self.line_w[w.line]) * 0.5;
        }
    }
}

type ShapeKey = (FontId, bool, String);

pub struct Text {
    faces: Vec<Face>,
    shaped: Cache<ShapeKey, Shaped>,
    glyphs: Cache<(FontId, u32, u16, u8), Sprite>,
    runs: Cache<RunKey, Sprite>,
}

impl Text {
    pub fn new() -> anyhow::Result<Text> {
        let mut faces = vec![];
        for data in [INTER_REGULAR, INTER_SEMIBOLD, INTER_BOLD, JB_MONO] {
            let ab = FontRef::try_from_slice(data).map_err(|e| anyhow::anyhow!("font: {e}"))?;
            let hb = rustybuzz::Face::from_slice(data, 0).ok_or_else(|| anyhow::anyhow!("font: shaping face"))?;
            let upem = ab.units_per_em().unwrap_or(1000.0);
            faces.push(Face { asc: ab.ascent_unscaled(), desc: -ab.descent_unscaled(), height: ab.height_unscaled(), upem, ab, hb });
        }
        Ok(Text { faces, shaped: Cache::new(8192), glyphs: Cache::new(16384), runs: Cache::new(4096) })
    }

    fn face(&self, f: FontId) -> &Face {
        &self.faces[f as usize]
    }

    pub fn ascent(&self, st: Style) -> f32 {
        let f = self.face(st.font);
        f.asc / f.upem * st.size
    }

    pub fn descent(&self, st: Style) -> f32 {
        let f = self.face(st.font);
        f.desc / f.upem * st.size
    }

    pub fn shape(&self, font: FontId, text: &str, tnum: bool) -> Arc<Shaped> {
        self.shaped.get_or((font, tnum, text.to_string()), || {
            let face = &self.face(font).hb;
            let mut buf = rustybuzz::UnicodeBuffer::new();
            buf.push_str(text);
            let feats: Vec<rustybuzz::Feature> = if tnum { vec![rustybuzz::Feature::new(rustybuzz::ttf_parser::Tag::from_bytes(b"tnum"), 1, ..)] } else { vec![] };
            let gb = rustybuzz::shape(face, &feats, buf);
            let mut pen = 0.0f32;
            let mut glyphs = Vec::with_capacity(gb.len());
            for (info, pos) in gb.glyph_infos().iter().zip(gb.glyph_positions()) {
                glyphs.push((info.glyph_id as u16, pen + pos.x_offset as f32, pos.y_offset as f32));
                pen += pos.x_advance as f32;
            }
            Shaped { glyphs, adv: pen }
        })
    }

    /// Advance width of `text` (px), letter spacing between glyphs only.
    pub fn width(&self, st: Style, text: &str) -> f32 {
        let s = self.shape(st.font, text, st.tnum);
        let k = st.size / self.face(st.font).upem;
        s.adv * k + st.ls * st.size * (s.glyphs.len().saturating_sub(1)) as f32
    }

    fn glyph(&self, font: FontId, size_q: u32, id: u16, sub: u8) -> Arc<Sprite> {
        self.glyphs.get_or((font, size_q, id, sub), || {
            let f = self.face(font);
            let size = size_q as f32 / 4.0;
            let scale = PxScale::from(size * f.height / f.upem);
            let g = GlyphId(id).with_scale_and_position(scale, point(sub as f32 / 4.0, 0.0));
            match f.ab.outline_glyph(g) {
                None => Sprite::default(),
                Some(og) => {
                    let b = og.px_bounds();
                    let (w, h) = (b.width().max(0.0) as u32, b.height().max(0.0) as u32);
                    let mut data = vec![0u8; (w * h) as usize];
                    og.draw(|x, y, c| {
                        if x < w && y < h {
                            data[(y * w + x) as usize] = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    });
                    Sprite { w, h, ox: b.min.x as i32, oy: b.min.y as i32, data }
                }
            }
        })
    }

    /// The coverage sprite of a single-line run, anchored at its pen origin on the baseline.
    pub fn run(&self, st: Style, text: &str) -> Arc<Sprite> {
        let key = RunKey { font: st.font, size_q: (st.size * 4.0).round() as u32, ls_q: (st.ls * 1000.0).round() as i32, tnum: st.tnum, text: text.to_string() };
        self.runs.get_or(key, || self.render_run(st, text))
    }

    fn render_run(&self, st: Style, text: &str) -> Sprite {
        let s = self.shape(st.font, text, st.tnum);
        let f = self.face(st.font);
        let k = st.size / f.upem;
        let size_q = (st.size * 4.0).round() as u32;
        let ls = st.ls * st.size;
        let mut placed = Vec::with_capacity(s.glyphs.len());
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (i, &(id, gx, gy)) in s.glyphs.iter().enumerate() {
            let pen = gx * k + ls * i as f32;
            let mut ix = pen.floor() as i32;
            let mut sub = ((pen - ix as f32) * 4.0).round() as u8;
            if sub >= 4 {
                ix += 1;
                sub = 0;
            }
            let iy = (-gy * k).round() as i32;
            let g = self.glyph(st.font, size_q, id, sub);
            if g.w == 0 || g.h == 0 {
                continue;
            }
            let (gx0, gy0) = (ix + g.ox, iy + g.oy);
            x0 = x0.min(gx0);
            y0 = y0.min(gy0);
            x1 = x1.max(gx0 + g.w as i32);
            y1 = y1.max(gy0 + g.h as i32);
            placed.push((gx0, gy0, g));
        }
        if placed.is_empty() {
            return Sprite::default();
        }
        let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
        let mut data = vec![0u8; (w * h) as usize];
        for (gx0, gy0, g) in placed {
            for gy in 0..g.h as usize {
                let drow = (gy0 - y0) as usize + gy;
                let dst = &mut data[drow * w as usize + (gx0 - x0) as usize..][..g.w as usize];
                let src = &g.data[gy * g.w as usize..(gy + 1) * g.w as usize];
                for (d, s) in dst.iter_mut().zip(src) {
                    let (a, b) = (*d as u32, *s as u32);
                    *d = (a + b - (a * b + 127) / 255) as u8;
                }
            }
        }
        Sprite { w, h, ox: x0, oy: y0, data }
    }

    /// Greedy word wrap to `max_w` (px).
    pub fn layout(&self, toks: &[Tok], st: Style, max_w: f32) -> Block {
        let space = self.width(st, " ") + st.ls * st.size;
        let mut words = Vec::with_capacity(toks.len());
        let mut line_w = vec![0.0f32];
        let mut line = 0usize;
        let mut x = 0.0f32;
        for (i, t) in toks.iter().enumerate() {
            let w = self.width(st, &t.text);
            let overflow = i > 0 && !t.brk && x + space + w > max_w;
            if overflow || (t.brk && i > 0) {
                line += 1;
                line_w.push(0.0);
                x = 0.0;
            } else if i > 0 {
                x += space;
            }
            words.push(LWord { text: t.text.clone(), x, line, tag: t.tag });
            x += w;
            line_w[line] = x;
        }
        let w = line_w.iter().cloned().fold(0.0, f32::max);
        Block { words, lines: line + 1, line_w, style: st, w, asc: self.ascent(st), desc: self.descent(st) }
    }

    /// Largest size <= `st.size` whose layout fits `max_w`, `max_lines` and `max_h`. Below
    /// `min_size` it keeps shrinking rather than overflow.
    pub fn fit(&self, toks: &[Tok], st: Style, max_w: f32, max_lines: usize, max_h: f32, min_size: f32) -> Block {
        let mut size = st.size;
        loop {
            let s = st.sized(size);
            let b = self.layout(toks, s, max_w);
            let ok_w = b.w <= max_w + 0.5;
            let ok_h = b.h() <= max_h + 0.5;
            let ok_l = b.lines <= max_lines || size <= min_size;
            if (ok_w && ok_h && ok_l) || size <= 4.0 {
                return b;
            }
            size = q((size * 0.94).min(size - 0.25));
        }
    }

    /// Draw a block with its top-left at (x, y). `colors[tag]` per word.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, pm: &mut Pixmap, b: &Block, x: f32, y: f32, colors: &[Rgb], opacity: f32, clip: IRect) {
        for w in &b.words {
            self.draw_word(pm, b, w, x, y, colors[(w.tag as usize).min(colors.len() - 1)], opacity, clip);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_word(&self, pm: &mut Pixmap, b: &Block, w: &LWord, x: f32, y: f32, rgb: Rgb, opacity: f32, clip: IRect) {
        if opacity <= 0.0 {
            return;
        }
        let s = self.run(b.style, &w.text);
        blit_mask(pm, &s, (x + w.x).round() as i32, (y + b.baseline(w.line)).round() as i32, rgb, opacity, clip);
    }

    /// Draw a single line with its baseline-left at (x, y).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_line(&self, pm: &mut Pixmap, st: Style, text: &str, x: f32, y: f32, rgb: Rgb, opacity: f32, clip: IRect) {
        if opacity <= 0.0 || text.is_empty() {
            return;
        }
        let s = self.run(st, text);
        blit_mask(pm, &s, x.round() as i32, y.round() as i32, rgb, opacity, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_tokens() {
        let t = tokens("Connect *Claude to* Poppy\nnow", true);
        let tags: Vec<u8> = t.iter().map(|t| t.tag).collect();
        assert_eq!(t.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["Connect", "Claude", "to", "Poppy", "now"]);
        assert_eq!(tags, [0, 1, 1, 0, 0]);
        assert!(t[4].brk && !t[3].brk);
        let raw = tokens("a *b*", false);
        assert_eq!(raw[1].text, "*b*");
    }

    #[test]
    fn kerning_and_tnum() {
        let tx = Text::new().unwrap();
        let st = Style::new(FontId::Bold, 100.0);
        let pair = tx.width(st, "AV");
        let apart = tx.width(st, "A") + tx.width(st, "V");
        assert!(pair < apart - 1.0, "GPOS kerning applies: {pair} vs {apart}");
        let t = st.tnum();
        assert!((tx.width(t, "111") - tx.width(t, "888")).abs() < 0.01, "tabular figures");
    }

    #[test]
    fn wrap_and_fit() {
        let tx = Text::new().unwrap();
        let toks = tokens("the quick brown fox jumps over the lazy dog again and again", false);
        let st = Style::new(FontId::Regular, 40.0);
        let b = tx.layout(&toks, st, 400.0);
        assert!(b.lines > 1);
        assert!(b.w <= 400.0);
        let f = tx.fit(&toks, Style::new(FontId::Bold, 120.0), 600.0, 2, 1000.0, 30.0);
        assert!(f.lines <= 2 && f.w <= 600.5, "{} lines, w {}", f.lines, f.w);
        let s = tx.run(st, "Hello");
        assert!(s.w > 50 && s.h > 20 && s.oy < 0);
    }
}
