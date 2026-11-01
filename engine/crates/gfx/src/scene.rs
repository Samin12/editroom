//! `scene` beats: layered, keyframed animations (`cutroom_core::scene`). Everything static (text
//! blocks, label sprites, image sizes, code layout, arrow geometry) is built once per beat and
//! cached with the stage layouts; every frame only computes each layer's state (position,
//! scale, opacity, draw-on progress) from the beat clock and draws it.
//!
//! Geometry: the scene box is the panel (`place: stage`), the frame (`place: frame`) or an inset
//! card (`bg: card`). Layer `x`/`y`/`w`/`h` are 0..1 of the box; sizes are units of box width /
//! 1920.

use crate::anim::{back_out, ease_in_out, ease_out, lerp, prog, Clock, ENTER_MS};
use crate::pal::{mix, parse_hex, Pal, Rgb};
use crate::raster::{self, blit_mask_skip, draw_img, fill_circle, fill_rrect, rrect, stroke_partial, stroke_rrect, IRect, Img, Round, Spot};
use crate::stage::{self, Geo};
use crate::text::{tokens, Block, FontId, Style};
use crate::Gfx;
use cutroom_core::resolve::ResolvedBeat;
use cutroom_core::scene::{unflatten, Align, Bg, Ease, ImgFit, Layer, LayerType as T, Mark, Motion, Place, Pt, Weight};
use cutroom_core::spec::BeatKind;
use tiny_skia::{FillRule, LineCap, LineJoin, Path, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

/// Default stagger between layers that have no `in.at` (ms).
const STAGGER_MS: f64 = 90.0;
/// Rise distance of `up` / `down` entrances and slide distance of `left` / `right` (units).
const RISE_U: f32 = 28.0;
const SLIDE_U: f32 = 64.0;
/// Counters count over this long (ms).
const COUNT_MS: f64 = 1000.0;
/// Keyframe transitions default to this long (ms).
const KEY_MS: f64 = 500.0;

/// Arrow head: tip, unit direction, length.
type Head = ((f32, f32), (f32, f32), f32);
/// A code line: its length in chars and its coloured segments (column, text, colour tag).
type CodeLine = (usize, Vec<(usize, String, u8)>);

pub(crate) struct SceneL {
    /// Scene box relative to the drawing rect's origin (px).
    bx: [f32; 4],
    u: f32,
    /// The inset card (`bg: card`), relative to the drawing rect.
    card: Option<[f32; 4]>,
    layers: Vec<LayerL>,
}

struct Enter {
    at: f64,
    ms: f64,
    motion: Motion,
    draw: bool,
}

struct Exit {
    at: f64,
    ms: f64,
    motion: Motion,
}

struct KeyL {
    t: f64,
    ms: f64,
    x: Option<f32>,
    y: Option<f32>,
    scale: Option<f32>,
    opacity: Option<f32>,
    rotate: Option<f32>,
}

struct LayerL {
    /// Anchor (px, relative to the box): text/counter anchor point, top-left of boxes, images,
    /// highlights and code, circle centre, first point of lines and arrows.
    ax: f32,
    ay: f32,
    /// Scale / rotation pivot relative to the anchor.
    pivot: (f32, f32),
    ease: Ease,
    opacity: f32,
    enter: Enter,
    exit: Option<Exit>,
    keys: Vec<KeyL>,
    shape: Shape,
}

enum Shape {
    Text {
        block: Block,
        /// Block top-left relative to the anchor.
        off: (f32, f32),
        /// Colours of plain and `*accent*` words.
        colors: [Rgb; 2],
        /// The block pre-rendered (for scaled frames).
        img: Img,
        pad: f32,
        /// Marker / underline rects behind the accent words (relative to the block top-left).
        marks: Vec<[f32; 4]>,
        mark: Option<Mark>,
    },
    Rect {
        w: f32,
        h: f32,
        radius: f32,
        circle: bool,
        fill: Option<Rgb>,
        stroke: Option<Rgb>,
        sw: f32,
        /// Hairline when there is no stroke.
        hair: bool,
        shadow: bool,
        label: Option<Img>,
        /// The settled shape (fill, stroke, label) pre-rendered, 1 px of margin around it.
        sprite: Img,
    },
    Path {
        path: Path,
        len: f32,
        color: Rgb,
        width: f32,
        dash: bool,
        head: Option<Head>,
    },
    Image {
        src: String,
        /// Frame (layer box) size.
        w: f32,
        h: f32,
        /// Drawn image rect relative to the anchor (contain: inside the frame; cover: covers it).
        draw: [f32; 4],
        load: (u32, u32),
        fit: ImgFit,
        radius: f32,
        shadow: bool,
        ring: Option<[f64; 4]>,
        found: bool,
        source: Option<(Img, f32, f32)>,
        missing: Option<Img>,
    },
    Code(Box<CodeL>),
    Counter {
        from: f64,
        to: f64,
        decimals: usize,
        commas: bool,
        prefix: String,
        suffix: String,
        st: Style,
        align: Align,
        color: Rgb,
    },
    Highlight {
        w: f32,
        h: f32,
        color: Rgb,
        mark: Mark,
        radius: f32,
    },
}

struct CodeL {
    w: f32,
    h: f32,
    radius: f32,
    bar_h: f32,
    title: Option<(Img, f32, f32)>,
    st: Style,
    adv: f32,
    line_h: f32,
    pad: f32,
    lines: Vec<CodeLine>,
    total: usize,
}

/// The state of one layer at one instant.
#[derive(Clone, Copy, Debug)]
struct St {
    op: f32,
    dx: f32,
    dy: f32,
    s: f32,
    rot: f32,
    draw: f32,
    ax: f32,
    ay: f32,
}

// ------------------------------------------------------------------------------------ layout

pub(crate) fn build(g: &Gfx, rect: &Geo, beat: &ResolvedBeat) -> SceneL {
    let BeatKind::Scene { bg, layers, .. } = &beat.kind else {
        return SceneL { bx: [0.0, 0.0, rect.w, rect.h], u: rect.u, card: None, layers: vec![] };
    };
    let u0 = rect.w / 1920.0;
    let card = (*bg == Bg::Card).then(|| {
        let m = 72.0 * u0;
        [m, m, rect.w - 2.0 * m, rect.h - 2.0 * m]
    });
    let bx = card.unwrap_or([0.0, 0.0, rect.w, rect.h]);
    let u = bx[2] / 1920.0;
    let pal = g.pal();
    let times = unflatten(layers, &beat.items_ms);
    let start = beat.start_ms;
    // base rects of layers arrows can attach to (box px)
    let anchors: Vec<(Option<&str>, Option<Attach>)> = layers.iter().map(|l| (l.id.as_deref(), attach_of(l, bx))).collect();
    let mut staggered = 0usize;
    let mut out = Vec::with_capacity(layers.len());
    for (i, l) in layers.iter().enumerate() {
        let lt = &times[i];
        let default_motion = if l.ty.draws() { Motion::Fade } else { Motion::Up };
        let e = l.enter.clone().unwrap_or_default();
        let draw = e.draw.unwrap_or(l.ty.draws());
        let motion = e.from.unwrap_or(default_motion);
        let at = match lt.enter {
            Some(t) => t - start,
            None => {
                let v = (staggered as f64 * STAGGER_MS).min(900.0);
                staggered += 1;
                v
            }
        };
        let ms = e.ms.unwrap_or(if draw && l.ty.draws() {
            600.0
        } else if motion == Motion::Pop {
            420.0
        } else {
            ENTER_MS
        });
        let enter = Enter { at, ms, motion, draw };
        let exit = l.exit.as_ref().and_then(|x| {
            let at = lt.exit? - start;
            Some(Exit { at, ms: x.ms.unwrap_or(260.0), motion: x.to.unwrap_or(Motion::Fade) })
        });
        let keys = l
            .keys
            .iter()
            .zip(&lt.keys)
            .filter(|(_, t)| t.is_finite())
            .map(|(k, t)| KeyL {
                t: t - start,
                ms: k.ms.unwrap_or(KEY_MS),
                x: k.x.map(|v| v as f32 * bx[2]),
                y: k.y.map(|v| v as f32 * bx[3]),
                scale: k.scale.map(|v| v as f32),
                opacity: k.opacity.map(|v| v as f32),
                rotate: k.rotate.map(|v| v as f32),
            })
            .collect();
        let (ax, ay, pivot, shape) = shape_of(g, &pal, l, bx, u, &anchors);
        out.push(LayerL { ax, ay, pivot, ease: l.ease.unwrap_or(Ease::Out), opacity: l.opacity.unwrap_or(1.0).clamp(0.0, 1.0) as f32, enter, exit, keys, shape });
    }
    SceneL { bx, u, card, layers: out }
}

/// What an arrow end can attach to: a rect or a circle (box px).
#[derive(Clone, Copy, Debug)]
enum Attach {
    Rect([f32; 4]),
    Circle(f32, f32, f32),
}

impl Attach {
    fn center(&self) -> (f32, f32) {
        match *self {
            Attach::Rect(r) => (r[0] + r[2] * 0.5, r[1] + r[3] * 0.5),
            Attach::Circle(x, y, _) => (x, y),
        }
    }
    /// Where the segment from the centre toward `p` leaves the shape, pushed out by `gap`.
    fn edge(&self, p: (f32, f32), gap: f32) -> (f32, f32) {
        let c = self.center();
        let (dx, dy) = (p.0 - c.0, p.1 - c.1);
        let d = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / d, dy / d);
        let t = match *self {
            Attach::Rect(r) => {
                let tx = if ux.abs() > 1e-6 { r[2] * 0.5 / ux.abs() } else { f32::INFINITY };
                let ty = if uy.abs() > 1e-6 { r[3] * 0.5 / uy.abs() } else { f32::INFINITY };
                tx.min(ty)
            }
            Attach::Circle(_, _, r) => r,
        };
        let t = (t + gap).min(d * 0.45);
        (c.0 + ux * t, c.1 + uy * t)
    }
}

fn attach_of(l: &Layer, bx: [f32; 4]) -> Option<Attach> {
    let (x, y) = (l.x? as f32 * bx[2], l.y? as f32 * bx[3]);
    match l.ty {
        T::Box | T::Image => Some(Attach::Rect([x, y, l.w? as f32 * bx[2], l.h? as f32 * bx[3]])),
        T::Circle => Some(Attach::Circle(x, y, l.r? as f32 * bx[3])),
        _ => None,
    }
}

pub(crate) fn color(pal: &Pal, s: &str) -> Option<Rgb> {
    let k = s.trim().to_ascii_lowercase();
    Some(match k.as_str() {
        "accent" => pal.accent,
        "ink" => pal.ink,
        "ink_soft" => pal.ink_soft,
        "muted" => pal.muted,
        "faint" => pal.faint,
        "card" => pal.card,
        "card2" => pal.card2,
        "surface" => pal.surface,
        "canvas" => pal.canvas,
        "line" => pal.line,
        "green" => pal.green,
        "red" => pal.red,
        "gold" => pal.gold,
        "white" => pal.white,
        "black" => pal.black,
        "accent_soft" => pal.accent_soft,
        "none" => return None,
        _ => return parse_hex(&k),
    })
}

fn font_of(w: Option<Weight>, size_u: f32) -> FontId {
    match w {
        Some(Weight::Regular) => FontId::Regular,
        Some(Weight::Semibold) => FontId::SemiBold,
        Some(Weight::Bold) => FontId::Bold,
        Some(Weight::Mono) => FontId::Mono,
        None if size_u >= 72.0 => FontId::Bold,
        None => FontId::SemiBold,
    }
}

fn style_for(font: FontId, size: f32, size_u: f32) -> Style {
    let st = Style::new(font, size);
    match font {
        FontId::Bold if size_u >= 72.0 => st.ls(-0.025).lh(1.08),
        FontId::Bold => st.ls(-0.01).lh(1.15),
        FontId::Mono => st.lh(1.45),
        _ => st.lh(1.25),
    }
}

/// Render a block into a premultiplied sprite with `pad` px around it.
fn block_img(g: &Gfx, b: &Block, colors: &[Rgb], pad: f32) -> Img {
    let w = (b.w + 2.0 * pad).ceil().max(1.0) as u32;
    let h = (b.h() + 2.0 * pad).ceil().max(1.0) as u32;
    let mut p = Pixmap::new(w, h).unwrap_or_else(|| Pixmap::new(1, 1).unwrap());
    let clip = IRect::of(&p);
    g.text().draw(&mut p, b, pad, pad, colors, 1.0, clip);
    Img::from_pixmap(p)
}

fn align_block(b: &mut Block, align: Align) {
    match align {
        Align::Left => {}
        Align::Center => {
            let w = b.w;
            b.center(w);
        }
        Align::Right => {
            for wd in &mut b.words {
                wd.x += b.w - b.line_w[wd.line];
            }
        }
    }
}

/// Label (+ sub line) centred, fitted inside `mw` x `mh`. `cols`: label, accent words, sub.
#[allow(clippy::too_many_arguments)]
fn label_img(g: &Gfx, label: &str, sub: Option<&str>, size: f32, mw: f32, mh: f32, cols: [Rgb; 3], u: f32) -> Img {
    let tx = g.text();
    let sub_size = (size * 0.56).max(24.0 * u);
    let sub_b = sub.filter(|s| !s.trim().is_empty()).map(|s| {
        let mut b = tx.fit(&tokens(s, true), Style::new(FontId::Regular, sub_size).lh(1.25), mw, 2, mh * 0.4, 18.0 * u);
        let w = b.w;
        b.center(w);
        b
    });
    let gap = 10.0 * u;
    let room = mh - sub_b.as_ref().map(|b| b.h() + gap).unwrap_or(0.0);
    let mut lb = tx.fit(&tokens(label, true), Style::new(FontId::SemiBold, size).ls(-0.01).lh(1.18), mw, 3, room.max(mh * 0.4), 16.0 * u);
    let lw = lb.w;
    lb.center(lw);
    let w = lb.w.max(sub_b.as_ref().map(|b| b.w).unwrap_or(0.0));
    let pad = (0.3 * size).ceil();
    let total_h = lb.h() + sub_b.as_ref().map(|b| b.h() + gap).unwrap_or(0.0);
    let iw = (w + 2.0 * pad).ceil() as u32;
    let ih = (total_h + 2.0 * pad).ceil() as u32;
    let mut p = Pixmap::new(iw.max(1), ih.max(1)).unwrap();
    let clip = IRect::of(&p);
    tx.draw(&mut p, &lb, pad + (w - lb.w) * 0.5, pad, &[cols[0], cols[1]], 1.0, clip);
    if let Some(sb) = &sub_b {
        tx.draw(&mut p, sb, pad + (w - sb.w) * 0.5, pad + lb.h() + gap, &[cols[2], cols[1]], 1.0, clip);
    }
    Img::from_pixmap(p)
}

/// The settled look of a box or circle as a sprite with `M` px of margin (stroke overhang).
#[allow(clippy::too_many_arguments)]
fn rect_sprite(pal: &Pal, w: f32, h: f32, radius: f32, circle: bool, fill: Option<Rgb>, stroke: Option<Rgb>, sw: f32, hair: bool, label: Option<&Img>, u: f32) -> Img {
    let m = (sw * 0.5).ceil() + 1.0;
    let (iw, ih) = ((w + 2.0 * m).ceil() as u32, (h + 2.0 * m).ceil() as u32);
    let Some(mut p) = Pixmap::new(iw.max(1), ih.max(1)) else { return Img::default() };
    let path = if circle { PathBuilder::from_circle(m + w * 0.5, m + h * 0.5, w * 0.5) } else { rrect(m, m, w, h, radius) };
    if let Some(path) = path {
        if let Some(f) = fill {
            p.fill_path(&path, &raster::paint(f, 1.0), FillRule::Winding, Transform::identity(), None);
        }
        if let Some(c) = stroke {
            let st = Stroke { width: sw, ..Stroke::default() };
            p.stroke_path(&path, &raster::paint(c, 1.0), &st, Transform::identity(), None);
        } else if hair {
            let st = Stroke { width: (1.5 * u).max(1.0), ..Stroke::default() };
            p.stroke_path(&path, &raster::paint(pal.white, 0.08), &st, Transform::identity(), None);
        }
    }
    if let Some(img) = label {
        let (x, y) = ((m + (w - img.w as f32) * 0.5).round(), (m + (h - img.h as f32) * 0.5).round());
        let clip = IRect::of(&p);
        draw_img(&mut p, img, [x, y, img.w as f32, img.h as f32], clip, 1.0, None, None);
    }
    Img::from_pixmap(p)
}

/// Rough colour distance (sum of channel differences).
fn dist(a: Rgb, b: Rgb) -> u32 {
    a.iter().zip(b).map(|(x, y)| (*x as i32 - y as i32).unsigned_abs()).sum()
}

/// Rects behind each run of accent words (relative to the block top-left).
fn accent_runs(g: &Gfx, b: &Block, mark: Mark) -> Vec<[f32; 4]> {
    let tx = g.text();
    let size = b.style.size;
    let mut out: Vec<[f32; 4]> = vec![];
    let mut run: Option<(usize, f32, f32)> = None; // line, x0, x1
    let flush = |run: &mut Option<(usize, f32, f32)>, out: &mut Vec<[f32; 4]>| {
        if let Some((line, x0, x1)) = run.take() {
            let base = b.baseline(line);
            let r = match mark {
                Mark::Marker => [x0 - 0.12 * size, base - b.cap() - 0.2 * size, x1 - x0 + 0.24 * size, b.cap() + 0.42 * size],
                Mark::Underline => {
                    let h = (0.075 * size).max(3.0);
                    [x0, base + 0.16 * size, x1 - x0, h]
                }
            };
            out.push(r);
        }
    };
    for w in &b.words {
        let x1 = w.x + tx.width(b.style, &w.text);
        match (&mut run, w.tag == 1) {
            (Some((line, _, end)), true) if *line == w.line => *end = x1,
            (_, true) => {
                flush(&mut run, &mut out);
                run = Some((w.line, w.x, x1));
            }
            (_, false) => flush(&mut run, &mut out),
        }
    }
    flush(&mut run, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn shape_of(g: &Gfx, pal: &Pal, l: &Layer, bx: [f32; 4], u: f32, anchors: &[(Option<&str>, Option<Attach>)]) -> (f32, f32, (f32, f32), Shape) {
    let tx = g.text();
    let (bw, bh) = (bx[2], bx[3]);
    let px = |v: Option<f64>| v.unwrap_or(0.0) as f32 * bw;
    let py = |v: Option<f64>| v.unwrap_or(0.0) as f32 * bh;
    let (x, y) = (px(l.x), py(l.y));
    let col = |s: &Option<String>, d: Rgb| s.as_deref().map(|c| color(pal, c)).unwrap_or(Some(d));
    match l.ty {
        T::Text => {
            let size_u = l.size.unwrap_or(56.0) as f32;
            let font = font_of(l.weight, size_u);
            let st = style_for(font, size_u * u, size_u);
            let align = l.align.unwrap_or(Align::Left);
            let margin = 48.0 * u;
            let maxw = match (l.w, align) {
                (Some(w), _) => w as f32 * bw,
                (None, Align::Left) => bw - x - margin,
                (None, Align::Center) => 2.0 * x.min(bw - x) - margin,
                (None, Align::Right) => x - margin,
            }
            .max(200.0 * u);
            let mut b = tx.fit(&tokens(l.text.as_deref().unwrap_or(""), true), st, maxw, l.lines.unwrap_or(3).max(1), (bh - y).max(bh * 0.2), 20.0 * u);
            align_block(&mut b, align);
            let offx = match align {
                Align::Left => 0.0,
                Align::Center => -b.w * 0.5,
                Align::Right => -b.w,
            };
            let color = col(&l.color, pal.ink).unwrap_or(pal.ink);
            // a marker carries the emphasis itself; the words stay in the text colour
            let colors = if l.style == Some(Mark::Marker) { [color, color] } else { [color, pal.accent] };
            let pad = (0.35 * b.style.size).ceil();
            let img = block_img(g, &b, &colors, pad);
            let marks = l.style.map(|m| accent_runs(g, &b, m)).unwrap_or_default();
            let pivot = (offx + b.w * 0.5, b.h() * 0.5);
            (x, y, pivot, Shape::Text { block: b, off: (offx, 0.0), colors, img, pad, marks, mark: l.style })
        }
        T::Box | T::Circle => {
            let circle = l.ty == T::Circle;
            let (w, h) = if circle {
                let r = l.r.unwrap_or(0.05) as f32 * bh;
                (2.0 * r, 2.0 * r)
            } else {
                (px(l.w), py(l.h))
            };
            let fill = match &l.fill {
                Some(f) => color(pal, f),
                None => Some(pal.card2),
            };
            let stroke = l.stroke.as_deref().and_then(|s| color(pal, s));
            let radius = if circle { w * 0.5 } else { l.radius.map(|r| r as f32 * u).unwrap_or(24.0 * u).min(w * 0.5).min(h * 0.5) };
            let label = l.label.as_deref().filter(|s| !s.trim().is_empty()).map(|lab| {
                let inner = if circle { 0.72 } else { 1.0 };
                let pad = (40.0 * u).min(w * 0.12).min(h * 0.18);
                let size = l.size.map(|s| s as f32 * u).unwrap_or((56.0 * u).min(h * 0.34).max(22.0 * u));
                let ink = col(&l.color, pal.ink).unwrap_or(pal.ink);
                // on an accent-coloured fill, accent words switch to the canvas colour
                let on_accent = fill.is_some_and(|f| dist(f, pal.accent) < 90);
                let acc = if on_accent { pal.canvas } else { pal.accent };
                let sub = if on_accent { mix(ink, fill.unwrap_or(pal.accent), 0.25) } else { pal.muted };
                label_img(g, lab, l.sub.as_deref(), size, w * inner - 2.0 * pad, h * inner - 2.0 * pad, [ink, acc, sub], u)
            });
            let (ax, ay) = (x, y);
            let pivot = if circle { (0.0, 0.0) } else { (w * 0.5, h * 0.5) };
            let sw = l.width.map(|v| v as f32 * u).unwrap_or(4.0 * u).max(1.0);
            let hair = stroke.is_none() && fill.is_some();
            let sprite = rect_sprite(pal, w, h, radius, circle, fill, stroke, sw, hair, label.as_ref(), u);
            (ax, ay, pivot, Shape::Rect { w, h, radius, circle, fill, stroke, sw, hair, shadow: l.shadow.unwrap_or(fill.is_some()), label, sprite })
        }
        T::Line | T::Arrow => {
            let point = |p: &Option<Pt>| -> Option<(f32, f32)> {
                match p {
                    Some(Pt::Xy(v)) => Some((v[0] as f32 * bw, v[1] as f32 * bh)),
                    _ => None,
                }
            };
            let att = |p: &Option<Pt>| -> Option<Attach> {
                match p {
                    Some(Pt::Id(id)) => anchors.iter().find(|(lid, _)| *lid == Some(id.as_str())).and_then(|(_, a)| *a),
                    _ => None,
                }
            };
            let gap = 14.0 * u;
            let mut pts: Vec<(f32, f32)> = match &l.points {
                Some(ps) if ps.len() >= 2 => ps.iter().map(|p| (p[0] as f32 * bw, p[1] as f32 * bh)).collect(),
                _ => {
                    let (fa, ta) = (att(&l.from), att(&l.to));
                    let aim_from = point(&l.from).or(fa.map(|a| a.center())).unwrap_or((0.0, 0.0));
                    let aim_to = point(&l.to).or(ta.map(|a| a.center())).unwrap_or((bw * 0.1, 0.0));
                    let a = match fa {
                        Some(f) => f.edge(aim_to, gap),
                        None => aim_from,
                    };
                    let b = match ta {
                        Some(t) => t.edge(aim_from, gap),
                        None => aim_to,
                    };
                    vec![a, b]
                }
            };
            let arrow = l.ty == T::Arrow;
            let width = l.width.map(|v| v as f32 * u).unwrap_or(if arrow { 6.0 } else { 4.0 } * u).max(1.0);
            let color = col(&l.color, if arrow { pal.accent } else { pal.muted }).unwrap_or(pal.accent);
            let (x0, y0) = pts[0];
            for p in &mut pts {
                p.0 -= x0;
                p.1 -= y0;
            }
            let curve = l.curve.unwrap_or(0.0) as f32;
            let (mut pb, mut len) = (PathBuilder::new(), 0.0f32);
            let mut head = None;
            if arrow {
                let (a, b) = (pts[0], *pts.last().unwrap());
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-3);
                let ctrl = ((a.0 + b.0) * 0.5 - dy / d * curve * d, (a.1 + b.1) * 0.5 + dx / d * curve * d);
                // direction at the tip
                let (tx_, ty_) = if curve.abs() > 1e-4 { (b.0 - ctrl.0, b.1 - ctrl.1) } else { (dx, dy) };
                let tl = (tx_ * tx_ + ty_ * ty_).sqrt().max(1e-3);
                let dir = (tx_ / tl, ty_ / tl);
                let hl = (width * 3.6).max(22.0 * u).min(d * 0.4);
                // shaft ends inside the head so the round cap never pokes through the tip
                let end = (b.0 - dir.0 * hl * 0.7, b.1 - dir.1 * hl * 0.7);
                pb.move_to(a.0, a.1);
                if curve.abs() > 1e-4 {
                    pb.quad_to(ctrl.0, ctrl.1, end.0, end.1);
                    len = quad_len(a, ctrl, end);
                } else {
                    pb.line_to(end.0, end.1);
                    len = ((end.0 - a.0).powi(2) + (end.1 - a.1).powi(2)).sqrt();
                }
                head = Some((b, dir, hl));
            } else {
                pb.move_to(pts[0].0, pts[0].1);
                for w in pts.windows(2) {
                    pb.line_to(w[1].0, w[1].1);
                    len += ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
                }
            }
            let path = pb.finish().unwrap_or_else(|| {
                let mut p = PathBuilder::new();
                p.move_to(0.0, 0.0);
                p.line_to(0.1, 0.0);
                p.finish().unwrap()
            });
            let (minx, maxx) = pts.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
            let (miny, maxy) = pts.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
            let pivot = ((minx + maxx) * 0.5, (miny + maxy) * 0.5);
            (x0, y0, pivot, Shape::Path { path, len: len.max(1.0), color, width, dash: l.dash.unwrap_or(false), head })
        }
        T::Image => {
            let (w, h) = (px(l.w), py(l.h));
            let src = l.src.clone().unwrap_or_default();
            let dims = crate::media::dims(&g.resolve_path(&src));
            let fit = l.fit.unwrap_or(ImgFit::Contain);
            let (iw, ih) = dims.map(|(a, b)| (a as f32, b as f32)).unwrap_or((w.max(1.0), h.max(1.0)));
            let k = match fit {
                ImgFit::Contain => (w / iw).min(h / ih),
                ImgFit::Cover => (w / iw).max(h / ih),
            };
            let (dw, dh) = ((iw * k).round(), (ih * k).round());
            let draw = [((w - dw) * 0.5).round(), ((h - dh) * 0.5).round(), dw, dh];
            // loaded a little larger so push-ins stay sharp
            let load = ((dw * 1.12).ceil().max(1.0) as u32, (dh * 1.12).ceil().max(1.0) as u32);
            let radius = l.radius.map(|r| r as f32 * u).unwrap_or(20.0 * u);
            let source = l.source.as_deref().filter(|s| !s.trim().is_empty()).map(|s| {
                let b = tx.fit(&tokens(s, false), Style::new(FontId::SemiBold, 26.0 * u).lh(1.2), dw * 0.7, 1, 60.0 * u, 18.0 * u);
                let (ph, padx, dot) = (b.h() + 20.0 * u, 20.0 * u, 18.0 * u);
                let pw = b.w + 2.0 * padx + dot;
                let mut p = Pixmap::new(pw.ceil() as u32, ph.ceil() as u32).unwrap();
                fill_rrect(&mut p, 0.0, 0.0, pw, ph, ph * 0.5, [16, 15, 14], 0.84);
                fill_circle(&mut p, padx + 4.0 * u, ph * 0.5, 5.0 * u, pal.accent, 1.0);
                let clip = IRect::of(&p);
                tx.draw(&mut p, &b, padx + dot, (ph - b.h()) * 0.5, &[pal.ink_soft], 1.0, clip);
                (Img::from_pixmap(p), pw, ph)
            });
            let found = dims.is_some();
            let missing = (!found).then(|| {
                let mut b = tx.fit(&tokens(&format!("Image not found: {src}"), false), Style::new(FontId::SemiBold, 30.0 * u), w * 0.8, 2, h * 0.5, 16.0 * u);
                let bw_ = b.w;
                b.center(bw_);
                block_img(g, &b, &[pal.muted], 4.0)
            });
            (x, y, (w * 0.5, h * 0.5), Shape::Image { src, w, h, draw, load, fit, radius, shadow: l.shadow.unwrap_or(true), ring: l.ring, found, source, missing })
        }
        T::Code => {
            let w = l.w.map(|v| v as f32 * bw).unwrap_or(bw * 0.8 - x.min(bw * 0.2));
            let code = build_code(g, pal, l.text.as_deref().unwrap_or(""), l.title.as_deref(), w, bh - y - 24.0 * u, l.size.map(|s| s as f32 * u), u);
            (x, y, (code.w * 0.5, code.h * 0.5), Shape::Code(Box::new(code)))
        }
        T::Counter => {
            let to = match l.to {
                Some(Pt::Num(v)) => v,
                _ => 0.0,
            };
            let from = match l.from {
                Some(Pt::Num(v)) => v,
                _ => 0.0,
            };
            let decimals = l.decimals.map(|d| d as usize).unwrap_or_else(|| (0..=2).find(|&d| [from, to].iter().all(|v| ((v * 10f64.powi(d as i32)) - (v * 10f64.powi(d as i32)).round()).abs() < 1e-6)).unwrap_or(2));
            let size_u = l.size.unwrap_or(160.0) as f32;
            let font = match l.weight {
                Some(w) => font_of(Some(w), size_u),
                None => FontId::Bold,
            };
            let st = Style::new(font, size_u * u).ls(if size_u >= 72.0 { -0.03 } else { 0.0 }).lh(1.1).tnum();
            let color = col(&l.color, pal.ink).unwrap_or(pal.ink);
            (x, y, (0.0, st.size * 0.55), Shape::Counter { from, to, decimals, commas: to.abs().max(from.abs()) >= 1000.0, prefix: l.prefix.clone().unwrap_or_default(), suffix: l.suffix.clone().unwrap_or_default(), st, align: l.align.unwrap_or(Align::Center), color })
        }
        T::Highlight => {
            let (w, h) = (px(l.w), py(l.h));
            let color = col(&l.color, pal.accent).unwrap_or(pal.accent);
            (x, y, (w * 0.5, h * 0.5), Shape::Highlight { w, h, color, mark: l.style.unwrap_or(Mark::Marker), radius: l.radius.map(|r| r as f32 * u).unwrap_or(8.0 * u) })
        }
    }
}

fn quad_len(a: (f32, f32), c: (f32, f32), b: (f32, f32)) -> f32 {
    let mut len = 0.0;
    let mut prev = a;
    for i in 1..=24 {
        let t = i as f32 / 24.0;
        let mt = 1.0 - t;
        let p = (mt * mt * a.0 + 2.0 * mt * t * c.0 + t * t * b.0, mt * mt * a.1 + 2.0 * mt * t * c.1 + t * t * b.1);
        len += ((p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2)).sqrt();
        prev = p;
    }
    len
}

#[allow(clippy::too_many_arguments)]
fn build_code(g: &Gfx, pal: &Pal, text: &str, title: Option<&str>, w: f32, max_h: f32, size: Option<f32>, u: f32) -> CodeL {
    let tx = g.text();
    let pad = 40.0 * u;
    let bar_h = 64.0 * u;
    let max_body = (max_h - bar_h - 2.0 * pad).max(120.0 * u);
    let mut sz = size.unwrap_or(32.0 * u);
    let (st, lines, adv) = loop {
        let st = Style::new(FontId::Mono, sz).lh(1.5);
        let adv = tx.width(st, "0");
        let cols = ((w - 2.0 * pad) / adv).floor().max(8.0) as usize;
        let lines = stage::wrap_code(text, cols);
        if lines.len().max(1) as f32 * st.lh * st.size <= max_body || sz <= 12.0 * u {
            break (st, lines, adv);
        }
        sz *= 0.94;
    };
    let line_h = st.lh * st.size;
    let h = (bar_h + lines.len().max(1) as f32 * line_h + 2.0 * pad).round();
    let title = title.filter(|t| !t.trim().is_empty()).map(|t| {
        let b = tx.fit(&tokens(t, false), Style::new(FontId::SemiBold, 26.0 * u), w - 240.0 * u, 1, bar_h, 16.0 * u);
        let img = block_img(g, &b, &[pal.muted], 2.0);
        ((w - b.w) * 0.5 - 2.0, (bar_h - b.h()) * 0.5 - 2.0, img)
    });
    let lines: Vec<CodeLine> = lines.iter().map(|l| (l.chars().count(), stage::code_segs(l))).collect();
    let total = lines.iter().map(|l| l.0 + 1).sum::<usize>();
    CodeL { w: w.round(), h, radius: 22.0 * u, bar_h, title: title.map(|(x, y, i)| (i, x, y)), st, adv, line_h, pad, lines, total }
}

// -------------------------------------------------------------------------------------- draw

/// Draw a scene beat into `rect` (the panel for `place: stage`, the frame for `place: frame`).
pub(crate) fn draw(g: &Gfx, pm: &mut Pixmap, rect: Rect, beat: &ResolvedBeat, t_ms: f64) {
    let BeatKind::Scene { place, bg, .. } = &beat.kind else { return };
    let geo = Geo::new(rect);
    let clip = IRect::round(rect).intersect(IRect::of(pm));
    if clip.is_empty() {
        return;
    }
    let pal = g.pal();
    match (place, bg) {
        (Place::Stage, _) => stage::draw_surface(g, pm, &geo, clip, &pal, t_ms),
        (Place::Frame, Bg::Canvas) => g.canvas(pm, t_ms),
        _ => {}
    }
    let ck = Clock::new(t_ms, beat.start_ms, beat.end_ms);
    if ck.lt < 0.0 || ck.lt > ck.dur {
        return;
    }
    let key = crate::hash_of(&("scene", stage::layout_key(beat, &geo)));
    let lay = g.inner().scenes.get_or(key, || build(g, &geo, beat));
    let (ox, oy) = (geo.x + lay.bx[0], geo.y + lay.bx[1]);
    if let Some(c) = lay.card {
        card(g, pm, &pal, geo.x + c[0], geo.y + c[1], c[2], c[3], 32.0 * lay.u, ck, clip, lay.u);
    }
    for l in &lay.layers {
        let s = state(l, ck, lay.u);
        if s.op <= 0.002 {
            continue;
        }
        draw_layer(g, pm, &pal, l, &s, ox, oy, ck, clip, lay.u);
    }
}

#[allow(clippy::too_many_arguments)]
fn card(g: &Gfx, pm: &mut Pixmap, pal: &Pal, x: f32, y: f32, w: f32, h: f32, r: f32, ck: Clock, clip: IRect, u: f32) {
    let p = ck.enter(0.0);
    let op = p * ck.exit();
    if op <= 0.0 {
        return;
    }
    let dy = ((1.0 - p) * 24.0 * u).round();
    let sh = g.shadow(w as u32, h as u32, r, 56.0 * u, -14.0 * u);
    let (bx, by) = (x.round() as i32, (y + dy + 28.0 * u).round() as i32);
    let skip = IRect::new(x.round() as i32 + sh.inner.x0, (y + dy).round() as i32 + sh.inner.y0, x.round() as i32 + sh.inner.x1, (y + dy).round() as i32 + sh.inner.y1);
    blit_mask_skip(pm, &sh.sprite, bx, by, pal.black, 0.55 * op, clip, Some(skip));
    fill_rrect(pm, x, y + dy, w, h, r, pal.card, op);
    stroke_rrect(pm, x + 0.5, y + dy + 0.5, w - 1.0, h - 1.0, r, (1.5 * u).max(1.0), pal.white, 0.08 * op);
}

fn ease_f(e: Ease, p: f32) -> f32 {
    match e {
        Ease::Out => ease_out(p),
        Ease::InOut => ease_in_out(p),
        Ease::Back => back_out(p),
        Ease::Linear => p.clamp(0.0, 1.0),
    }
}

fn state(l: &LayerL, ck: Clock, u: f32) -> St {
    let lt = ck.lt;
    let mut s = St { op: 1.0, dx: 0.0, dy: 0.0, s: 1.0, rot: 0.0, draw: 1.0, ax: l.ax, ay: l.ay };
    let e = &l.enter;
    if lt < e.at {
        s.op = 0.0;
        return s;
    }
    let pin = prog(lt, e.at, e.ms);
    let p = ease_f(l.ease, pin);
    let draws = e.draw && matches!(l.shape, Shape::Rect { .. } | Shape::Path { .. } | Shape::Highlight { .. });
    if draws {
        s.draw = p.clamp(0.0, 1.0);
    }
    let (rise, slide) = (RISE_U * u, SLIDE_U * u);
    match e.motion {
        Motion::Up => {
            s.dy += (1.0 - p) * rise;
            s.op = pin.clamp(0.0, 1.0).powf(0.6);
        }
        Motion::Down => {
            s.dy -= (1.0 - p) * rise;
            s.op = pin.powf(0.6);
        }
        Motion::Left => {
            s.dx -= (1.0 - p) * slide;
            s.op = pin.powf(0.6);
        }
        Motion::Right => {
            s.dx += (1.0 - p) * slide;
            s.op = pin.powf(0.6);
        }
        Motion::Fade => s.op = p.clamp(0.0, 1.0),
        Motion::Scale => {
            s.s *= lerp(0.86, 1.0, p);
            s.op = pin.powf(0.6);
        }
        Motion::Pop => {
            s.s *= lerp(0.4, 1.0, back_out(pin));
            s.op = (pin * 3.0).min(1.0);
        }
        Motion::None => s.op = 1.0,
    }
    if draws && e.motion == Motion::Fade {
        // strokes show as soon as they start drawing
        s.op = (pin * 6.0).min(1.0);
    }
    // keyframes, in order
    let (mut ks, mut ko, mut kr) = (1.0f32, 1.0f32, 0.0f32);
    for k in &l.keys {
        if lt <= k.t {
            continue;
        }
        let q = ease_f(l.ease, prog(lt, k.t, k.ms));
        if let Some(v) = k.x {
            s.ax = lerp(s.ax, v, q);
        }
        if let Some(v) = k.y {
            s.ay = lerp(s.ay, v, q);
        }
        if let Some(v) = k.scale {
            ks = lerp(ks, v, q);
        }
        if let Some(v) = k.opacity {
            ko = lerp(ko, v, q);
        }
        if let Some(v) = k.rotate {
            kr = lerp(kr, v, q);
        }
    }
    // exit
    if let Some(x) = &l.exit {
        if lt >= x.at {
            let q = ease_in_out(prog(lt, x.at, x.ms));
            match x.motion {
                Motion::Up => s.dy -= q * rise,
                Motion::Down => s.dy += q * rise,
                Motion::Left => s.dx -= q * slide,
                Motion::Right => s.dx += q * slide,
                Motion::Scale => s.s *= lerp(1.0, 0.86, q),
                Motion::Pop => s.s *= lerp(1.0, 1.12, q),
                Motion::Fade | Motion::None => {}
            }
            s.op *= if x.motion == Motion::None { 0.0 } else { 1.0 - q };
        }
    }
    s.op *= ck.exit() * l.opacity * ko.clamp(0.0, 1.0);
    s.s *= ks.max(0.01);
    s.rot = kr;
    s
}

#[allow(clippy::too_many_arguments)]
fn draw_layer(g: &Gfx, pm: &mut Pixmap, pal: &Pal, l: &LayerL, s: &St, ox: f32, oy: f32, ck: Clock, clip: IRect, u: f32) {
    let settled = (s.s - 1.0).abs() < 1e-3 && s.rot.abs() < 1e-3;
    let (ax, ay) = (ox + s.ax + s.dx, oy + s.ay + s.dy);
    // transform for vector shapes: anchor, then rotate + scale about the pivot
    let shape_tr = |pivot: (f32, f32)| -> Transform {
        let (px, py) = pivot;
        Transform::from_translate(ax + px, ay + py).pre_rotate(s.rot).pre_scale(s.s, s.s).pre_translate(-px, -py)
    };
    match &l.shape {
        Shape::Text { block, off, colors, img, pad, marks, mark } => {
            let (x, y) = (ax + off.0, ay + off.1);
            let (cx, cy) = (ax + l.pivot.0, ay + l.pivot.1);
            if let Some(m) = mark {
                // the decoration sweeps in once the text has landed
                let p = ease_out(prog(ck.lt, l.enter.at + l.enter.ms + 120.0, 520.0));
                if p > 0.0 {
                    let (col, a) = match m {
                        Mark::Marker => (pal.accent, 0.34),
                        Mark::Underline => (pal.accent, 0.95),
                    };
                    for r in marks {
                        let rx = cx + (x + r[0] - cx) * s.s;
                        let ry = cy + (y + r[1] - cy) * s.s;
                        let (rw, rh) = (r[2] * s.s * p, r[3] * s.s);
                        fill_rrect(pm, rx, ry, rw, rh, (rh * 0.22).min(10.0 * u), col, a * s.op);
                    }
                }
            }
            if settled {
                g.text().draw(pm, block, x.round(), y.round(), colors, s.op, clip);
            } else {
                let (w, h) = (img.w as f32 * s.s, img.h as f32 * s.s);
                let _ = pad;
                draw_img(pm, img, [cx - w * 0.5, cy - h * 0.5, w, h], clip, s.op, None, None);
            }
        }
        Shape::Rect { w, h, radius, circle, fill, stroke, sw, hair, shadow, label, sprite } => {
            let (w, h) = (*w, *h);
            // geometry local to the anchor; circles are anchored at their centre
            let (lx, ly) = if *circle { (-w * 0.5, -h * 0.5) } else { (0.0, 0.0) };
            let fill_a = if s.draw < 1.0 { ((s.draw - 0.45) / 0.55).clamp(0.0, 1.0) } else { 1.0 };
            if *shadow && fill.is_some() && s.rot.abs() < 1e-3 && fill_a > 0.0 {
                let sh = g.shadow(w.max(1.0) as u32, h.max(1.0) as u32, *radius, 40.0 * u, -10.0 * u);
                let (x0, y0) = ((ax + lx).round() as i32, (ay + ly).round() as i32);
                // an opaque, settled fill covers the middle of its own shadow
                let skip = (settled && s.op * fill_a >= 0.999).then(|| IRect::new(x0 + sh.inner.x0, y0 + sh.inner.y0, x0 + sh.inner.x1, y0 + sh.inner.y1));
                blit_mask_skip(pm, &sh.sprite, x0, y0 + (22.0 * u).round() as i32, pal.black, 0.45 * s.op * fill_a, clip, skip);
            }
            if settled && s.draw >= 1.0 && sprite.w > 0 {
                let m = (sw * 0.5).ceil() + 1.0;
                let (x, y) = ((ax + lx - m).round(), (ay + ly - m).round());
                draw_img(pm, sprite, [x, y, sprite.w as f32, sprite.h as f32], clip, s.op, None, None);
                return;
            }
            let tr = shape_tr(if *circle { (0.0, 0.0) } else { l.pivot });
            let path = if *circle { PathBuilder::from_circle(0.0, 0.0, w * 0.5) } else { rrect(lx, ly, w, h, *radius) };
            let Some(path) = path else { return };
            if let Some(f) = fill {
                if fill_a > 0.0 {
                    pm.fill_path(&path, &raster::paint(*f, s.op * fill_a), FillRule::Winding, tr, None);
                }
            }
            match stroke {
                Some(c) => {
                    let len = if *circle { std::f32::consts::TAU * w * 0.5 } else { raster::rrect_len(w, h, *radius) };
                    stroke_partial(pm, &path, len, s.draw, *sw, *c, s.op, tr);
                }
                None if *hair && fill_a > 0.0 => {
                    let st = Stroke { width: (1.5 * u).max(1.0) / s.s.max(0.1), ..Stroke::default() };
                    pm.stroke_path(&path, &raster::paint(pal.white, 0.08 * s.op * fill_a), &st, tr, None);
                }
                None => {}
            }
            if let Some(img) = label {
                let (cx, cy) = if *circle { (ax, ay) } else { (ax + w * 0.5, ay + h * 0.5) };
                let (iw, ih) = (img.w as f32 * s.s, img.h as f32 * s.s);
                let (x, y) = (cx - iw * 0.5, cy - ih * 0.5);
                let (x, y) = if settled { (x.round(), y.round()) } else { (x, y) };
                draw_img(pm, img, [x, y, iw, ih], clip, s.op * fill_a.max(if fill.is_none() { s.draw } else { 0.0 }), None, None);
            }
        }
        Shape::Path { path, len, color, width, dash, head } => {
            let tr = shape_tr(l.pivot);
            if *dash {
                let mut st = Stroke { width: *width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
                st.dash = StrokeDash::new(vec![*width * 0.01, *width * 2.6], 0.0);
                pm.stroke_path(path, &raster::paint(*color, s.op * s.draw.min(1.0)), &st, tr, None);
            } else {
                stroke_partial(pm, path, *len, s.draw, *width, *color, s.op, tr);
            }
            if let Some((tip, dir, hl)) = head {
                let hp = back_out(((s.draw - 0.82) / 0.18).clamp(0.0, 1.0));
                if hp > 0.0 {
                    let hl = hl * hp;
                    let hw = hl * 0.62;
                    let (bx, by) = (tip.0 - dir.0 * hl, tip.1 - dir.1 * hl);
                    let (nx, ny) = (-dir.1, dir.0);
                    let mut pb = PathBuilder::new();
                    pb.move_to(tip.0, tip.1);
                    pb.line_to(bx + nx * hw, by + ny * hw);
                    pb.quad_to(tip.0 - dir.0 * hl * 0.72, tip.1 - dir.1 * hl * 0.72, bx - nx * hw, by - ny * hw);
                    pb.close();
                    if let Some(hp) = pb.finish() {
                        pm.fill_path(&hp, &raster::paint(*color, s.op), FillRule::Winding, tr, None);
                        let st = Stroke { width: (width * 0.5).max(1.0), line_join: LineJoin::Round, ..Stroke::default() };
                        pm.stroke_path(&hp, &raster::paint(*color, s.op), &st, tr, None);
                    }
                }
            }
        }
        Shape::Image { src, w, h, draw, load, fit, radius, shadow, ring, found, source, missing } => {
            let (cx, cy) = (ax + w * 0.5, ay + h * 0.5);
            // the visible rect: the drawn image (contain) or the layer frame (cover)
            let vis = match fit {
                ImgFit::Contain => [draw[0], draw[1], draw[2], draw[3]],
                ImgFit::Cover => [0.0, 0.0, *w, *h],
            };
            let sc = |r: [f32; 4]| -> [f32; 4] {
                let (x0, y0) = (ax + r[0] - cx, ay + r[1] - cy);
                [cx + x0 * s.s, cy + y0 * s.s, r[2] * s.s, r[3] * s.s]
            };
            let mut v = sc(vis);
            let mut d = sc(*draw);
            if settled {
                for r in [&mut v, &mut d] {
                    r[0] = r[0].round();
                    r[1] = r[1].round();
                }
            }
            let rad = *radius * s.s;
            if *shadow {
                let sh = g.shadow(vis[2].max(1.0) as u32, vis[3].max(1.0) as u32, *radius, 48.0 * u, -12.0 * u);
                blit_mask_skip(pm, &sh.sprite, (ax + vis[0]).round() as i32, (ay + vis[1] + 26.0 * u).round() as i32, pal.black, 0.55 * s.op, clip, None);
            }
            let round = Round::new(v[0], v[1], v[2], v[3], rad);
            // ring + spotlight on a region of the image, following the push-in
            let hp = ease_out(prog(ck.lt, l.enter.at + 600.0, 450.0));
            let ring_r = ring.filter(|_| hp > 0.0 && *found).map(|r| {
                let pad = 10.0 * u * s.s;
                let m = 6.0 * u;
                let x0 = (d[0] + r[0] as f32 * d[2] - pad).max(v[0] + m);
                let y0 = (d[1] + r[1] as f32 * d[3] - pad).max(v[1] + m);
                let x1 = (d[0] + (r[0] + r[2]) as f32 * d[2] + pad).min(v[0] + v[2] - m);
                let y1 = (d[1] + (r[1] + r[3]) as f32 * d[3] + pad).min(v[1] + v[3] - m);
                [x0, y0, (x1 - x0).max(4.0 * u), (y1 - y0).max(4.0 * u)]
            });
            let spot = ring_r.map(|r| Spot { area: Round::new(r[0], r[1], r[2], r[3], 14.0 * u), keep: (256.0 * (1.0 - 0.38 * hp)) as u32 });
            if *found {
                let im = g.image(src, load.0, load.1);
                match im.as_ref() {
                    Some(img) => {
                        if !img.is_opaque() {
                            fill_rrect(pm, v[0], v[1], v[2], v[3], rad, pal.card2, s.op);
                        }
                        draw_img(pm, img, d, clip, s.op, Some(round), spot);
                    }
                    None => fill_rrect(pm, v[0], v[1], v[2], v[3], rad, pal.card2, s.op),
                }
            } else {
                fill_rrect(pm, v[0], v[1], v[2], v[3], rad, pal.card2, s.op);
                if let Some(m) = missing {
                    draw_img(pm, m, [(v[0] + (v[2] - m.w as f32) * 0.5).round(), (v[1] + (v[3] - m.h as f32) * 0.5).round(), m.w as f32, m.h as f32], clip, s.op, None, None);
                }
            }
            stroke_rrect(pm, v[0] + 0.5, v[1] + 0.5, v[2] - 1.0, v[3] - 1.0, rad, (1.5 * u).max(1.0), pal.white, 0.10 * s.op);
            if let Some(r) = ring_r {
                if let Some(path) = rrect(r[0], r[1], r[2], r[3], 14.0 * u) {
                    let len = raster::rrect_len(r[2], r[3], 14.0 * u);
                    stroke_partial(pm, &path, len, hp, 12.0 * u, pal.accent, 0.22 * s.op, Transform::identity());
                    stroke_partial(pm, &path, len, hp, 5.0 * u, pal.accent, s.op, Transform::identity());
                }
            }
            if let Some((img, pw, ph)) = source {
                let a = prog(ck.lt, l.enter.at + 380.0, 160.0) * s.op;
                if a > 0.0 {
                    let (x, y) = ((v[0] + 22.0 * u).round(), (v[1] + v[3] - 22.0 * u - ph).round());
                    draw_img(pm, img, [x, y, *pw, *ph], clip, a, None, None);
                }
            }
        }
        Shape::Code(c) => draw_code(g, pm, pal, c, l, s, ax, ay, ck, clip, u),
        Shape::Counter { from, to, decimals, commas, prefix, suffix, st, align, color } => {
            let k = ease_out(prog(ck.lt, l.enter.at + 120.0, COUNT_MS)) as f64;
            let v = from + (to - from) * k;
            let num = fmt_num(v, *decimals, *commas);
            let tx = g.text();
            let st = if settled { *st } else { st.sized(st.size * s.s) };
            let gapw = st.ls.abs() * st.size;
            let (wp, wn, ws) = (tx.width(st, prefix), tx.width(st, &num), tx.width(st, suffix));
            let total = wp + wn + ws + if prefix.is_empty() { 0.0 } else { gapw } + if suffix.is_empty() { 0.0 } else { gapw };
            let mut x = match align {
                Align::Left => ax,
                Align::Center => ax - total * 0.5,
                Align::Right => ax - total,
            };
            // baseline of the unscaled line, then scaled about the line's middle
            let st0 = if settled { st } else { st.sized(st.size / s.s) };
            let base0 = ay + (st0.lh * st0.size - (tx.ascent(st0) + tx.descent(st0))) * 0.5 + tx.ascent(st0);
            let cy = ay + l.pivot.1;
            let base = cy + (base0 - cy) * s.s;
            tx.draw_line(pm, st, prefix, x, base, pal.accent, s.op, clip);
            if !prefix.is_empty() {
                x += wp + gapw;
            }
            tx.draw_line(pm, st, &num, x, base, *color, s.op, clip);
            x += wn + gapw;
            tx.draw_line(pm, st, suffix, x, base, pal.accent, s.op, clip);
        }
        Shape::Highlight { w, h, color, mark, radius } => {
            let p = s.draw;
            match mark {
                Mark::Marker => fill_rrect(pm, ax, ay, w * p, *h, radius.min(h * 0.5), *color, 0.30 * s.op),
                Mark::Underline => {
                    let bh = (h * 0.14).max(6.0 * u);
                    fill_rrect(pm, ax, ay + h - bh, w * p, bh, bh * 0.5, *color, 0.95 * s.op);
                }
            }
        }
    }
}

fn fmt_num(v: f64, decimals: usize, commas: bool) -> String {
    let neg = v < 0.0;
    let s = format!("{:.*}", decimals, v.abs());
    let (int, frac) = match s.split_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (s.clone(), None),
    };
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    for (k, c) in int.chars().enumerate() {
        if commas && k > 0 && (int.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&f);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn draw_code(g: &Gfx, pm: &mut Pixmap, pal: &Pal, c: &CodeL, l: &LayerL, s: &St, ax: f32, ay: f32, ck: Clock, clip: IRect, u: f32) {
    let tx = g.text();
    let (x, y) = (ax.round(), ay.round());
    let (w, h) = (c.w, c.h);
    let sh = g.shadow(w as u32, h as u32, c.radius, 56.0 * u, -14.0 * u);
    blit_mask_skip(pm, &sh.sprite, x as i32, (y + 30.0 * u).round() as i32, pal.black, 0.6 * s.op, clip, None);
    let body = mix(pal.canvas, pal.card, 0.55);
    fill_rrect(pm, x, y, w, h, c.radius, body, s.op);
    if let Some(path) = top_rounded(x, y, w, c.bar_h, c.radius) {
        pm.fill_path(&path, &raster::paint(pal.card2, s.op), FillRule::Winding, Transform::identity(), None);
    }
    raster::fill_rect(pm, IRect::new(x as i32, (y + c.bar_h).round() as i32, (x + w) as i32, (y + c.bar_h).round() as i32 + (1.5 * u).round().max(1.0) as i32).intersect(clip), mix(body, pal.line, s.op));
    stroke_rrect(pm, x + 0.5, y + 0.5, w - 1.0, h - 1.0, c.radius, (1.5 * u).max(1.0), pal.white, 0.08 * s.op);
    for (i, col) in [pal.red, pal.gold, pal.green].iter().enumerate() {
        fill_circle(pm, x + 34.0 * u + i as f32 * 26.0 * u, y + c.bar_h * 0.5, 7.5 * u, *col, s.op * 0.9);
    }
    if let Some((img, tx_, ty_)) = &c.title {
        draw_img(pm, img, [(x + tx_).round(), (y + ty_).round(), img.w as f32, img.h as f32], clip, s.op, None, None);
    }
    // typing: 28 chars/s from just after the entrance, done before the beat ends
    let t0 = l.enter.at + 300.0;
    let done = (c.total as f64 / 28.0) * 1000.0;
    let t1 = (ck.dur - 700.0).min(t0 + done).max(t0 + 300.0);
    let typed = (prog(ck.lt, t0, t1 - t0) as f64 * c.total as f64).round() as usize;
    let mut left = typed;
    let text_x = x + c.pad;
    let text_y = y + c.bar_h + c.pad;
    let colors = [pal.ink, pal.accent, pal.muted];
    let mut cursor = (text_x, 0usize);
    for (i, (chars, segs)) in c.lines.iter().enumerate() {
        let base = text_y + i as f32 * c.line_h + (c.line_h - c.st.size * 1.21) * 0.5 + tx.ascent(c.st);
        let show = left.min(*chars);
        if show > 0 {
            let limit = (text_x + show as f32 * c.adv).round() as i32;
            let cl = IRect { x1: limit.min(clip.x1), ..clip };
            for (col, seg, tag) in segs {
                if *col >= show {
                    break;
                }
                tx.draw_line(pm, c.st, seg, text_x + *col as f32 * c.adv, base, colors[*tag as usize], s.op, cl);
            }
        }
        cursor = (text_x + show as f32 * c.adv, i);
        if left <= *chars {
            break;
        }
        left -= chars + 1;
    }
    let blink = if typed >= c.total { ((ck.lt / 530.0) as i64 % 2 == 0) as u8 as f32 } else { 1.0 };
    if ck.lt >= t0 && blink > 0.0 {
        let cy0 = text_y + cursor.1 as f32 * c.line_h + (c.line_h - c.st.size * 1.15) * 0.5;
        fill_rrect(pm, cursor.0 + 2.0 * u, cy0, c.adv * 0.6, c.st.size * 1.15, 2.0 * u, pal.accent, s.op * 0.9);
    }
}

/// A rect whose top corners are rounded (title bars).
fn top_rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
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

    #[test]
    fn numbers_format() {
        assert_eq!(fmt_num(10075.0, 0, true), "10,075");
        assert_eq!(fmt_num(1234567.891, 2, true), "1,234,567.89");
        assert_eq!(fmt_num(42.0, 0, false), "42");
        assert_eq!(fmt_num(-1500.0, 0, true), "-1,500");
    }

    #[test]
    fn attach_edges() {
        let r = Attach::Rect([0.0, 0.0, 100.0, 50.0]);
        let (x, y) = r.edge((300.0, 25.0), 0.0);
        assert!((x - 100.0).abs() < 1e-3 && (y - 25.0).abs() < 1e-3, "{x} {y}");
        let c = Attach::Circle(0.0, 0.0, 10.0);
        let (x, y) = c.edge((0.0, 100.0), 5.0);
        assert!(x.abs() < 1e-3 && (y - 15.0).abs() < 1e-3);
    }
}
