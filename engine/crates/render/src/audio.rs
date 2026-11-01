//! The soundtrack: every kept stretch of the mic (and system audio) decoded to PCM, joined with
//! 5 ms fades at each cut (no clicks), silence under inserted cards, encoded once.

use anyhow::{anyhow, Context, Result};
use cutroom_core::model::{Source, Track};
use cutroom_core::timemap::Piece;
use cutroom_core::Plan;
use rayon::prelude::*;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const RATE: u32 = 48_000;
const FADE_MS: f64 = 5.0;

#[derive(Clone, Debug)]
struct Part {
    /// Output sample range.
    s0: u64,
    s1: u64,
    path: PathBuf,
    media_ms: f64,
    gain: f32,
}

fn sample(ms: f64) -> u64 {
    (ms * RATE as f64 / 1000.0).round().max(0.0) as u64
}

/// Decode `n` stereo f32 samples of `path` from `media_ms`.
fn decode(path: &Path, media_ms: f64, n: u64) -> Result<Vec<f32>> {
    let out = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-ss", &format!("{:.6}", media_ms / 1000.0)])
        .arg("-i")
        .arg(path)
        .args(["-vn", "-ac", "2", "-ar", &RATE.to_string(), "-t", &format!("{:.6}", n as f64 / RATE as f64 + 0.05), "-f", "f32le", "-"])
        .stderr(Stdio::null())
        .output()
        .with_context(|| format!("decoding audio of {}", path.display()))?;
    let mut v: Vec<f32> = out.stdout.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    v.resize((n * 2) as usize, 0.0);
    Ok(v)
}

fn parts(plan: &Plan, src: &Source, range: (f64, f64), track: Track, gain: f32) -> Vec<Part> {
    let mut out = vec![];
    for p in &plan.map.pieces {
        let Piece::Src { src0, src1, out0 } = *p else { continue };
        for c in src.clips.iter().filter(|c| c.track == track) {
            let a = src0.max(c.start_ms);
            let b = src1.min(c.end_ms());
            if b <= a {
                continue;
            }
            let oa = (out0 + (a - src0)).max(range.0);
            let ob = (out0 + (b - src0)).min(range.1);
            if ob <= oa {
                continue;
            }
            let Some(m) = src.media(&c.media) else { continue };
            let media_ms = c.media_at(a + (oa - (out0 + (a - src0))));
            out.push(Part { s0: sample(oa - range.0), s1: sample(ob - range.0), path: m.path.clone(), media_ms, gain });
        }
    }
    out
}

/// Render the soundtrack of output range `range` (ms) to `out` (AAC .m4a).
pub fn render(plan: &Plan, src: &Source, range: (f64, f64), out: &Path, loudnorm: bool, system_gain: f32) -> Result<()> {
    let total = sample(range.1 - range.0);
    let mut all = parts(plan, src, range, Track::Mic, 1.0);
    if system_gain > 0.0 {
        all.extend(parts(plan, src, range, Track::System, system_gain));
    }
    // decode in parallel
    let decoded: Vec<(Part, Vec<f32>)> = all
        .into_par_iter()
        .map(|p| {
            let n = p.s1 - p.s0;
            let v = decode(&p.path, p.media_ms, n)?;
            Ok((p, v))
        })
        .collect::<Result<_>>()?;
    let mut mix = vec![0f32; (total * 2) as usize];
    let fade = sample(FADE_MS).max(1);
    for (p, v) in &decoded {
        let n = (p.s1 - p.s0) as usize;
        for i in 0..n {
            let o = p.s0 as usize + i;
            if o >= total as usize {
                break;
            }
            let fi = (i as u64).min(n as u64 - 1 - i as u64);
            let g = if fi < fade { fi as f32 / fade as f32 } else { 1.0 } * p.gain;
            mix[o * 2] += v[i * 2] * g;
            mix[o * 2 + 1] += v[i * 2 + 1] * g;
        }
    }
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-nostdin", "-v", "error", "-y", "-f", "f32le", "-ar", &RATE.to_string(), "-ac", "2", "-i", "-"]);
    if loudnorm {
        cmd.args(["-af", "loudnorm=I=-14:TP=-1.5:LRA=11"]);
    }
    cmd.args(["-c:a", "aac", "-b:a", "256k", "-ar", &RATE.to_string()]).arg(out).stdin(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().context("spawning ffmpeg for audio")?;
    {
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let mut buf = Vec::with_capacity(1 << 20);
        for chunk in mix.chunks(1 << 18) {
            buf.clear();
            buf.extend(chunk.iter().flat_map(|s| s.clamp(-1.0, 1.0).to_le_bytes()));
            stdin.write_all(&buf)?;
        }
    }
    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut err);
    }
    let st = child.wait()?;
    if !st.success() {
        return Err(anyhow!("audio encode failed: {err}"));
    }
    Ok(())
}
