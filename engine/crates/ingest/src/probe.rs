//! Media metadata via ffprobe (only container/stream headers are read).

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Probe {
    pub width: u32,
    pub height: u32,
    /// Average frame rate of the first video stream (0 when there is none).
    pub fps: f64,
    pub duration_ms: f64,
    pub has_video: bool,
    pub has_audio: bool,
}

pub fn probe(path: &Path) -> Result<Probe> {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .output()
        .context("running ffprobe")?;
    if !out.status.success() {
        bail!("ffprobe failed on {}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    let mut p = Probe::default();
    let streams = v.get("streams").and_then(Value::as_array).cloned().unwrap_or_default();
    for s in &streams {
        match s.get("codec_type").and_then(Value::as_str) {
            Some("video") if !p.has_video => {
                // cover art / attached pictures are not footage
                if s.pointer("/disposition/attached_pic").and_then(Value::as_i64) == Some(1) {
                    continue;
                }
                p.has_video = true;
                p.width = s.get("width").and_then(Value::as_u64).unwrap_or(0) as u32;
                p.height = s.get("height").and_then(Value::as_u64).unwrap_or(0) as u32;
                p.fps = rate(s.get("avg_frame_rate")).filter(|r| *r > 0.0).or_else(|| rate(s.get("r_frame_rate"))).unwrap_or(0.0);
            }
            Some("audio") => p.has_audio = true,
            _ => {}
        }
    }
    let dur = v.pointer("/format/duration").and_then(Value::as_str).and_then(|d| d.parse::<f64>().ok());
    let dur = dur.or_else(|| streams.iter().filter_map(|s| s.get("duration").and_then(Value::as_str)?.parse::<f64>().ok()).reduce(f64::max));
    p.duration_ms = dur.unwrap_or(0.0) * 1000.0;
    Ok(p)
}

fn rate(v: Option<&Value>) -> Option<f64> {
    let s = v?.as_str()?;
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let (a, b) = (a.parse::<f64>().ok()?, b.parse::<f64>().ok()?);
    (b > 0.0).then(|| a / b)
}
