//! Stage beats: templates that fill the panel (title, bullets, number, image, video, code,
//! quote). Each beat is laid out once (cached by content) in panel space, then every frame only
//! composites cached sprites with the motion grammar applied.

use crate::anim::{back_out, ease_in_out, ease_out, geo_zoom, lerp, prog, Clock};
use crate::canvas;
use crate::numfmt::NumFmt;
use crate::pal::{mix, Pal, Rgb};
use crate::raster::{self, blit_img, blit_mask_skip, draw_img, fill_circle, fill_rrect, rrect, stroke_partial, stroke_rrect, IRect, Round, Spot};
use crate::text::{tokens, Block, FontId, Style, Tok};
use crate::{hash_of, Gfx};
use cutroom_core::resolve::ResolvedBeat;
use cutroom_core::spec::BeatKind;
use tiny_skia::{FillRule, PathBuilder, Pixmap, Rect, Transform};

/// Panel geometry: origin and size in px, `u` px per stage unit (panel width = 1920 units).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geo {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u: f32,
}

impl Geo {
    pub fn new(panel: Rect) -> Geo {
        Geo { x: panel.left(), y: panel.top(), w: panel.width(), h: panel.height(), u: panel.width() / 1920.0 }
    }
}

/// A placed text block (top-left relative to the panel, px).
pub(crate) struct Placed {
    pub b: Block,
    pub x: f32,
    pub y: f32,
}

pub(crate) enum Layout {
    Title(TitleL),
    Bullets(BulletsL),
    Number(NumberL),
    Media(MediaL),
    Code(CodeL),
    Quote(QuoteL),
    Empty,
}

pub(crate) struct TitleL {
    kicker: Option<Placed>,
    rule: [f32; 4],
    head: Placed,
    word_ms: Vec<f64>,
    sub: Option<Placed>,
    sub_ms: f64,
}

pub(crate) struct BulletsL {
    title: Option<Placed>,
    items: Vec<Placed>,
    markers: Vec<(f32, f32)>,
    marker_r: f32,
    rules: Vec<[f32; 3]>,
    item_ms: Vec<f64>,
}

pub(crate) struct NumberL {
    fmt: Option<NumFmt>,
    value: String,
    st: Style,
    cx: f32,
    base: f32,
    bar: [f32; 4],
    label: Option<Placed>,
}

pub(crate) struct MediaL {
    /// Card rect relative to the panel (px, pixel aligned).
    card: [f32; 4],
    radius: f32,
    /// Image source and its pre-scaled size; None for video.
    src: Option<(String, u32, u32)>,
    found: bool,
    highlight: Option<[f64; 4]>,
    caption: Option<Placed>,
    source: Option<(Placed, [f32; 4])>,
    missing: Option<Placed>,
}

pub(crate) struct CodeL {
    card: [f32; 4],
    radius: f32,
    bar_h: f32,
    title: Option<Placed>,
    st: Style,
    adv: f32,
    line_h: f32,
    text_x: f32,
    text_y: f32,
    lines: Vec<CodeLine>,
    total: usize,
}

struct CodeLine {
    chars: usize,
    segs: Vec<(usize, String, u8)>,
}

pub(crate) struct QuoteL {
    /// Ball centre of the first quote mark and the ball radius.
    mark: (f32, f32, f32),
    text: Placed,
    by: Option<Placed>,
    rule: [f32; 4],
}

pub(crate) fn draw(g: &Gfx, pm: &mut Pixmap, panel: Rect, beat: &ResolvedBeat, t_ms: f64, media: Option<&Pixmap>) {
    let geo = Geo::new(panel);
    let clip = IRect::round(panel).intersect(IRect::of(pm));
    if clip.is_empty() {
        return;
    }
    let pal = g.pal();
    draw_surface(g, pm, &geo, clip, &pal, t_ms);
    let key = layout_key(beat, &geo);
    let layout = g.inner().layouts.get_or(key, || build(g, &geo, beat));
    let ck = Clock::new(t_ms, beat.start_ms, beat.end_ms);
    if ck.lt < 0.0 || ck.lt > ck.dur {
        return;
    }
    let f = Frame { g, geo, clip, pal, ck };
    match &*layout {
        Layout::Title(l) => f.title(pm, l),
        Layout::Bullets(l) => f.bullets(pm, l),
        Layout::Number(l) => f.number(pm, l),
        Layout::Media(l) => f.media(pm, l, media),
        Layout::Code(l) => f.code(pm, l),
        Layout::Quote(l) => f.quote(pm, l),
        Layout::Empty => {}
    }
}

fn layout_key(beat: &ResolvedBeat, geo: &Geo) -> u64 {
    let words: Vec<(&str, u64)> = beat.words.iter().map(|w| (w.text.as_str(), w.start_ms.to_bits())).collect();
    let items: Vec<u64> = beat.items_ms.iter().map(|v| v.to_bits()).collect();
    hash_of(&(format!("{:?}", beat.kind), beat.start_ms.to_bits(), beat.end_ms.to_bits(), items, words, [geo.w.to_bits(), geo.h.to_bits()]))
}

/// The raised panel surface plus its own slower dot grid (parallax against the canvas).
fn draw_surface(g: &Gfx, pm: &mut Pixmap, geo: &Geo, clip: IRect, pal: &Pal, t_ms: f64) {
    let r = g.slot_radius();
    let (x, y) = (geo.x.round(), geo.y.round());
    let (w, h) = ((geo.x + geo.w).round() - x, (geo.y + geo.h).round() - y);
    let key = hash_of(&("surface", w as u32, h as u32, (r * 4.0) as u32, pal));
    let layer = g.layer(key, || canvas::surface_layer(w as u32, h as u32, r, pal, 0.05));
    blit_img(pm, &layer, x as i32, y as i32, 1.0, clip);
    let sprites = g.dot_sprites(1.35 * g.scale);
    let round = Round::new(x + 2.0, y + 2.0, w - 4.0, h - 4.0, r);
    canvas::stamp_dots(pm, &sprites, clip, 32.0 * g.scale, g.drift(t_ms) * 0.8, mix(pal.surface, pal.dot, 0.8), 1.0, Some(round));
}

// ------------------------------------------------------------------------------------ layout

fn build(g: &Gfx, geo: &Geo, beat: &ResolvedBeat) -> Layout {
    match &beat.kind {
        BeatKind::Title { text, kicker, sub } => Layout::Title(build_title(g, geo, beat, text, kicker.as_deref(), sub.as_deref())),
        BeatKind::Bullets { title, items } => {
            let texts: Vec<&str> = items.iter().map(|i| i.text()).collect();
            Layout::Bullets(build_bullets(g, geo, beat, title.as_deref(), &texts))
        }
        BeatKind::Number { value, label } => Layout::Number(build_number(g, geo, value, label.as_deref())),
        BeatKind::Image { src, caption, highlight, source } => Layout::Media(build_media(g, geo, Some(src), caption.as_deref(), *highlight, source.as_deref())),
        BeatKind::Video { caption, .. } => Layout::Media(build_media(g, geo, None, caption.as_deref(), None, None)),
        BeatKind::Code { text, title } => Layout::Code(build_code(g, geo, text, title.as_deref())),
        BeatKind::Quote { text, by } => Layout::Quote(build_quote(g, geo, text, by.as_deref())),
        _ => Layout::Empty,
    }
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// When each headline word appears (local ms): on the spoken word when the narration says it
/// inside the beat, otherwise a short stagger after the previous word.
fn word_times(beat: &ResolvedBeat, toks: &[Tok]) -> Vec<f64> {
    let dur = beat.end_ms - beat.start_ms;
    let spoken: Vec<(String, f64)> = beat.words.iter().map(|w| (norm(&w.text), w.start_ms - beat.start_ms)).collect();
    let latest = (dur - 900.0).max(0.0);
    let mut j = 0;
    let mut prev = 0.0f64;
    let mut out = Vec::with_capacity(toks.len());
    for (i, t) in toks.iter().enumerate() {
        let n = norm(&t.text);
        let mut hit = None;
        if !n.is_empty() {
            for (k, (w, at)) in spoken.iter().enumerate().skip(j).take(6) {
                if *w == n {
                    hit = Some(*at - 60.0);
                    j = k + 1;
                    break;
                }
            }
        }
        let floor = if i == 0 { 0.0 } else { prev + 45.0 };
        let v = match hit {
            Some(at) => at.max(floor),
            None => {
                if i == 0 {
                    0.0
                } else {
                    prev + 90.0
                }
            }
        };
        let v = v.clamp(0.0, latest.max(floor.min(latest)));
        prev = v;
        out.push(v);
    }
    out
}

fn build_title(g: &Gfx, geo: &Geo, beat: &ResolvedBeat, text: &str, kicker: Option<&str>, sub: Option<&str>) -> TitleL {
    let tx = g.text();
    let u = geo.u;
    let mx = 128.0 * u;
    let maxw = geo.w - 2.0 * mx;
    let toks = tokens(text, true);
    let pref = match toks.len() {
        0..=3 => 136.0,
        4..=6 => 120.0,
        _ => 104.0,
    };
    let kick = kicker.filter(|k| !k.trim().is_empty()).map(|k| tx.fit(&tokens(&k.to_uppercase(), false), Style::new(FontId::Bold, 30.0 * u).ls(0.16).lh(1.2), maxw - 70.0 * u, 1, 60.0 * u, 28.0 * u));
    let sub_b = sub.filter(|s| !s.trim().is_empty()).map(|s| tx.fit(&tokens(s, true), Style::new(FontId::Regular, 44.0 * u).lh(1.3), maxw.min(1500.0 * u), 3, geo.h * 0.22, 36.0 * u));
    let reserve = kick.as_ref().map(|b| b.h() + 34.0 * u).unwrap_or(0.0) + sub_b.as_ref().map(|b| b.h() + 44.0 * u).unwrap_or(0.0);
    let head = tx.fit(&toks, Style::new(FontId::Bold, pref * u).ls(-0.03).lh(1.04), maxw, 3, (geo.h - 2.0 * 90.0 * u - reserve).max(geo.h * 0.3), 56.0 * u);
    let total = reserve + head.h();
    let mut y = ((geo.h - total) * 0.47).max(60.0 * u);
    let mut rule = [0.0; 4];
    let kicker = kick.map(|b| {
        let ry = y + b.baseline(0) - b.cap() * 0.5;
        rule = [mx, ry - 2.0 * u, 52.0 * u, 4.0 * u];
        let p = Placed { x: mx + 70.0 * u, y, b };
        y += p.b.h() + 34.0 * u;
        p
    });
    let word_ms = word_times(beat, &toks);
    let head = Placed { b: head, x: mx, y };
    y += head.b.h() + 44.0 * u;
    let sub_ms = word_ms.last().copied().unwrap_or(0.0) + 160.0;
    let sub = sub_b.map(|b| Placed { b, x: mx + 2.0 * u, y });
    TitleL { kicker, rule, head, word_ms, sub, sub_ms }
}

fn build_bullets(g: &Gfx, geo: &Geo, beat: &ResolvedBeat, title: Option<&str>, items: &[&str]) -> BulletsL {
    let tx = g.text();
    let u = geo.u;
    let mx = 128.0 * u;
    let maxw = geo.w - 2.0 * mx;
    let indent = 64.0 * u;
    let avail = geo.h - 2.0 * 100.0 * u;
    let mut size = 58.0f32;
    let (title_b, blocks) = loop {
        let tb = title.filter(|t| !t.trim().is_empty()).map(|t| tx.fit(&tokens(t, true), Style::new(FontId::Bold, size * 1.5 * u).ls(-0.02).lh(1.08), maxw, 2, geo.h * 0.3, 40.0 * u));
        let bs: Vec<Block> = items.iter().map(|it| tx.layout(&tokens(it, true), Style::new(FontId::SemiBold, size * u).ls(-0.01).lh(1.25), maxw - indent)).collect();
        let gap = size * 0.95 * u;
        let total = tb.as_ref().map(|b| b.h() + 64.0 * u).unwrap_or(0.0) + bs.iter().map(|b| b.h()).sum::<f32>() + gap * bs.len().saturating_sub(1) as f32;
        if total <= avail || size <= 20.0 {
            break (tb, bs);
        }
        size *= 0.94;
    };
    let gap = size * 0.95 * u;
    let total = title_b.as_ref().map(|b| b.h() + 64.0 * u).unwrap_or(0.0) + blocks.iter().map(|b| b.h()).sum::<f32>() + gap * blocks.len().saturating_sub(1) as f32;
    let mut y = ((geo.h - total) * 0.47).max(40.0 * u);
    let title = title_b.map(|b| {
        let p = Placed { b, x: mx, y };
        y += p.b.h() + 64.0 * u;
        p
    });
    let mut out = vec![];
    let mut markers = vec![];
    let mut rules = vec![];
    for (k, b) in blocks.into_iter().enumerate() {
        if k > 0 {
            rules.push([mx + indent, y - gap * 0.5, maxw - indent]);
        }
        let cy = y + b.baseline(0) - b.cap() * 0.5;
        markers.push((mx + 14.0 * u, cy));
        let h = b.h();
        out.push(Placed { b, x: mx + indent, y });
        y += h + gap;
    }
    let n = out.len();
    let item_ms: Vec<f64> = (0..n)
        .map(|k| match beat.items_ms.get(k) {
            Some(t) => (t - beat.start_ms).max(0.0),
            None => 300.0 + 600.0 * k as f64,
        })
        .collect();
    BulletsL { title, items: out, markers, marker_r: (size * 0.17 * u).max(3.0), rules, item_ms }
}

fn build_number(g: &Gfx, geo: &Geo, value: &str, label: Option<&str>) -> NumberL {
    let tx = g.text();
    let u = geo.u;
    let fmt = NumFmt::parse(value);
    let maxw = geo.w - 2.0 * 160.0 * u;
    let mut st = Style::new(FontId::Bold, 300.0 * u).ls(-0.04).tnum();
    while tx.width(st, value.trim()) > maxw && st.size > 24.0 * u {
        st = st.sized(st.size * 0.95);
    }
    let label_b = label.filter(|l| !l.trim().is_empty()).map(|l| {
        let mut b = tx.fit(&tokens(l, true), Style::new(FontId::SemiBold, 48.0 * u).lh(1.25), maxw.min(1400.0 * u), 2, geo.h * 0.2, 36.0 * u);
        let w = b.w;
        b.center(w);
        b
    });
    let cap = 0.727 * st.size;
    // Below the descender of a comma, then breathing room.
    let gap1 = 0.24 * st.size + 44.0 * u;
    let bar_h = 8.0 * u;
    let gap2 = 48.0 * u;
    let total = cap + gap1 + bar_h + label_b.as_ref().map(|b| gap2 + b.h()).unwrap_or(0.0);
    let top = (geo.h - total) * 0.46;
    let base = top + cap;
    let bar = [geo.w * 0.5, base + gap1, 120.0 * u, bar_h];
    let label = label_b.map(|b| Placed { x: (geo.w - b.w) * 0.5, y: base + gap1 + bar_h + gap2, b });
    NumberL { fmt, value: value.trim().to_string(), st, cx: geo.w * 0.5, base, bar, label }
}

fn build_media(g: &Gfx, geo: &Geo, src: Option<&String>, caption: Option<&str>, highlight: Option<[f64; 4]>, source: Option<&str>) -> MediaL {
    let tx = g.text();
    let u = geo.u;
    let mx = 128.0 * u;
    let maxw = geo.w - 2.0 * mx;
    let cap_b = caption.filter(|c| !c.trim().is_empty()).map(|c| {
        let mut b = tx.fit(&tokens(c, true), Style::new(FontId::SemiBold, 40.0 * u).lh(1.25), maxw, 2, 130.0 * u, 36.0 * u);
        let w = b.w;
        b.center(w);
        b
    });
    let cap_space = cap_b.as_ref().map(|b| b.h() + 40.0 * u).unwrap_or(0.0);
    let (cw, ch, found, dims) = match src {
        Some(s) => {
            let dims = crate::media::dims(&g.resolve_path(s));
            let (iw, ih) = dims.unwrap_or((16, 9));
            let bw = maxw;
            let bh = geo.h - 2.0 * 96.0 * u - cap_space;
            let k = (bw / iw as f32).min(bh / ih as f32);
            ((iw as f32 * k).round(), (ih as f32 * k).round(), dims.is_some(), dims)
        }
        None => {
            let (w, h) = g.stage_media_size(Rect::from_xywh(geo.x, geo.y, geo.w, geo.h).unwrap_or(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()));
            (w as f32, h as f32, true, None)
        }
    };
    let total = ch + cap_space;
    let cx = ((geo.w - cw) * 0.5).round();
    let cy = ((geo.h - total) * 0.5).round();
    let card = [cx, cy, cw, ch];
    let caption = cap_b.map(|b| Placed { x: (geo.w - b.w) * 0.5, y: cy + ch + 40.0 * u, b });
    let src = match (src, dims) {
        (Some(s), Some(_)) => Some((s.clone(), (cw * 1.06).ceil() as u32, (ch * 1.06).ceil() as u32)),
        (Some(s), None) => Some((s.clone(), 0, 0)),
        _ => None,
    };
    let source = source.filter(|s| !s.trim().is_empty()).map(|s| {
        let st = Style::new(FontId::SemiBold, 28.0 * u).lh(1.2);
        let b = tx.fit(&tokens(s, false), st, cw * 0.7, 1, 60.0 * u, 20.0 * u);
        let (ph, padx, dot) = (b.h() + 22.0 * u, 22.0 * u, 18.0 * u);
        let pw = b.w + 2.0 * padx + dot;
        let pill = [cx + 24.0 * u, cy + ch - 24.0 * u - ph, pw, ph];
        (Placed { x: pill[0] + padx + dot, y: pill[1] + (ph - b.h()) * 0.5, b }, pill)
    });
    let missing = if found {
        None
    } else {
        let name = src.as_ref().map(|s| s.0.clone()).unwrap_or_default();
        let mut b = tx.fit(&tokens(&format!("Image not found: {name}"), false), Style::new(FontId::SemiBold, 36.0 * u), cw * 0.8, 2, ch * 0.5, 20.0 * u);
        let w = b.w;
        b.center(w);
        Some(Placed { x: cx + (cw - b.w) * 0.5, y: cy + (ch - b.h()) * 0.5, b })
    };
    MediaL { card, radius: 28.0 * u, src, found, highlight, caption, source, missing }
}

fn wrap_code(text: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(8);
    let mut out = vec![];
    for raw in text.replace('\t', "    ").lines() {
        let line = raw.trim_end();
        let chars: Vec<char> = line.chars().collect();
        if chars.len() <= cols {
            out.push(line.to_string());
            continue;
        }
        // Continuation lines hang under the text after the indent or a prompt marker.
        let lead = chars.iter().take_while(|c| **c == ' ').count();
        let rest: String = chars[lead..].iter().collect();
        let marker = ["$ ", "> ", "❯ ", "- ", "* "].iter().find(|m| rest.starts_with(**m)).map(|m| m.chars().count()).unwrap_or(0);
        let indent = (lead + marker).min(cols / 2);
        let mut start = 0;
        let mut first = true;
        while start < chars.len() {
            let room = if first { cols } else { cols - indent };
            let end = (start + room).min(chars.len());
            let mut cut = end;
            if end < chars.len() {
                if let Some(sp) = (start + 1..end).rev().find(|&i| chars[i] == ' ') {
                    cut = sp + 1;
                }
            }
            let piece: String = chars[start..cut].iter().collect();
            out.push(if first { piece.trim_end().to_string() } else { format!("{}{}", " ".repeat(indent), piece.trim_end()) });
            start = cut;
            first = false;
        }
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out
}

fn code_segs(line: &str) -> Vec<(usize, String, u8)> {
    let t = line.trim_start();
    let lead = line.chars().count() - t.chars().count();
    if t.starts_with('#') || t.starts_with("//") || t.starts_with("--") {
        return vec![(0, line.to_string(), 2)];
    }
    for p in ["$ ", "> ", "❯ "] {
        if let Some(rest) = t.strip_prefix(p) {
            let pre: String = line.chars().take(lead + p.chars().count()).collect();
            let n = pre.chars().count();
            return vec![(0, pre, 1), (n, rest.to_string(), 0)];
        }
    }
    vec![(0, line.to_string(), 0)]
}

fn build_code(g: &Gfx, geo: &Geo, text: &str, title: Option<&str>) -> CodeL {
    let tx = g.text();
    let u = geo.u;
    let maxw = geo.w - 2.0 * 128.0 * u;
    let pad = 48.0 * u;
    let bar_h = 72.0 * u;
    let max_body = geo.h - 2.0 * 90.0 * u - bar_h - 2.0 * pad;
    // Prefer one size that fits the longest line without wrapping (down to 32 units); wrap
    // long prompts at 38 units otherwise. Then shrink until the height fits.
    let em_adv = tx.width(Style::new(FontId::Mono, 100.0), "0") / 100.0;
    let longest = text.replace('\t', "    ").lines().map(|l| l.trim_end().chars().count()).max().unwrap_or(1).max(1);
    let no_wrap = (maxw - 2.0 * pad) / (longest as f32 * em_adv);
    let mut size = if no_wrap >= 32.0 * u { (no_wrap * 0.995 - 0.25).min(42.0 * u) } else { 38.0 * u };
    let (st, lines, adv) = loop {
        let st = Style::new(FontId::Mono, size).lh(1.55);
        let adv = tx.width(st, "0");
        let cols = ((maxw - 2.0 * pad) / adv).floor() as usize;
        let lines = wrap_code(text, cols);
        if (lines.len().max(1) as f32 * st.lh * st.size <= max_body) || size <= 12.0 * u {
            break (st, lines, adv);
        }
        size *= 0.94;
    };
    let max_cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let body_w = max_cols as f32 * adv;
    let cw = (body_w + 2.0 * pad).max(960.0 * u).min(maxw).round();
    let body_h = lines.len().max(1) as f32 * st.lh * st.size;
    let ch = (bar_h + body_h + 2.0 * pad).round();
    let cx = ((geo.w - cw) * 0.5).round();
    let cy = ((geo.h - ch) * 0.46).round();
    let title = title.filter(|t| !t.trim().is_empty()).map(|t| {
        let b = tx.fit(&tokens(t, false), Style::new(FontId::SemiBold, 28.0 * u), cw - 260.0 * u, 1, bar_h, 18.0 * u);
        Placed { x: cx + (cw - b.w) * 0.5, y: cy + (bar_h - b.h()) * 0.5, b }
    });
    let lines: Vec<CodeLine> = lines.iter().map(|l| CodeLine { chars: l.chars().count(), segs: code_segs(l) }).collect();
    let total = lines.iter().map(|l| l.chars + 1).sum::<usize>();
    let line_h = st.lh * st.size;
    let text_y = cy + bar_h + pad;
    CodeL { card: [cx, cy, cw, ch], radius: 24.0 * u, bar_h, title, st, adv, line_h, text_x: cx + pad, text_y, lines, total }
}

fn build_quote(g: &Gfx, geo: &Geo, text: &str, by: Option<&str>) -> QuoteL {
    let tx = g.text();
    let u = geo.u;
    let mx = 160.0 * u;
    let maxw = (geo.w - 2.0 * mx).min(1600.0 * u);
    let by_b = by.filter(|b| !b.trim().is_empty()).map(|b| tx.fit(&tokens(b, false), Style::new(FontId::SemiBold, 36.0 * u).lh(1.2), maxw - 70.0 * u, 1, 80.0 * u, 28.0 * u));
    let pref = if text.chars().count() <= 70 { 84.0 } else { 68.0 };
    let body = tx.fit(&tokens(text, true), Style::new(FontId::SemiBold, pref * u).ls(-0.02).lh(1.18), maxw, 6, geo.h * 0.55, 38.0 * u);
    let mark_h = 120.0 * u;
    let total = mark_h + body.h() + by_b.as_ref().map(|b| 56.0 * u + b.h()).unwrap_or(0.0);
    let top = ((geo.h - total) * 0.45).max(40.0 * u);
    let r = 21.0 * u;
    let mark = (mx + r, top + mark_h - r - 34.0 * u, r);
    let ty = top + mark_h;
    let text = Placed { x: mx, y: ty, b: body };
    let mut rule = [0.0; 4];
    let by = by_b.map(|b| {
        let y = ty + text.b.h() + 56.0 * u;
        rule = [mx, y + b.baseline(0) - b.cap() * 0.5 - 2.0 * u, 44.0 * u, 4.0 * u];
        Placed { x: mx + 64.0 * u, y, b }
    });
    QuoteL { mark, text, by, rule }
}

// ------------------------------------------------------------------------------------- draw

struct Frame<'a> {
    g: &'a Gfx,
    geo: Geo,
    clip: IRect,
    pal: Pal,
    ck: Clock,
}

impl Frame<'_> {
    fn u(&self) -> f32 {
        self.geo.u
    }

    fn text_at(&self, pm: &mut Pixmap, p: &Placed, dx: f32, dy: f32, colors: &[Rgb], op: f32) {
        self.g.text().draw(pm, &p.b, self.geo.x + p.x + dx, self.geo.y + p.y + dy, colors, op, self.clip);
    }

    fn title(&self, pm: &mut Pixmap, l: &TitleL) {
        let (u, ck, pal, tx) = (self.u(), self.ck, self.pal, self.g.text());
        let (ox, oy) = (self.geo.x, self.geo.y);
        let exit = ck.exit();
        if let Some(k) = &l.kicker {
            let p = ck.enter(0.0);
            let [rx, ry, rw, rh] = l.rule;
            fill_rrect(pm, ox + rx, oy + ry, rw * p, rh, rh * 0.5, pal.accent, exit);
            let pt = ck.enter(60.0);
            self.text_at(pm, k, -(1.0 - pt) * 16.0 * u, 0.0, &[pal.accent], pt * exit);
        }
        let head = &l.head;
        let rise = head.b.style.size * 0.28;
        for (i, w) in head.b.words.iter().enumerate() {
            let (op, dy) = ck.rise(l.word_ms[i], rise);
            let col = if w.tag == 1 { pal.accent } else { pal.ink };
            tx.draw_word(pm, &head.b, w, ox + head.x, oy + head.y + dy, col, op, self.clip);
        }
        if let Some(s) = &l.sub {
            let (op, dy) = ck.rise(l.sub_ms, 24.0 * u);
            self.text_at(pm, s, 0.0, dy, &[pal.muted, pal.accent], op);
        }
    }

    fn bullets(&self, pm: &mut Pixmap, l: &BulletsL) {
        let (u, ck, pal) = (self.u(), self.ck, self.pal);
        let (ox, oy) = (self.geo.x, self.geo.y);
        if let Some(t) = &l.title {
            let (op, dy) = ck.rise(0.0, 24.0 * u);
            self.text_at(pm, t, 0.0, dy, &[pal.ink, pal.accent], op);
        }
        let exit = ck.exit();
        for (k, item) in l.items.iter().enumerate() {
            let at = l.item_ms[k];
            if k > 0 {
                let p = ck.enter(at);
                let [x, y, w] = l.rules[k - 1];
                let ry = (oy + y).round();
                raster::fill_rect(pm, IRect::new((ox + x).round() as i32, ry as i32, (ox + x + w * p).round() as i32, ry as i32 + (1.5 * u).round().max(1.0) as i32).intersect(self.clip), mix(pal.surface, pal.line, 0.8 * exit));
            }
            let (op, dy) = ck.rise(at, 24.0 * u);
            let pop = back_out(prog(ck.lt, at, 380.0));
            let (mx, my) = l.markers[k];
            fill_circle(pm, ox + mx, oy + my + dy * 0.5, l.marker_r * pop, pal.accent, op);
            self.text_at(pm, item, 0.0, dy, &[pal.ink, pal.accent], op);
        }
    }

    fn number(&self, pm: &mut Pixmap, l: &NumberL) {
        let (u, ck, pal, tx) = (self.u(), self.ck, self.pal, self.g.text());
        let (ox, oy) = (self.geo.x, self.geo.y);
        let (op, dy) = ck.rise(0.0, 30.0 * u);
        let (pre, num, suf) = count_parts(l, ck.lt);
        let st = l.st;
        let gapw = st.ls * st.size;
        let (wp, wn, ws) = (tx.width(st, pre), tx.width(st, &num), tx.width(st, suf));
        let total = wp + wn + ws + if pre.is_empty() { 0.0 } else { gapw } + if suf.is_empty() { 0.0 } else { gapw };
        let mut x = ox + l.cx - total * 0.5;
        let y = oy + l.base + dy;
        tx.draw_line(pm, st, pre, x, y, pal.accent, op, self.clip);
        if !pre.is_empty() {
            x += wp + gapw;
        }
        tx.draw_line(pm, st, &num, x, y, pal.ink, op, self.clip);
        x += wn + gapw;
        tx.draw_line(pm, st, suf, x, y, pal.accent, op, self.clip);
        let pb = ease_out(prog(ck.lt, 520.0, 520.0));
        let [bx, by, bw, bh] = l.bar;
        let w = bw * pb;
        fill_rrect(pm, ox + bx - w * 0.5, oy + by + dy * 0.6, w, bh, bh * 0.5, pal.accent, ck.exit());
        if let Some(lb) = &l.label {
            let (op, dy) = ck.rise(260.0, 24.0 * u);
            self.text_at(pm, lb, 0.0, dy, &[pal.muted, pal.accent], op);
        }
    }

    fn media(&self, pm: &mut Pixmap, l: &MediaL, media: Option<&Pixmap>) {
        let (u, ck, pal, g) = (self.u(), self.ck, self.pal, self.g);
        let (ox, oy) = (self.geo.x, self.geo.y);
        let p = ck.enter(0.0);
        let exit = ck.exit();
        let op = p * exit;
        if op <= 0.0 {
            return;
        }
        let dy = ((1.0 - p) * 28.0 * u - (1.0 - exit) * 10.0 * u).round();
        let s_in = lerp(0.965, 1.0, p);
        let [cx, cy, cw, ch] = l.card;
        let (x, y) = (ox + cx, oy + cy + dy);
        let (sw, sh) = (cw * s_in, ch * s_in);
        let (mut sx, mut sy) = (x + (cw - sw) * 0.5, y + (ch - sh) * 0.5);
        if s_in >= 0.9999 {
            // Settled: keep the card on whole pixels so a matching video frame copies 1:1.
            sx = sx.round();
            sy = sy.round();
        }
        // Shadow.
        let sh_ = g.shadow(cw as u32, ch as u32, l.radius, 56.0 * u, -14.0 * u);
        let (bx, by) = (x.round() as i32, (y + 30.0 * u).round() as i32);
        let skip = IRect::new(x.round() as i32 + sh_.inner.x0, y.round() as i32 + sh_.inner.y0, x.round() as i32 + sh_.inner.x1, y.round() as i32 + sh_.inner.y1);
        blit_mask_skip(pm, &sh_.sprite, bx, by, pal.black, 0.62 * op, self.clip, Some(skip));
        let round = Round::new(sx, sy, sw, sh, l.radius);
        // Content.
        let mut ring: Option<[f32; 4]> = None;
        let hp = ease_out(prog(ck.lt, 520.0, 420.0));
        match &l.src {
            Some((src, iw, ih)) if l.found => {
                let im = g.image(src, *iw, *ih);
                if let Some(img) = im.as_ref() {
                    let zt = ease_in_out(prog(ck.lt, 0.0, ck.dur));
                    let z = geo_zoom(zt, 1.0, 1.06);
                    let (ou, ov) = match l.highlight {
                        Some(h) => ((h[0] + h[2] * 0.5) as f32, (h[1] + h[3] * 0.5) as f32),
                        None => (0.5, 0.45),
                    };
                    let (ou, ov) = (ou.clamp(0.0, 1.0), ov.clamp(0.0, 1.0));
                    let (dw, dh) = (sw * z, sh * z);
                    let (dx, dy2) = (sx + sw * ou * (1.0 - z), sy + sh * ov * (1.0 - z));
                    if !img.is_opaque() {
                        fill_rrect(pm, sx, sy, sw, sh, l.radius, pal.card2, op);
                    }
                    let spot = l.highlight.map(|h| {
                        let pad = 10.0 * u;
                        // Keep the ring (and its stroke) inside the card.
                        let m = 6.0 * u;
                        let rx0 = (dx + h[0] as f32 * dw - pad).max(sx + m);
                        let ry0 = (dy2 + h[1] as f32 * dh - pad).max(sy + m);
                        let rx1 = (dx + (h[0] + h[2]) as f32 * dw + pad).min(sx + sw - m);
                        let ry1 = (dy2 + (h[1] + h[3]) as f32 * dh + pad).min(sy + sh - m);
                        let r = [rx0, ry0, (rx1 - rx0).max(4.0 * u), (ry1 - ry0).max(4.0 * u)];
                        ring = Some(r);
                        Spot { area: Round::new(r[0], r[1], r[2], r[3], 14.0 * u), keep: (256.0 * (1.0 - 0.38 * hp)) as u32 }
                    });
                    draw_img(pm, img, [dx, dy2, dw, dh], self.clip, op, Some(round), spot.filter(|_| hp > 0.0));
                } else {
                    fill_rrect(pm, sx, sy, sw, sh, l.radius, pal.card2, op);
                }
            }
            Some(_) => fill_rrect(pm, sx, sy, sw, sh, l.radius, pal.card2, op),
            None => match media {
                Some(m) if m.width() > 0 && m.height() > 0 => {
                    let k = (sw / m.width() as f32).max(sh / m.height() as f32);
                    let (dw, dh) = (m.width() as f32 * k, m.height() as f32 * k);
                    let (dx, dy2) = (sx + (sw - dw) * 0.5, sy + (sh - dh) * 0.5);
                    let snap = |v: f32| if (v - v.round()).abs() < 0.02 { v.round() } else { v };
                    let view = raster::ImgRef { w: m.width(), h: m.height(), data: m.data() };
                    raster::draw_img_ref(pm, view, [snap(dx), snap(dy2), dw, dh], self.clip, op, Some(round), None);
                }
                _ => fill_rrect(pm, sx, sy, sw, sh, l.radius, pal.card2, op),
            },
        }
        stroke_rrect(pm, sx + 0.5, sy + 0.5, sw - 1.0, sh - 1.0, l.radius, (1.5 * u).max(1.0), pal.white, 0.10 * op);
        if let Some(r) = ring {
            if hp > 0.0 {
                if let Some(path) = rrect(r[0], r[1], r[2], r[3], 14.0 * u) {
                    let len = raster::rrect_len(r[2], r[3], 14.0 * u);
                    let pulse = 0.85 + 0.15 * (((ck.lt - 940.0).max(0.0) / 1400.0 * std::f64::consts::TAU).cos() as f32);
                    stroke_partial(pm, &path, len, hp, 12.0 * u, pal.accent, 0.22 * op * pulse, Transform::identity());
                    stroke_partial(pm, &path, len, hp, 5.0 * u, pal.accent, op, Transform::identity());
                }
            }
        }
        if let Some(m) = &l.missing {
            self.text_at(pm, m, 0.0, dy, &[pal.muted], op);
        }
        if let Some((b, pill)) = &l.source {
            let pp = back_out(prog(ck.lt, 380.0, 360.0));
            let a = (prog(ck.lt, 380.0, 120.0)) * exit;
            if a > 0.0 {
                let [px, py, pw, ph] = *pill;
                let s = lerp(0.7, 1.0, pp);
                let (cxp, cyp) = (ox + px + pw * 0.5, oy + py + ph * 0.5 + dy);
                fill_rrect(pm, cxp - pw * s * 0.5, cyp - ph * s * 0.5, pw * s, ph * s, ph * s * 0.5, [16, 15, 14], 0.82 * a);
                fill_circle(pm, ox + px + (22.0 + 4.0) * u, cyp, 5.0 * u * s, pal.accent, a);
                if pp > 0.6 {
                    self.text_at(pm, b, 0.0, dy, &[pal.ink_soft], a * ((pp - 0.6) / 0.4).min(1.0));
                }
            }
        }
        if let Some(c) = &l.caption {
            let (op, dy) = ck.rise(220.0, 20.0 * u);
            self.text_at(pm, c, 0.0, dy, &[pal.ink_soft, pal.accent], op);
        }
    }

    fn code(&self, pm: &mut Pixmap, l: &CodeL) {
        let (u, ck, pal, tx) = (self.u(), self.ck, self.pal, self.g.text());
        let (ox, oy) = (self.geo.x, self.geo.y);
        let p = ck.enter(0.0);
        let exit = ck.exit();
        let op = p * exit;
        if op <= 0.0 {
            return;
        }
        let dy = ((1.0 - p) * 28.0 * u - (1.0 - exit) * 10.0 * u).round();
        let [cx, cy, cw, ch] = l.card;
        let (x, y) = (ox + cx, oy + cy + dy);
        let sh_ = self.g.shadow(cw as u32, ch as u32, l.radius, 56.0 * u, -14.0 * u);
        let skip = IRect::new(x.round() as i32 + sh_.inner.x0, y.round() as i32 + sh_.inner.y0, x.round() as i32 + sh_.inner.x1, y.round() as i32 + sh_.inner.y1);
        blit_mask_skip(pm, &sh_.sprite, x.round() as i32, (y + 30.0 * u).round() as i32, pal.black, 0.62 * op, self.clip, Some(skip));
        let body = mix(pal.canvas, pal.card, 0.55);
        fill_rrect(pm, x, y, cw, ch, l.radius, body, op);
        if let Some(path) = top_rounded(x, y, cw, l.bar_h, l.radius) {
            pm.fill_path(&path, &raster::paint(pal.card2, op), FillRule::Winding, Transform::identity(), None);
        }
        raster::fill_rect(pm, IRect::new(x.round() as i32, (y + l.bar_h).round() as i32, (x + cw).round() as i32, (y + l.bar_h).round() as i32 + (1.5 * u).round().max(1.0) as i32).intersect(self.clip), mix(body, pal.line, op));
        stroke_rrect(pm, x + 0.5, y + 0.5, cw - 1.0, ch - 1.0, l.radius, (1.5 * u).max(1.0), pal.white, 0.08 * op);
        for (i, c) in [pal.red, pal.gold, pal.green].iter().enumerate() {
            fill_circle(pm, x + 36.0 * u + i as f32 * 28.0 * u, y + l.bar_h * 0.5, 8.0 * u, *c, op * 0.9);
        }
        if let Some(t) = &l.title {
            self.text_at(pm, t, 0.0, dy, &[pal.muted], op);
        }
        // Typing: a steady rate, done by ~60% of the beat (sooner for short text).
        let t0 = 380.0;
        let rate_done = (l.total as f64 / 28.0) * 1000.0;
        let t1 = (ck.dur * 0.6).min(t0 + rate_done).max(t0 + 300.0);
        let typed = (prog(ck.lt, t0, t1 - t0) as f64 * l.total as f64).round() as usize;
        let mut left = typed;
        let mut cursor = (l.text_x, 0usize);
        let colors = [pal.ink, pal.accent, pal.muted];
        for (i, line) in l.lines.iter().enumerate() {
            let base = oy + l.text_y + dy + i as f32 * l.line_h + (l.line_h - l.st.size * 1.21) * 0.5 + tx.ascent(l.st);
            let show = left.min(line.chars);
            if show > 0 {
                let limit = (ox + l.text_x + show as f32 * l.adv).round() as i32;
                let clip = IRect { x1: limit.min(self.clip.x1), ..self.clip };
                for (col, s, tag) in &line.segs {
                    if *col >= show {
                        break;
                    }
                    tx.draw_line(pm, l.st, s, ox + l.text_x + *col as f32 * l.adv, base, colors[*tag as usize], op, clip);
                }
            }
            cursor = (l.text_x + show as f32 * l.adv, i);
            if left <= line.chars {
                break;
            }
            left -= line.chars + 1;
        }
        let blink = if typed >= l.total { ((ck.lt / 530.0) as i64 % 2 == 0) as u8 as f32 } else { 1.0 };
        if ck.lt >= t0 && blink > 0.0 {
            let cy0 = oy + l.text_y + dy + cursor.1 as f32 * l.line_h + (l.line_h - l.st.size * 1.15) * 0.5;
            fill_rrect(pm, ox + cursor.0 + 2.0 * u, cy0, l.adv * 0.6, l.st.size * 1.15, 2.0 * u, pal.accent, op * 0.9);
        }
    }

    fn quote(&self, pm: &mut Pixmap, l: &QuoteL) {
        let (u, ck, pal, tx) = (self.u(), self.ck, self.pal, self.g.text());
        let (ox, oy) = (self.geo.x, self.geo.y);
        let (mx, my, r) = l.mark;
        let mop = prog(ck.lt, 0.0, 160.0) * ck.exit();
        if let Some(path) = quote_mark(ox + mx, oy + my + (1.0 - ck.enter(0.0)) * 30.0 * u, r) {
            pm.fill_path(&path, &raster::paint(pal.accent, mop), FillRule::Winding, Transform::identity(), None);
        }
        let b = &l.text.b;
        for w in &b.words {
            let (op, dy) = ck.rise(160.0 + 110.0 * w.line as f64, 24.0 * u);
            let col = if w.tag == 1 { pal.accent } else { pal.ink };
            tx.draw_word(pm, b, w, ox + l.text.x, oy + l.text.y + dy, col, op, self.clip);
        }
        if let Some(by) = &l.by {
            let at = 160.0 + 110.0 * b.lines as f64 + 120.0;
            let (op, dy) = ck.rise(at, 20.0 * u);
            let [rx, ry, rw, rh] = l.rule;
            fill_rrect(pm, ox + rx, oy + ry + dy, rw * ck.enter(at), rh, rh * 0.5, pal.accent, op);
            self.text_at(pm, by, 0.0, dy, &[pal.muted], op);
        }
    }
}

/// Prefix, number and suffix shown `lt` ms into a number beat: counts from zero over ~0.8 s
/// keeping the original grouping and decimals.
fn count_parts(l: &NumberL, lt: f64) -> (&str, String, &str) {
    match &l.fmt {
        Some(f) => {
            let k = ease_out(prog(lt, 120.0, 800.0)) as f64;
            (f.prefix.as_str(), f.number(f.value * k), f.suffix.as_str())
        }
        None => ("", l.value.clone(), ""),
    }
}

/// An opening quote mark: two "6" shapes (ball plus a rising tail), first ball centred at
/// (cx, cy) with radius r. All sub-paths wind the same way so they union under one fill.
fn quote_mark(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Path> {
    let k = 0.552_284_8 * r;
    let mut pb = PathBuilder::new();
    for i in 0..2 {
        let cx = cx + i as f32 * 2.7 * r;
        pb.move_to(cx + r, cy);
        pb.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
        pb.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
        pb.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
        pb.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
        pb.close();
        pb.move_to(cx - r, cy);
        pb.quad_to(cx - r * 1.05, cy - r * 2.3, cx + r * 0.95, cy - r * 3.0);
        pb.line_to(cx + r * 1.05, cy - r * 2.55);
        pb.quad_to(cx - r * 0.05, cy - r * 2.05, cx + r * 0.2, cy - r * 0.9);
        pb.close();
    }
    pb.finish()
}

/// A rect whose top corners are rounded (title bars).
fn top_rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w * 0.5).min(h);
    let k = 0.552_284_8 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(x, y + h);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h);
    pb.close();
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme;
    use std::collections::BTreeMap;

    fn gfx() -> Gfx {
        Gfx::new(Theme::from_style(&BTreeMap::new()), 1920, 1080, std::path::Path::new(".")).unwrap()
    }

    fn panel() -> Geo {
        Geo::new(Rect::from_xywh(19.2, 19.2, 1473.8, 1041.6).unwrap())
    }

    fn beat(kind: BeatKind) -> ResolvedBeat {
        ResolvedBeat { idx: 0, lane: kind.lane(), kind, start_ms: 0.0, end_ms: 4000.0, items_ms: vec![], words: vec![], origin: String::new() }
    }

    #[test]
    fn count_up_keeps_the_format() {
        let g = gfx();
        let l = build_number(&g, &panel(), "$10,075", Some("per video"));
        let at = |lt: f64| {
            let (p, n, s) = count_parts(&l, lt);
            format!("{p}{n}{s}")
        };
        assert_eq!(at(0.0), "$0");
        let mid = at(400.0);
        assert!(mid.starts_with('$') && mid.contains(',') && mid != "$10,075", "mid-way stays formatted: {mid}");
        let v: f64 = mid[1..].replace(',', "").parse().unwrap();
        assert!(v > 1000.0 && v < 10075.0);
        assert_eq!(at(2000.0), "$10,075");
        let c = build_number(&g, &panel(), "4.76M", None);
        let (_, n, s) = count_parts(&c, 300.0);
        assert!(n.contains('.') && n.split('.').nth(1).unwrap().len() == 2 && s == "M", "{n}{s}");
        let plain = build_number(&g, &panel(), "Day one", None);
        assert_eq!(count_parts(&plain, 0.0).1, "Day one");
    }

    #[test]
    fn long_titles_shrink_and_fit() {
        let g = gfx();
        let geo = panel();
        let short = build_title(&g, &geo, &beat(BeatKind::Title { text: "Short".into(), kicker: None, sub: None }), "Short", None, None);
        let text = "This is an extremely long headline that keeps going and going well past anything that could ever fit on three lines at the preferred size";
        let long = build_title(&g, &geo, &beat(BeatKind::Title { text: text.into(), kicker: Some("kicker".into()), sub: Some("and a sub line too".into()) }), text, Some("kicker"), Some("and a sub line too"));
        assert!(long.head.b.style.size < short.head.b.style.size * 0.7, "long title shrinks: {} vs {}", long.head.b.style.size, short.head.b.style.size);
        assert!(long.head.b.lines <= 3);
        assert!(long.head.x + long.head.b.w <= geo.w - 100.0 * geo.u, "fits the width");
        let bottom = long.sub.as_ref().map(|s| s.y + s.b.h()).unwrap();
        assert!(long.kicker.as_ref().unwrap().y >= 0.0 && bottom <= geo.h, "fits the height: {bottom} > {}", geo.h);
        // A single unbreakable word wider than the panel still fits.
        let word = "Supercalifragilisticexpialidocious-and-then-some-more";
        let w = build_title(&g, &geo, &beat(BeatKind::Title { text: word.into(), kicker: None, sub: None }), word, None, None);
        assert!(w.head.x + w.head.b.w <= geo.w, "{} > {}", w.head.b.w, geo.w);
    }

    #[test]
    fn bullets_and_code_fit_the_panel() {
        let g = gfx();
        let geo = panel();
        let items: Vec<String> = (0..12).map(|i| format!("Item number {i} with a fairly long description that wraps")).collect();
        let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        let b = build_bullets(&g, &geo, &beat(BeatKind::Bullets { title: Some("Twelve things".into()), items: vec![] }), Some("Twelve things"), &refs);
        let last = b.items.last().unwrap();
        assert!(last.y + last.b.h() <= geo.h, "bullets fit");
        assert!(b.items.iter().all(|p| p.x + p.b.w <= geo.w));
        let code = (0..60).map(|i| format!("let line_{i} = compute(\"a fairly long argument string number {i}\");")).collect::<Vec<_>>().join("\n");
        let c = build_code(&g, &geo, &code, Some("main.rs"));
        assert!(c.card[1] >= 0.0 && c.card[1] + c.card[3] <= geo.h + 0.5, "code card fits: {:?}", c.card);
        assert!(c.card[0] >= 0.0 && c.card[0] + c.card[2] <= geo.w);
    }

    #[test]
    fn code_wraps_with_a_hanging_indent() {
        let w = wrap_code("> one two three four five six seven", 14);
        assert!(w.len() > 1);
        assert!(w[1].starts_with("  ") && !w[1].starts_with("   "), "{w:?}");
        assert!(w.iter().all(|l| l.chars().count() <= 14), "{w:?}");
    }

    #[test]
    fn headline_words_follow_the_narration() {
        let mut b = beat(BeatKind::Title { text: "Connect Claude".into(), kicker: None, sub: None });
        b.words = vec![
            cutroom_core::resolve::OutWord { id: 0, text: "connect".into(), start_ms: 130.0, end_ms: 400.0 },
            cutroom_core::resolve::OutWord { id: 1, text: "your".into(), start_ms: 500.0, end_ms: 700.0 },
            cutroom_core::resolve::OutWord { id: 2, text: "Claude,".into(), start_ms: 1500.0, end_ms: 1900.0 },
        ];
        let t = word_times(&b, &tokens("Connect Claude", true));
        assert!((t[0] - 70.0).abs() < 1e-6, "{t:?}");
        assert!((t[1] - 1440.0).abs() < 1e-6, "{t:?}");
        let none = word_times(&beat(BeatKind::Title { text: "a b c".into(), kicker: None, sub: None }), &tokens("a b c", true));
        assert_eq!(none, vec![0.0, 90.0, 180.0]);
    }
}
