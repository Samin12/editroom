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
    /// A music bed looped under the whole video, ducked under the narration.
    #[serde(default)]
    pub music: Option<MusicSpec>,
}

impl Default for OutputSpec {
    fn default() -> Self {
        OutputSpec { height: d_height(), fps: d_fps(), chapter_cards: CardMode::default(), card_ms: d_card_ms(), badge: true, captions: false, music: None }
    }
}

/// `output.music`: a bed track looped to the video length, ducked while he talks, faded in at the
/// start and out at the end, mixed before loudness normalization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MusicSpec {
    /// Audio file (relative to the spec's folder, or absolute).
    pub src: String,
    /// Level of the bed against the file's own level (dB).
    #[serde(default = "d_music_gain")]
    pub gain_db: f64,
    /// Extra attenuation while narration is present (dB, positive = quieter).
    #[serde(default = "d_music_duck")]
    pub duck_db: f64,
    /// How fast the bed ducks when he starts talking (ms).
    #[serde(default = "d_music_attack")]
    pub attack_ms: f64,
    /// How fast it comes back after he stops (ms).
    #[serde(default = "d_music_release")]
    pub release_ms: f64,
    /// Fade in at the start and out at the end of the video (ms).
    #[serde(default = "d_music_fade")]
    pub fade_ms: f64,
    /// Narration level (dBFS, short-term peak) above which the bed ducks.
    #[serde(default = "d_music_threshold")]
    pub threshold_db: f64,
}

fn d_music_gain() -> f64 {
    -18.0
}
fn d_music_duck() -> f64 {
    12.0
}
fn d_music_attack() -> f64 {
    60.0
}
fn d_music_release() -> f64 {
    450.0
}
fn d_music_fade() -> f64 {
    1500.0
}
fn d_music_threshold() -> f64 {
    -45.0
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
    /// Who decided the cut: `agent` (cutroom agent's editorial pass), `human`, or none (the
    /// detector). `cutroom cut` and `cutroom agent` regenerate their own cuts and keep `human` ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// One line for humans ("re-said at w412").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
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
        /// A request to capture real proof into `src`. While the file is missing the beat is
        /// pending: `check` warns, `render` refuses, `cutroom proofs` lists it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proof: Option<ProofReq>,
    },
    /// A muted video clip (b-roll, real footage, a pre-rendered animation). `fit: card` plays it
    /// on a card in the panel, `panel` fills the panel slot, `frame` covers the whole frame under
    /// the camera tile (with `alpha`, a transparent overlay such as a lower third).
    Video {
        src: String,
        /// Start inside the clip, seconds.
        #[serde(default)]
        from: Option<f64>,
        /// Shown under the card (`fit: card` only).
        #[serde(default)]
        caption: Option<String>,
        #[serde(default)]
        fit: VideoFit,
        /// Composite the clip's transparency. Default: detected from the file.
        #[serde(default)]
        alpha: Option<bool>,
        /// Restart at the end. Default: true for `card`, false otherwise (the last frame holds).
        #[serde(default, rename = "loop")]
        looped: Option<bool>,
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
    /// A custom animation: layers (text, shapes, arrows, images, code, counters, highlights)
    /// with their own entrances, exits and keyframes, anchored to words. See `crate::scene`.
    Scene {
        #[serde(default)]
        place: crate::scene::Place,
        #[serde(default)]
        bg: crate::scene::Bg,
        layers: Vec<crate::scene::Layer>,
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

/// Where a `video` beat is drawn.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum VideoFit {
    /// On a card in the panel, with an optional caption (the stage look).
    #[default]
    Card,
    /// Fills the panel slot (cover-cropped, rounded like the screen recording).
    Panel,
    /// Covers the whole frame, above the screen and below the camera tile and badge.
    Frame,
}

impl VideoFit {
    /// Whether a clip of this fit loops by default.
    pub fn loops_by_default(self) -> bool {
        self == VideoFit::Card
    }
}

/// What an `image` beat should show once someone captures it: the claim he makes and where to find
/// the real thing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProofReq {
    pub claim: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub find: Option<String>,
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
            // A full-frame clip does not take over the panel: what is under it keeps playing.
            Video { fit: VideoFit::Frame, .. } => Lane::Overlay,
            Title { .. } | Bullets { .. } | Number { .. } | Image { .. } | Video { .. } | Code { .. } | Quote { .. } | Scene { .. } => Lane::Stage,
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
            Scene { .. } => "scene",
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
            Scene { .. } => 6000.0,
        }
    }

    /// A stage beat drawn over the whole frame instead of the panel (`scene` with
    /// `place: frame`).
    pub fn full_frame(&self) -> bool {
        matches!(self, BeatKind::Scene { place: crate::scene::Place::Frame, .. })
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
        let s = match read_spec(&inc) {
            Ok(s) => s,
            // another process (the agent archiving last run's beats) removed it after we listed the
            // folder: a file that is gone contributes nothing
            Err(_) if !inc.exists() => continue,
            Err(e) => return Err(e),
        };
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

/// The source.json a spec points at, from the root file alone (no includes). For commands that only
/// read the recording, so they never race with agents rewriting beat files.
pub fn source_path(path: &Path) -> Result<PathBuf> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let root: Spec = read_spec(path)?;
    Ok(dir.join(root.source.unwrap_or_else(|| "source.json".into())))
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

/// One value as a single-line YAML flow mapping (compact JSON, which YAML reads as-is), with null
/// fields left out. Used for machine-written spec lines (`- {"from":"w12","why":"retake"}`).
pub fn flow<T: Serialize>(v: &T) -> String {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.retain(|_, x| !x.is_null());
                m.values_mut().for_each(strip);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut j = serde_json::to_value(v).unwrap_or(serde_json::Value::Null);
    strip(&mut j);
    serde_json::to_string(&j).unwrap_or_default()
}

pub fn read_spec(path: &Path) -> Result<Spec> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Spec::default());
    }
    serde_yaml::from_str(&text).map_err(|e| anyhow!("{}: {e}", path.display()))
}
