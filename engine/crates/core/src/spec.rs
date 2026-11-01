//! The edit spec: a small YAML file an agent writes. Everything is anchored to WORDS, never to
//! milliseconds, so re-cutting never knocks a visual out of sync.
//!
//! ```yaml
//! source: source.json
//! include: [cuts.yaml, beats/ch01.yaml]
//! chapters:
//!   - {at: w12, title: Find the idea}
//! beats:
//!   - {at: "connect Claude to Poppy", kind: title, text: "Connect Claude + Poppy"}
//!   - {at: w300, until: w340, kind: ring, rect: [0.62, 0.08, 0.2, 0.06]}
//! ```

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A normalized rectangle `[x, y, w, h]` (0..1).
pub type RectN = [f64; 4];
/// A normalized point `[x, y]` (0..1).
pub type PointN = [f64; 2];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub output: OutputSpec,
    /// Theme overrides passed to the graphics renderer (accent, canvas, ink, ...).
    #[serde(default)]
    pub style: BTreeMap<String, String>,
    #[serde(default)]
    pub layout: LayoutSpec,
    /// Other spec files (relative paths) whose cuts, chapters and beats are merged in.
    #[serde(default)]
    pub include: Vec<String>,
    /// Phrase anchors in this file only match words inside this range (`[from, to]`).
    #[serde(default)]
    pub scope: Option<[Anchor; 2]>,
    #[serde(default)]
    pub cuts: Vec<CutSpec>,
    #[serde(default)]
    pub chapters: Vec<ChapterSpec>,
    #[serde(default)]
    pub beats: Vec<BeatSpec>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct OutputSpec {
    #[serde(default = "d_height")]
    pub height: u32,
    #[serde(default = "d_fps")]
    pub fps: f64,
    /// `insert` adds a short card before each chapter (narration pauses), `overlay` shows it over
    /// the panel without adding time, `off` shows none.
    #[serde(default)]
    pub chapter_cards: CardMode,
    #[serde(default = "d_card_ms")]
    pub card_ms: f64,
    /// Persistent "step" badge above the camera tile.
    #[serde(default = "d_true")]
    pub badge: bool,
    /// Burn captions into the picture (an .srt is always written).
    #[serde(default)]
    pub captions: bool,
}

impl Default for OutputSpec {
    fn default() -> Self {
        OutputSpec { height: d_height(), fps: d_fps(), chapter_cards: CardMode::default(), card_ms: d_card_ms(), badge: true, captions: false }
    }
}

fn d_height() -> u32 {
    1080
}
fn d_fps() -> f64 {
    30.0
}
fn d_card_ms() -> f64 {
    1800.0
}
fn d_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardMode {
    #[default]
    Insert,
    Overlay,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Screen recording in the left panel, camera tile on the right (Samin's default look).
    #[default]
    Panel,
    /// Full-frame screen recording, no camera.
    Screen,
    /// Full-frame camera.
    Camera,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// Fill the slot, cropping the overflow (Borumi's behaviour).
    #[default]
    Cover,
    /// Show the whole frame, letterboxed.
    Contain,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmptyPanel {
    /// When the panel has nothing to show, switch to the full-frame camera.
    #[default]
    Camera,
    /// Treat an empty panel as an error in `check`.
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LayoutSpec {
    #[serde(default)]
    pub default: LayoutMode,
    #[serde(default)]
    pub fit: Fit,
    #[serde(default)]
    pub empty_panel: EmptyPanel,
    /// Panel slot (frame-normalized). Default: Samin's Borumi screen layout.
    #[serde(default = "d_panel")]
    pub panel: RectN,
    /// Camera tile (frame-normalized).
    #[serde(default = "d_camera")]
    pub camera: RectN,
    /// Crop of the camera frame shown in the tile (camera-normalized); default: centre crop.
    #[serde(default)]
    pub camera_crop: Option<RectN>,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        LayoutSpec { default: LayoutMode::Panel, fit: Fit::Cover, empty_panel: EmptyPanel::Camera, panel: d_panel(), camera: d_camera(), camera_crop: None }
    }
}

pub fn d_panel() -> RectN {
    [0.01, 0.0178, 0.7676, 0.9644]
}
pub fn d_camera() -> RectN {
    [0.7966, 0.2126, 0.1843, 0.5747]
}

/// An anchor: `w123` (a word id), `w123+0.5s`, a quoted phrase (`"connect claude"`, `"..."#2` for
/// the 2nd match), `s:754.2` (source seconds) or `o:61.5` (output seconds).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(transparent)]
pub struct Anchor(pub String);

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct CutSpec {
    /// First word removed.
    #[serde(default)]
    pub from: Option<Anchor>,
    /// Last word removed (inclusive). Defaults to `from`.
    #[serde(default)]
    pub to: Option<Anchor>,
    /// Exact source range in milliseconds (machine-written cuts: silences).
    #[serde(default)]
    pub ms: Option<[f64; 2]>,
    #[serde(default)]
    pub why: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ChapterSpec {
    pub at: Anchor,
    pub title: String,
    /// Short label for the badge (defaults to the title).
    #[serde(default)]
    pub short: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BeatSpec {
    /// Where the beat starts (lands `lead` before this word).
    pub at: Anchor,
    /// Where it ends (the END of this word). Default: `hold` after `at`.
    #[serde(default)]
    pub until: Option<Anchor>,
    /// Duration when there is no `until`: "3s", "1500ms" or seconds.
    #[serde(default)]
    pub hold: Option<Dur>,
    /// Lead before the anchor word, ms (default 130, about 4 frames).
    #[serde(default)]
    pub lead: Option<f64>,
    #[serde(flatten)]
    pub kind: BeatKind,
}

/// A duration: `3s`, `1500ms`, `2.5` (seconds).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Dur {
    Secs(f64),
    Text(String),
}

impl Dur {
    pub fn ms(&self) -> Result<f64> {
        match self {
            Dur::Secs(s) => Ok(s * 1000.0),
            Dur::Text(t) => parse_dur_ms(t),
        }
    }
}

pub fn parse_dur_ms(t: &str) -> Result<f64> {
    let t = t.trim();
    let (num, mul) = if let Some(n) = t.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = t.strip_suffix('s') {
        (n, 1000.0)
    } else {
        (t, 1000.0)
    };
    num.trim().parse::<f64>().map(|v| v * mul).map_err(|_| anyhow!("bad duration {t:?} (use 3s, 1500ms or 2.5)"))
}

/// A bullet item: plain text, or text that appears at its own anchor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Item {
    Text(String),
    At { text: String, at: Anchor },
}

impl Item {
    pub fn text(&self) -> &str {
        match self {
            Item::Text(t) => t,
            Item::At { text, .. } => text,
        }
    }
}

/// What a beat shows. Stage beats replace the panel content; overlays draw on top; screen ops
/// change how the screen recording is shown.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum BeatKind {
    // ---- stage (fills the panel) ----
    /// A headline that builds word by word.
    Title {
        text: String,
        #[serde(default)]
        kicker: Option<String>,
        #[serde(default)]
        sub: Option<String>,
    },
    /// A list; items with their own `at` appear when he says them, others stagger in.
    Bullets {
        #[serde(default)]
        title: Option<String>,
        items: Vec<Item>,
    },
    /// A big number (counts up when it starts with digits).
    Number {
        value: String,
        #[serde(default)]
        label: Option<String>,
    },
    /// A screenshot or photo card with a slow push-in; optional highlight (image-normalized).
    Image {
        src: String,
        #[serde(default)]
        caption: Option<String>,
        #[serde(default)]
        highlight: Option<RectN>,
        /// Small source pill ("youtube.com", "Stripe dashboard").
        #[serde(default)]
        source: Option<String>,
    },
    /// A muted video clip playing in the panel (b-roll, real footage).
    Video {
        src: String,
        /// Start inside the clip, seconds.
        #[serde(default)]
        from: Option<f64>,
        #[serde(default)]
        caption: Option<String>,
    },
    /// A code / prompt card that types on.
    Code {
        text: String,
        #[serde(default)]
        title: Option<String>,
    },
    Quote {
        text: String,
        #[serde(default)]
        by: Option<String>,
    },
    // ---- overlays (coordinates are SCREEN-normalized, so they follow the panel and zooms) ----
    /// A ring drawn around something on screen.
    Ring { rect: RectN },
    /// A marker highlight (or underline) on screen text.
    Highlight {
        rect: RectN,
        #[serde(default)]
        underline: bool,
    },
    Arrow { from: PointN, to: PointN },
    /// A pill of text. `pos` is FRAME-normalized (default: top of the panel).
    Callout {
        text: String,
        #[serde(default)]
        pos: Option<PointN>,
    },
    /// Lower-third band.
    Lower {
        title: String,
        #[serde(default)]
        sub: Option<String>,
    },
    // ---- screen ops ----
    /// Blur rectangles of the screen recording (privacy). Screen-normalized.
    Blur { rects: Vec<RectN> },
    /// Zoom the panel into this screen region.
    Zoom { rect: RectN },
    /// Switch layout for this stretch.
    Layout { mode: LayoutMode },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Lane {
    Stage,
    Overlay,
    Screen,
    Layout,
}

impl BeatKind {
    pub fn lane(&self) -> Lane {
        use BeatKind::*;
        match self {
            Title { .. } | Bullets { .. } | Number { .. } | Image { .. } | Video { .. } | Code { .. } | Quote { .. } => Lane::Stage,
            Ring { .. } | Highlight { .. } | Arrow { .. } | Callout { .. } | Lower { .. } => Lane::Overlay,
            Blur { .. } | Zoom { .. } => Lane::Screen,
            Layout { .. } => Lane::Layout,
        }
    }

    pub fn name(&self) -> &'static str {
        use BeatKind::*;
        match self {
            Title { .. } => "title",
            Bullets { .. } => "bullets",
            Number { .. } => "number",
            Image { .. } => "image",
            Video { .. } => "video",
            Code { .. } => "code",
            Quote { .. } => "quote",
            Ring { .. } => "ring",
            Highlight { .. } => "highlight",
            Arrow { .. } => "arrow",
            Callout { .. } => "callout",
            Lower { .. } => "lower",
            Blur { .. } => "blur",
            Zoom { .. } => "zoom",
            Layout { .. } => "layout",
        }
    }

    /// Default on-screen time when the beat has neither `until` nor `hold`.
    pub fn default_hold_ms(&self) -> f64 {
        use BeatKind::*;
        match self {
            Title { .. } | Number { .. } | Quote { .. } => 3500.0,
            Bullets { items, .. } => 2500.0 + 900.0 * items.len() as f64,
            Image { .. } | Video { .. } => 5000.0,
            Code { text, .. } => (2500.0 + 25.0 * text.len() as f64).min(9000.0),
            Ring { .. } | Highlight { .. } | Arrow { .. } | Callout { .. } => 2500.0,
            Lower { .. } => 4000.0,
            Blur { .. } | Zoom { .. } | Layout { .. } => 3000.0,
        }
    }
}

/// A spec with its includes merged. Beats remember which file they came from (for messages)
/// and the phrase scope of that file.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub spec: Spec,
    pub dir: PathBuf,
    pub source_path: PathBuf,
    /// Per beat / chapter / cut: (file, scope).
    pub beat_origin: Vec<Origin>,
    pub chapter_origin: Vec<Origin>,
    pub cut_origin: Vec<Origin>,
}

#[derive(Clone, Debug, Default)]
pub struct Origin {
    pub file: String,
    pub scope: Option<[Anchor; 2]>,
}

pub fn load(path: &Path) -> Result<Loaded> {
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut root: Spec = read_spec(path)?;
    let file = rel(path, &dir);
    let mut beat_origin = vec![Origin { file: file.clone(), scope: root.scope.clone() }; root.beats.len()];
    let mut chapter_origin = vec![Origin { file: file.clone(), scope: root.scope.clone() }; root.chapters.len()];
    let mut cut_origin = vec![Origin { file: file.clone(), scope: root.scope.clone() }; root.cuts.len()];
    let includes = expand_includes(&dir, &root.include)?;
    for inc in includes {
        let s = read_spec(&inc)?;
        let f = rel(&inc, &dir);
        let o = Origin { file: f, scope: s.scope.clone() };
        beat_origin.extend(std::iter::repeat_n(o.clone(), s.beats.len()));
        chapter_origin.extend(std::iter::repeat_n(o.clone(), s.chapters.len()));
        cut_origin.extend(std::iter::repeat_n(o.clone(), s.cuts.len()));
        root.beats.extend(s.beats);
        root.chapters.extend(s.chapters);
        root.cuts.extend(s.cuts);
        if !s.include.is_empty() {
            return Err(anyhow!("{}: nested include is not supported", inc.display()));
        }
    }
    let source_path = dir.join(root.source.clone().unwrap_or_else(|| "source.json".into()));
    Ok(Loaded { spec: root, dir, source_path, beat_origin, chapter_origin, cut_origin })
}

fn rel(p: &Path, dir: &Path) -> String {
    p.strip_prefix(dir).unwrap_or(p).display().to_string()
}

/// Include entries may end in `/*.yaml` to take every YAML file of a folder (sorted).
fn expand_includes(dir: &Path, inc: &[String]) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    for i in inc {
        if let Some(folder) = i.strip_suffix("/*.yaml").or_else(|| i.strip_suffix("/*.yml")) {
            let d = dir.join(folder);
            if !d.exists() {
                continue;
            }
            let mut files: Vec<PathBuf> = std::fs::read_dir(&d)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yaml") | Some("yml")))
                .collect();
            files.sort();
            out.extend(files);
        } else {
            let p = dir.join(i);
            if p.exists() {
                out.push(p);
            } else {
                return Err(anyhow!("include {i:?} not found"));
            }
        }
    }
    Ok(out)
}

pub fn read_spec(path: &Path) -> Result<Spec> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Spec::default());
    }
    serde_yaml::from_str(&text).map_err(|e| anyhow!("{}: {e}", path.display()))
}
