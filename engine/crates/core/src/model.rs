//! The raw recording, flattened: one "source timeline" holding every take back to back.
//!
//! All times are milliseconds on the source timeline (the raw recording before any cut),
//! except `Clip::media_in_ms`, which is the time inside the media file.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Track {
    /// The presenter's camera.
    Camera,
    /// The screen recording.
    Screen,
    /// The narration microphone. Its words drive everything.
    Mic,
    /// System audio (a clip playing on screen).
    System,
}

impl Track {
    pub fn is_video(self) -> bool {
        matches!(self, Track::Camera | Track::Screen)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Media {
    pub id: String,
    pub path: PathBuf,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub fps: f64,
    pub duration_ms: f64,
}

/// A piece of a media file placed on the source timeline.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Clip {
    pub track: Track,
    pub media: String,
    /// Source-timeline start.
    pub start_ms: f64,
    pub dur_ms: f64,
    /// Time inside the media file that plays at `start_ms`.
    pub media_in_ms: f64,
    /// Take index (takes are numbered in source order).
    #[serde(default)]
    pub take: u32,
}

impl Clip {
    pub fn end_ms(&self) -> f64 {
        self.start_ms + self.dur_ms
    }
    pub fn contains(&self, t: f64) -> bool {
        t >= self.start_ms && t < self.end_ms()
    }
    /// Media time playing at source time `t`.
    pub fn media_at(&self, t: f64) -> f64 {
        self.media_in_ms + (t - self.start_ms)
    }
}

/// One transcribed word. Its id is its index in `Source::words`, written `w<id>` in specs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Scene {
    pub name: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

/// Everything the engine knows about the raw recording. Written by `cutroom ingest`.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Source {
    #[serde(default = "one")]
    pub version: u32,
    pub name: String,
    /// Where it came from (a Borumi bundle path, or "files").
    #[serde(default)]
    pub origin: String,
    pub duration_ms: f64,
    pub media: Vec<Media>,
    pub clips: Vec<Clip>,
    /// Sorted by start, non-overlapping.
    pub words: Vec<Word>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
}

fn one() -> u32 {
    1
}

impl Source {
    pub fn media(&self, id: &str) -> Option<&Media> {
        self.media.iter().find(|m| m.id == id)
    }

    /// The clip of `track` playing at source time `t` (the latest-starting one wins when stacked).
    pub fn clip_at(&self, track: Track, t: f64) -> Option<&Clip> {
        self.clips
            .iter()
            .filter(|c| c.track == track && c.contains(t))
            .max_by(|a, b| a.start_ms.total_cmp(&b.start_ms))
    }

    /// Index of the first word whose end is after `t`.
    pub fn word_at_or_after(&self, t: f64) -> usize {
        self.words.partition_point(|w| w.end_ms <= t)
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Source> {
        let s: Source = serde_json::from_slice(&std::fs::read(path)?)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        Ok(s)
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        std::fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    /// Scenes `a..=b` (0-based) only, shifted to start at 0: clips and words outside are dropped,
    /// clips crossing the edges are trimmed. Media keep their ids.
    pub fn slice_scenes(&self, a: usize, b: usize) -> Source {
        let b = b.min(self.scenes.len().saturating_sub(1));
        let (t0, t1) = match (self.scenes.get(a), self.scenes.get(b)) {
            (Some(x), Some(y)) => (x.start_ms, y.end_ms),
            _ => return self.clone(),
        };
        let clips = self
            .clips
            .iter()
            .filter(|c| c.end_ms() > t0 && c.start_ms < t1)
            .map(|c| {
                let s = c.start_ms.max(t0);
                let e = c.end_ms().min(t1);
                Clip { start_ms: s - t0, dur_ms: e - s, media_in_ms: c.media_in_ms + (s - c.start_ms), ..c.clone() }
            })
            .collect::<Vec<_>>();
        let used: std::collections::HashSet<&str> = clips.iter().map(|c| c.media.as_str()).collect();
        Source {
            version: self.version,
            name: format!("{} (scenes {}-{})", self.name, a + 1, b + 1),
            origin: self.origin.clone(),
            duration_ms: t1 - t0,
            media: self.media.iter().filter(|m| used.contains(m.id.as_str())).cloned().collect(),
            words: self.words.iter().filter(|w| w.start_ms >= t0 && w.end_ms <= t1).map(|w| Word { text: w.text.clone(), start_ms: w.start_ms - t0, end_ms: w.end_ms - t0 }).collect(),
            scenes: self.scenes[a..=b].iter().map(|s| Scene { name: s.name.clone(), start_ms: s.start_ms - t0, end_ms: s.end_ms - t0 }).collect(),
            clips,
        }
    }

    /// Basic invariants: words sorted, clips inside the timeline, media referenced exist.
    pub fn validate(&self) -> Vec<String> {
        let mut out = vec![];
        for (i, w) in self.words.windows(2).enumerate() {
            if w[1].start_ms < w[0].start_ms {
                out.push(format!("words not sorted at w{}", i + 1));
                break;
            }
        }
        for c in &self.clips {
            if self.media(&c.media).is_none() {
                out.push(format!("clip references unknown media {}", c.media));
            }
            if c.dur_ms <= 0.0 {
                out.push(format!("clip of {} has no duration", c.media));
            }
        }
        out
    }
}
