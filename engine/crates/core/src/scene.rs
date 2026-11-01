//! `scene` beats: a custom animation made of layers (text, boxes, circles, lines, arrows,
//! images, code, counters, highlights), each with its own entrance, exit and keyframes, all
//! anchored to words. cutroom-gfx draws them natively, so a bespoke animation is ~20-60 lines
//! of YAML instead of a hand-written video.
//!
//! ```yaml
//! - at: "connect Claude to Poppy"
//!   until: w1290
//!   kind: scene
//!   layers:
//!     - {type: text, text: "Claude *talks to* Poppy", x: 0.5, y: 0.12, size: 96, weight: bold, align: center}
//!     - {id: a, type: box, x: 0.08, y: 0.4, w: 0.32, h: 0.26, label: Claude, in: {at: "Claude", from: left}}
//!     - {id: b, type: box, x: 0.6, y: 0.4, w: 0.32, h: 0.26, label: Poppy, in: {at: "Poppy", from: right}}
//!     - {type: arrow, from: a, to: b, in: {at: "talks"}}
//! ```
//!
//! Geometry is normalized to the scene's box (the panel for `place: stage`, the whole frame for
//! `place: frame`, the inset card with `bg: card`): `x`/`y` 0..1 from its top-left. Sizes
//! (`size`, `width`, `radius`) are in units where the box is 1920 wide.
//!
//! Resolved layer times travel in `ResolvedBeat::items_ms`, flattened per layer as
//! `[in, out, key times...]` (NaN when not given); [`unflatten`] reads them back.

use crate::resolve::Severity;
use crate::spec::{Anchor, PointN, RectN};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    /// In the panel slot, on the raised panel surface, like every other stage beat.
    #[default]
    Stage,
    /// The whole frame, over the canvas; the camera tile and badge stay on top.
    Frame,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Bg {
    /// Stage: the panel surface. Frame: the canvas as it is.
    #[default]
    None,
    /// An inset card; layer geometry is relative to the card.
    Card,
    /// Repaint the dot-grid canvas under the layers.
    Canvas,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LayerType {
    Text,
    Box,
    Circle,
    Line,
    Arrow,
    Image,
    Code,
    Counter,
    Highlight,
}

impl LayerType {
    pub fn name(self) -> &'static str {
        match self {
            LayerType::Text => "text",
            LayerType::Box => "box",
            LayerType::Circle => "circle",
            LayerType::Line => "line",
            LayerType::Arrow => "arrow",
            LayerType::Image => "image",
            LayerType::Code => "code",
            LayerType::Counter => "counter",
            LayerType::Highlight => "highlight",
        }
    }

    /// Strokes that draw on by default.
    pub fn draws(self) -> bool {
        matches!(self, LayerType::Line | LayerType::Arrow | LayerType::Highlight)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Weight {
    Regular,
    Semibold,
    Bold,
    Mono,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ImgFit {
    Contain,
    Cover,
}

/// How a layer enters (`in.from`) or leaves (`out.to`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    Up,
    Down,
    Left,
    Right,
    Fade,
    Scale,
    Pop,
    None,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    Out,
    InOut,
    Back,
    Linear,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Marker,
    Underline,
}

/// A layer time: seconds after the beat starts (`1.5`, `"+0.4s"`, `"+400ms"`), or any anchor
/// (`w123`, `"a phrase"`, `w123+0.2s`, `s:754.2`, `o:61.5`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Time {
    Secs(f64),
    Text(String),
}

/// `from` / `to`: a point `[x, y]` (arrows, lines), the id of a box / circle / image layer
/// (arrows attach to its edge), or a number (counters).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Pt {
    Num(f64),
    Xy(PointN),
    Id(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Anim {
    /// When (default: the beat start, staggered by layer order).
    #[serde(default)]
    pub at: Option<Time>,
    /// Entrance motion (`in` only).
    #[serde(default)]
    pub from: Option<Motion>,
    /// Exit motion (`out` only).
    #[serde(default)]
    pub to: Option<Motion>,
    /// Duration in ms.
    #[serde(default)]
    pub ms: Option<f64>,
    /// Draw strokes on (boxes, circles, lines, arrows, highlights).
    #[serde(default)]
    pub draw: Option<bool>,
}

/// From `t`, animate to these values over `ms` (default 500).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub t: Time,
    #[serde(default)]
    pub ms: Option<f64>,
    /// New anchor position (box-normalized).
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub scale: Option<f64>,
    #[serde(default)]
    pub opacity: Option<f64>,
    /// Degrees (shapes only).
    #[serde(default)]
    pub rotate: Option<f64>,
}

/// One layer. Every type shares this flat field set; `cutroom check` says which fields a type
/// needs and which it ignores.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    #[serde(rename = "type")]
    pub ty: LayerType,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub w: Option<f64>,
    #[serde(default)]
    pub h: Option<f64>,
    /// Circle radius, as a fraction of the box height.
    #[serde(default)]
    pub r: Option<f64>,
    #[serde(default)]
    pub from: Option<Pt>,
    #[serde(default)]
    pub to: Option<Pt>,
    /// Line vertices.
    #[serde(default)]
    pub points: Option<Vec<PointN>>,
    #[serde(default)]
    pub text: Option<String>,
    /// Font size (units; the box is 1920 units wide).
    #[serde(default)]
    pub size: Option<f64>,
    #[serde(default)]
    pub weight: Option<Weight>,
    #[serde(default)]
    pub align: Option<Align>,
    /// Max lines before the text shrinks.
    #[serde(default)]
    pub lines: Option<usize>,
    /// Text / stroke / marker colour: a theme name or hex.
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub stroke: Option<String>,
    /// Stroke width (units).
    #[serde(default)]
    pub width: Option<f64>,
    /// Corner radius (units).
    #[serde(default)]
    pub radius: Option<f64>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub fit: Option<ImgFit>,
    /// Source pill on an image ("stripe.com").
    #[serde(default)]
    pub source: Option<String>,
    /// Image: ring and spotlight a region of the image (image-normalized `[x, y, w, h]`); it
    /// follows push-ins and draws on just after the image lands.
    #[serde(default)]
    pub ring: Option<RectN>,
    /// Code card title.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub decimals: Option<u32>,
    /// Arrow bend: sideways offset of the middle, as a fraction of its length (-0.5..0.5).
    #[serde(default)]
    pub curve: Option<f64>,
    #[serde(default)]
    pub dash: Option<bool>,
    /// Highlight style; on text, a marker or underline behind the `*accent*` words.
    #[serde(default)]
    pub style: Option<Mark>,
    #[serde(default)]
    pub opacity: Option<f64>,
    #[serde(default)]
    pub shadow: Option<bool>,
    #[serde(default, rename = "in")]
    pub enter: Option<Anim>,
    #[serde(default, rename = "out")]
    pub exit: Option<Anim>,
    #[serde(default)]
    pub keys: Vec<Key>,
    #[serde(default)]
    pub ease: Option<Ease>,
}

/// Resolved times (absolute output ms) of one layer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerTimes {
    pub enter: Option<f64>,
    pub exit: Option<f64>,
    pub keys: Vec<f64>,
}

/// Number of `items_ms` slots the layers use.
pub fn times_len(layers: &[Layer]) -> usize {
    layers.iter().map(|l| 2 + l.keys.len()).sum()
}

/// Read the flattened `items_ms` back into per-layer times (missing entries become None).
pub fn unflatten(layers: &[Layer], flat: &[f64]) -> Vec<LayerTimes> {
    let get = |i: usize| flat.get(i).copied().filter(|v| v.is_finite());
    let mut i = 0;
    layers
        .iter()
        .map(|l| {
            let t = LayerTimes { enter: get(i), exit: get(i + 1), keys: (0..l.keys.len()).map(|k| get(i + 2 + k).unwrap_or(f64::NAN)).collect() };
            i += 2 + l.keys.len();
            t
        })
        .collect()
}

/// Resolve every layer time to output ms. `anchor(a)` gives the output time an anchor lands at
/// (already lead-adjusted for words). Returns the flattened times and problems (layer label,
/// severity, message).
pub fn resolve_times(layers: &[Layer], start: f64, end: f64, anchor: &dyn Fn(&Anchor) -> Result<f64>) -> (Vec<f64>, Vec<(Severity, String)>) {
    let mut out = Vec::with_capacity(times_len(layers));
    let mut probs = vec![];
    let frame = 1000.0 / 60.0;
    for (i, l) in layers.iter().enumerate() {
        let name = label(i, l);
        let mut one = |t: Option<&Time>, what: &str| -> f64 {
            let Some(t) = t else { return f64::NAN };
            let v = match t {
                Time::Secs(s) => Ok(start + s * 1000.0),
                Time::Text(s) => {
                    let s = s.trim();
                    if let Some(rel) = s.strip_prefix('+') {
                        crate::spec::parse_dur_ms(rel).map(|ms| start + ms)
                    } else {
                        anchor(&Anchor(s.to_string()))
                    }
                }
            };
            match v {
                Ok(v) => {
                    if v < start - frame || v > end + frame {
                        probs.push((Severity::Warn, format!("{name} {what} lands {} the beat window; it is clamped to the {}", if v < start { "before" } else { "after" }, if v < start { "start" } else { "end" })));
                    }
                    v.clamp(start, end)
                }
                Err(e) => {
                    probs.push((Severity::Error, format!("{name} {what}: {e}")));
                    f64::NAN
                }
            }
        };
        out.push(one(l.enter.as_ref().and_then(|a| a.at.as_ref()), "in.at"));
        out.push(one(l.exit.as_ref().and_then(|a| a.at.as_ref()), "out.at"));
        for (k, key) in l.keys.iter().enumerate() {
            out.push(one(Some(&key.t), &format!("keys[{k}].t")));
        }
    }
    (out, probs)
}

/// "layer 3 (id a, box)".
pub fn label(i: usize, l: &Layer) -> String {
    match &l.id {
        Some(id) => format!("layer {i} ({} {id:?})", l.ty.name()),
        None => format!("layer {i} ({})", l.ty.name()),
    }
}

/// Colour names the renderer knows (besides hex).
pub const COLORS: &[&str] = &["accent", "ink", "muted", "faint", "card", "card2", "surface", "canvas", "line", "green", "red", "gold", "white", "black", "accent_soft", "ink_soft"];

fn color_ok(c: &str) -> bool {
    let c = c.trim();
    if COLORS.contains(&c.to_ascii_lowercase().as_str()) || c.eq_ignore_ascii_case("none") {
        return true;
    }
    let h = c.trim_start_matches('#');
    (h.len() == 6 || h.len() == 3) && h.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Static checks of a scene's layers (geometry, fields, colours, files, ids). `camera`: the
/// camera tile (frame-normalized) when frame scenes should keep clear of it.
pub fn lint(layers: &[Layer], place: Place, spec_dir: &Path, camera: Option<RectN>) -> Vec<(Severity, String)> {
    use LayerType as T;
    let mut out: Vec<(Severity, String)> = vec![];
    macro_rules! err {
        ($s:expr, $m:expr) => {
            out.push(($s, $m))
        };
    }
    if layers.is_empty() {
        err!(Severity::Error, "scene has no layers".into());
    }
    if layers.len() > 40 {
        err!(Severity::Warn, format!("scene has {} layers; more than ~12 rarely reads on screen", layers.len()));
    }
    let mut ids: BTreeMap<&str, LayerType> = BTreeMap::new();
    for (i, l) in layers.iter().enumerate() {
        if let Some(id) = &l.id {
            if ids.insert(id.as_str(), l.ty).is_some() {
                err!(Severity::Error, format!("{}: id {id:?} is used twice", label(i, l)));
            }
        }
    }
    for (i, l) in layers.iter().enumerate() {
        let name = label(i, l);
        macro_rules! need {
            ($f:expr, $what:expr) => {
                if !$f {
                    out.push((Severity::Error, format!("{name} needs `{}`", $what)));
                }
            };
        }
        // required fields
        match l.ty {
            T::Text => {
                need!(l.text.as_ref().is_some_and(|t| !t.trim().is_empty()), "text");
                need!(l.x.is_some() && l.y.is_some(), "x and y");
            }
            T::Box | T::Highlight => need!(l.x.is_some() && l.y.is_some() && l.w.is_some() && l.h.is_some(), "x, y, w and h"),
            T::Image => {
                need!(l.src.is_some(), "src");
                need!(l.x.is_some() && l.y.is_some() && l.w.is_some() && l.h.is_some(), "x, y, w and h");
            }
            T::Circle => need!(l.x.is_some() && l.y.is_some() && l.r.is_some(), "x, y and r"),
            T::Line => need!(l.points.as_ref().is_some_and(|p| p.len() >= 2) || (l.from.is_some() && l.to.is_some()), "points (2+) or from and to"),
            T::Arrow => need!(l.from.is_some() && l.to.is_some(), "from and to"),
            T::Code => {
                need!(l.text.as_ref().is_some_and(|t| !t.trim().is_empty()), "text");
                need!(l.x.is_some() && l.y.is_some(), "x and y");
            }
            T::Counter => {
                need!(matches!(l.to, Some(Pt::Num(_))), "to (a number)");
                need!(l.x.is_some() && l.y.is_some(), "x and y");
            }
        }
        // fields this type ignores
        let used: &[&str] = match l.ty {
            T::Text => &["x", "y", "w", "text", "size", "weight", "align", "lines", "color", "style"],
            T::Box => &["x", "y", "w", "h", "fill", "stroke", "width", "radius", "label", "sub", "size", "color", "shadow"],
            T::Circle => &["x", "y", "r", "fill", "stroke", "width", "label", "size", "color", "shadow"],
            T::Line => &["points", "from", "to", "color", "width", "dash"],
            T::Arrow => &["from", "to", "color", "width", "curve", "dash"],
            T::Image => &["x", "y", "w", "h", "src", "fit", "radius", "source", "shadow", "ring"],
            T::Code => &["x", "y", "w", "text", "size", "title"],
            T::Counter => &["x", "y", "from", "to", "prefix", "suffix", "decimals", "size", "weight", "align", "color"],
            T::Highlight => &["x", "y", "w", "h", "color", "style", "radius"],
        };
        let set: [(&str, bool); 31] = [
            ("x", l.x.is_some()),
            ("y", l.y.is_some()),
            ("w", l.w.is_some()),
            ("h", l.h.is_some()),
            ("r", l.r.is_some()),
            ("from", l.from.is_some()),
            ("to", l.to.is_some()),
            ("points", l.points.is_some()),
            ("text", l.text.is_some()),
            ("size", l.size.is_some()),
            ("weight", l.weight.is_some()),
            ("align", l.align.is_some()),
            ("lines", l.lines.is_some()),
            ("color", l.color.is_some()),
            ("fill", l.fill.is_some()),
            ("stroke", l.stroke.is_some()),
            ("width", l.width.is_some()),
            ("radius", l.radius.is_some()),
            ("label", l.label.is_some()),
            ("sub", l.sub.is_some()),
            ("src", l.src.is_some()),
            ("fit", l.fit.is_some()),
            ("source", l.source.is_some()),
            ("title", l.title.is_some()),
            ("prefix", l.prefix.is_some()),
            ("suffix", l.suffix.is_some()),
            ("decimals", l.decimals.is_some()),
            ("curve", l.curve.is_some()),
            ("dash", l.dash.is_some()),
            ("style", l.style.is_some()),
            ("ring", l.ring.is_some()),
        ];
        for (f, on) in set {
            if on && !used.contains(&f) {
                err!(Severity::Warn, format!("{name}: `{f}` is ignored on {} layers", l.ty.name()));
            }
        }
        if let Some(r) = l.ring {
            if r[2] <= 0.0 || r[3] <= 0.0 || r[0] < -0.01 || r[1] < -0.01 || r[0] + r[2] > 1.01 || r[1] + r[3] > 1.01 {
                err!(Severity::Error, format!("{name}: ring {r:?} must be [x, y, w, h] inside 0..1 of the image"));
            }
        }
        if l.style.is_some() && l.ty == T::Text && !l.text.as_deref().unwrap_or("").contains('*') {
            err!(Severity::Warn, format!("{name}: `style` marks the *accent* words, and this text has none"));
        }
        if l.ty == T::Counter {
            // the number is drawn between them: a digit here shows twice ("250+ 250")
            for (f, v) in [("prefix", &l.prefix), ("suffix", &l.suffix)] {
                if let Some(v) = v.as_deref().filter(|v| v.chars().any(|c| c.is_ascii_digit())) {
                    err!(Severity::Error, format!("{name}: counter `{f}` {v:?} has a digit; the number goes in `to`, and {f} is a symbol or unit only ($, +, %, x, k, wpm)"));
                }
            }
        }
        if l.sub.is_some() && l.label.is_none() && l.ty == T::Box {
            err!(Severity::Warn, format!("{name}: `sub` shows only under a `label`"));
        }
        // numbers
        let nums = [("x", l.x), ("y", l.y), ("w", l.w), ("h", l.h), ("r", l.r), ("size", l.size), ("width", l.width), ("radius", l.radius), ("curve", l.curve), ("opacity", l.opacity)];
        for (f, v) in nums {
            if let Some(v) = v {
                if !v.is_finite() {
                    err!(Severity::Error, format!("{name}: `{f}` is not a number"));
                }
            }
        }
        for (f, v) in [("w", l.w), ("h", l.h), ("r", l.r), ("size", l.size), ("width", l.width)] {
            if let Some(v) = v {
                if v <= 0.0 {
                    err!(Severity::Error, format!("{name}: `{f}` must be > 0 (got {v})"));
                }
            }
        }
        if let Some(s) = l.size {
            if s > 0.0 && !(14.0..=420.0).contains(&s) {
                err!(Severity::Warn, format!("{name}: size {s} is outside 14..420 units (body text reads at 36+, labels 28+)"));
            } else if s > 0.0 && s < 28.0 && matches!(l.ty, T::Text | T::Counter) {
                err!(Severity::Warn, format!("{name}: size {s} is small; on-screen text should be 28+ units (36+ for body)"));
            }
        }
        if let Some(o) = l.opacity {
            if !(0.0..=1.0).contains(&o) {
                err!(Severity::Error, format!("{name}: opacity {o} must be 0..1"));
            }
        }
        if let Some(c) = l.curve {
            if c.abs() > 0.8 {
                err!(Severity::Warn, format!("{name}: curve {c} is extreme; use -0.5..0.5"));
            }
        }
        // colours
        for (f, c) in [("color", &l.color), ("fill", &l.fill), ("stroke", &l.stroke)] {
            if let Some(c) = c {
                if !color_ok(c) {
                    err!(Severity::Error, format!("{name}: {f} {c:?} is not a colour (use {} or #RRGGBB)", COLORS.join(", ")));
                }
            }
        }
        // from / to
        for (f, p) in [("from", &l.from), ("to", &l.to)] {
            let Some(p) = p else { continue };
            match (l.ty, p) {
                (T::Counter, Pt::Num(_)) => {}
                (T::Counter, _) => err!(Severity::Error, format!("{name}: counter `{f}` must be a number")),
                (T::Arrow | T::Line, Pt::Xy(_)) => {}
                (T::Arrow | T::Line, Pt::Id(id)) => match ids.get(id.as_str()) {
                    Some(T::Box | T::Circle | T::Image) => {}
                    Some(t) => err!(Severity::Error, format!("{name}: `{f}: {id}` points at a {} layer; arrows attach to box, circle or image layers", t.name())),
                    None => err!(Severity::Error, format!("{name}: `{f}: {id}` names no layer (ids: {})", ids.keys().cloned().collect::<Vec<_>>().join(", "))),
                },
                (T::Arrow | T::Line, Pt::Num(_)) => err!(Severity::Error, format!("{name}: `{f}` must be [x, y] or a layer id")),
                _ => {}
            }
        }
        if let (Some(Pt::Id(a)), Some(Pt::Id(b))) = (&l.from, &l.to) {
            if a == b {
                err!(Severity::Error, format!("{name}: from and to are the same layer {a:?}"));
            }
        }
        // geometry inside the box
        let off = |v: f64| !(-0.02..=1.02).contains(&v);
        let mut pts: Vec<(f64, f64)> = vec![];
        if let (Some(x), Some(y)) = (l.x, l.y) {
            pts.push((x, y));
            if let (Some(w), Some(h)) = (l.w, l.h) {
                if matches!(l.ty, T::Box | T::Image | T::Highlight) {
                    pts.push((x + w, y + h));
                }
            }
        }
        for p in l.points.iter().flatten() {
            pts.push((p[0], p[1]));
        }
        for p in [&l.from, &l.to].into_iter().flatten() {
            if let Pt::Xy(p) = p {
                pts.push((p[0], p[1]));
            }
        }
        if pts.iter().any(|(x, y)| off(*x) || off(*y)) {
            err!(Severity::Warn, format!("{name} reaches outside the scene box (x and y are 0..1)"));
        }
        if let (Some(cam), Place::Frame, Some(x), Some(y)) = (camera, place, l.x, l.y) {
            let (w, h) = (l.w.unwrap_or(0.0), l.h.unwrap_or(0.0));
            let (x0, y0, x1, y1) = match l.ty {
                T::Box | T::Image | T::Highlight => (x, y, x + w, y + h),
                _ => (x, y, x, y),
            };
            if x1 > cam[0] && x0 < cam[0] + cam[2] && y1 > cam[1] && y0 < cam[1] + cam[3] {
                err!(Severity::Warn, format!("{name} sits under the camera tile (frame scenes: keep x < {:.2})", cam[0] - 0.01));
            }
        }
        // files
        if let Some(src) = &l.src {
            if !spec_dir.join(src).exists() {
                err!(Severity::Error, format!("{name}: file {src:?} not found"));
            }
        }
        // animation
        let flat = matches!(l.ty, T::Text | T::Image | T::Code | T::Counter);
        for (k, key) in l.keys.iter().enumerate() {
            if key.x.is_none() && key.y.is_none() && key.scale.is_none() && key.opacity.is_none() && key.rotate.is_none() {
                err!(Severity::Warn, format!("{name} keys[{k}] changes nothing (set x, y, scale, opacity or rotate)"));
            }
            if key.rotate.is_some() && flat {
                err!(Severity::Warn, format!("{name} keys[{k}]: rotate is ignored on {} layers", l.ty.name()));
            }
            if key.scale.is_some() && l.ty == T::Code {
                err!(Severity::Warn, format!("{name} keys[{k}]: scale is ignored on code layers"));
            }
            if key.ms.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
                err!(Severity::Error, format!("{name} keys[{k}]: ms must be >= 0"));
            }
            if key.scale.is_some_and(|s| !(s.is_finite() && s > 0.0)) {
                err!(Severity::Error, format!("{name} keys[{k}]: scale must be > 0"));
            }
            if key.opacity.is_some_and(|o| !(0.0..=1.0).contains(&o)) {
                err!(Severity::Error, format!("{name} keys[{k}]: opacity must be 0..1"));
            }
        }
        if let Some(a) = &l.enter {
            if a.to.is_some() {
                err!(Severity::Warn, format!("{name}: `in` takes `from` (where it comes from), not `to`"));
            }
            if a.ms.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
                err!(Severity::Error, format!("{name}: in.ms must be >= 0"));
            }
            if a.draw == Some(true) && matches!(l.ty, T::Text | T::Image | T::Code | T::Counter) {
                err!(Severity::Warn, format!("{name}: in.draw only applies to boxes, circles, lines, arrows and highlights"));
            }
            if a.from == Some(Motion::Scale) && l.ty == T::Code || a.from == Some(Motion::Pop) && l.ty == T::Code {
                err!(Severity::Warn, format!("{name}: code layers do not scale; they rise in"));
            }
        }
        if let Some(a) = &l.exit {
            if a.from.is_some() {
                err!(Severity::Warn, format!("{name}: `out` takes `to` (where it goes), not `from`"));
            }
            if a.at.is_none() {
                err!(Severity::Warn, format!("{name}: `out` without `at` does nothing (layers already fade with the beat)"));
            }
            if a.ms.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
                err!(Severity::Error, format!("{name}: out.ms must be >= 0"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Vec<Layer> {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn parses_the_plan_example() {
        let l = parse(
            r#"
- {id: head, type: text, text: "Claude *talks to* Poppy", x: 0.5, y: 0.14, size: 64, weight: bold, align: center}
- {id: a, type: box, x: 0.08, y: 0.38, w: 0.32, h: 0.26, radius: 24, fill: card, stroke: accent, label: Claude, in: {at: "+0.4s", from: left}}
- {type: arrow, from: [0.40, 0.51], to: [0.60, 0.51], color: accent, width: 6, in: {at: "talks", draw: true}}
- {type: counter, from: 0, to: 10075, prefix: "$", x: 0.5, y: 0.82, size: 72, in: {at: w1302}}
- {type: image, src: proof/rates.png, x: 0.1, y: 0.2, w: 0.8, h: 0.6, in: {from: scale}, keys: [{t: "+2s", scale: 1.08}], ease: in_out}
- {type: arrow, from: a, to: head, curve: 0.2}
"#,
        );
        assert_eq!(l.len(), 6);
        assert_eq!(l[0].weight, Some(Weight::Bold));
        assert_eq!(l[1].enter.as_ref().unwrap().at, Some(Time::Text("+0.4s".into())));
        assert_eq!(l[2].from, Some(Pt::Xy([0.4, 0.51])));
        assert_eq!(l[3].from, Some(Pt::Num(0.0)));
        assert_eq!(l[3].enter.as_ref().unwrap().at, Some(Time::Text("w1302".into())));
        assert_eq!(l[4].keys[0].scale, Some(1.08));
        assert_eq!(l[4].ease, Some(Ease::InOut));
        assert_eq!(l[5].from, Some(Pt::Id("a".into())));
        assert_eq!(times_len(&l), 13);
    }

    #[test]
    fn rejects_unknown_fields() {
        let e = serde_yaml::from_str::<Vec<Layer>>("- {type: text, txt: hi, x: 0, y: 0}").unwrap_err().to_string();
        assert!(e.contains("txt"), "{e}");
        let e = serde_yaml::from_str::<Vec<Layer>>("- {type: blob, x: 0}").unwrap_err().to_string();
        assert!(e.contains("blob"), "{e}");
        let e = serde_yaml::from_str::<Vec<Layer>>("- {type: text, text: a, x: 0, y: 0, in: {at: 1, form: up}}").unwrap_err().to_string();
        assert!(e.contains("form"), "{e}");
    }

    #[test]
    fn times_resolve_and_round_trip() {
        let l = parse(
            r#"
- {type: text, text: a, x: 0, y: 0, in: {at: 1.5}, out: {at: "+3s"}}
- {type: box, x: 0, y: 0, w: 0.1, h: 0.1, keys: [{t: w7, scale: 2}, {t: "+99s", x: 0.5}]}
- {type: text, text: b, x: 0, y: 0, in: {at: "nope"}}
"#,
        );
        let anchor = |a: &Anchor| -> Result<f64> {
            if a.0 == "w7" {
                Ok(2500.0)
            } else {
                anyhow::bail!("phrase {:?} not found", a.0)
            }
        };
        let (flat, probs) = resolve_times(&l, 1000.0, 6000.0, &anchor);
        assert_eq!(flat.len(), times_len(&l));
        let t = unflatten(&l, &flat);
        assert_eq!(t[0].enter, Some(2500.0));
        assert_eq!(t[0].exit, Some(4000.0));
        assert_eq!(t[1].enter, None);
        assert_eq!(t[1].keys, vec![2500.0, 6000.0], "a key after the beat is clamped");
        assert_eq!(t[2].enter, None);
        assert!(probs.iter().any(|(s, m)| *s == Severity::Warn && m.contains("keys[1].t") && m.contains("after")), "{probs:?}");
        assert!(probs.iter().any(|(s, m)| *s == Severity::Error && m.contains("layer 2") && m.contains("not found")), "{probs:?}");
    }

    #[test]
    fn lint_catches_mistakes() {
        let l = parse(
            r#"
- {id: a, type: box, x: 0.1, y: 0.1, w: 0.2, h: 0.2, fill: purpleish, text: hi}
- {id: a, type: circle, x: 0.5, y: 0.5}
- {type: arrow, from: a, to: zz}
- {type: arrow, from: [0.1, 0.1], to: t1}
- {id: t1, type: text, text: hello, x: 1.3, y: 0.5, size: 10}
- {type: counter, to: big, x: 0.5, y: 0.5}
- {type: image, src: missing.png, x: 0.85, y: 0.3, w: 0.1, h: 0.1}
"#,
        );
        let msgs: Vec<String> = lint(&l, Place::Frame, Path::new("/nonexistent"), Some([0.7966, 0.2126, 0.1843, 0.5747])).into_iter().map(|(s, m)| format!("{s:?} {m}")).collect();
        let has = |s: &str| msgs.iter().any(|m| m.contains(s));
        assert!(has("id \"a\" is used twice"), "{msgs:#?}");
        assert!(has("purpleish"), "{msgs:#?}");
        assert!(has("`text` is ignored on box"), "{msgs:#?}");
        assert!(has("needs `x, y and r`"), "{msgs:#?}");
        assert!(has("`to: zz` names no layer"), "{msgs:#?}");
        assert!(has("points at a text layer"), "{msgs:#?}");
        assert!(has("reaches outside the scene box"), "{msgs:#?}");
        assert!(has("size 10"), "{msgs:#?}");
        assert!(has("needs `to (a number)`"), "{msgs:#?}");
        assert!(has("missing.png"), "{msgs:#?}");
        assert!(has("under the camera tile"), "{msgs:#?}");
        let good = parse("- {type: text, text: fine, x: 0.1, y: 0.1, size: 64}");
        assert!(lint(&good, Place::Stage, Path::new("."), None).is_empty());
    }
}
