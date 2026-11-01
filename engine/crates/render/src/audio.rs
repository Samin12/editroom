//! The soundtrack: every kept stretch of the narration (and system audio) decoded to PCM, joined
//! with 5 ms fades at each cut (no clicks), silence under inserted cards, an optional music bed
//! ducked under the narration, encoded once.
//!
//! It is built in fixed blocks of output time (a batch of blocks decoded in parallel, then
//! streamed to the encoder in order), so memory stays bounded however long the video is. Split
//! decodes join sample-exactly (ffmpeg's accurate seek), so block edges are inaudible.

use anyhow::{anyhow, Context, Result};
use cutroom_core::model::{Source, Track};
use cutroom_core::spec::MusicSpec;
use cutroom_core::timemap::Piece;
use cutroom_core::Plan;
use rayon::prelude::*;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

pub const RATE: u32 = 48_000;
const FADE_MS: f64 = 5.0;
/// Output time per block.
const BLOCK_MS: f64 = 20_000.0;
/// Blocks decoded at once (bounds memory: BATCH x BLOCK_MS of stereo f32).
const BATCH: usize = 12;
/// The bed stays ducked this long after speech stops (bridges the gaps between words).
const DUCK_HOLD_MS: f64 = 250.0;
/// Release of the narration level detector (ms).
const DETECT_RELEASE_MS: f64 = 40.0;

/// What goes into the soundtrack besides the narration.
#[derive(Clone, Debug, Default)]
pub struct AudioOpts {
    pub loudnorm: bool,
    /// System audio under the narration (0 = mute).
    pub system_gain: f32,
    /// The music bed and its file (resolved path).
    pub music: Option<(MusicSpec, PathBuf)>,
}

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

/// Whether a file has an audio stream.
pub fn has_audio(path: &Path) -> bool {
    Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "a", "-show_entries", "stream=index", "-of", "csv=p=0"])
        .arg(path)
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// Length of a media file (ms), 0 when unknown.
fn duration_ms(path: &Path) -> f64 {
    Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(path)
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
        .map(|s| s * 1000.0)
        .unwrap_or(0.0)
}

/// The track that carries his voice: the mic, else the camera's own audio, else the system
/// audio. None when there is nothing to hear.
pub fn narration_track(src: &Source) -> Option<Track> {
    let with_audio = |t: Track| {
        let mut seen = std::collections::HashSet::new();
        src.clips.iter().filter(|c| c.track == t && seen.insert(c.media.clone())).filter_map(|c| src.media(&c.media)).any(|m| has_audio(&m.path))
    };
    if src.clips.iter().any(|c| c.track == Track::Mic) {
        return Some(Track::Mic);
    }
    [Track::Camera, Track::System].into_iter().find(|&t| with_audio(t))
}

/// Whether [`render`] has anything to play: narration, system audio or a music bed.
pub fn has_sound(src: &Source, opts: &AudioOpts) -> bool {
    opts.music.is_some() || narration_track(src).is_some()
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

/// Mix the parts overlapping output samples `[b0, b1)` (stereo f32, `(b1 - b0) * 2` values).
fn mix_block(parts: &[Part], b0: u64, b1: u64) -> Result<Vec<f32>> {
    let fade = sample(FADE_MS).max(1);
    let decoded: Vec<(&Part, u64, Vec<f32>)> = parts
        .par_iter()
        .filter(|p| p.s0 < b1 && p.s1 > b0)
        .map(|p| {
            let a = p.s0.max(b0);
            let e = p.s1.min(b1);
            let v = decode(&p.path, p.media_ms + (a - p.s0) as f64 * 1000.0 / RATE as f64, e - a)?;
            Ok((p, a, v))
        })
        .collect::<Result<_>>()?;
    let mut mix = vec![0f32; ((b1 - b0) * 2) as usize];
    for (p, a, v) in &decoded {
        let plen = p.s1 - p.s0;
        let n = v.len() / 2;
        for i in 0..n {
            let idx = a - p.s0 + i as u64;
            let fi = idx.min(plen - 1 - idx);
            let g = if fi < fade { fi as f32 / fade as f32 } else { 1.0 } * p.gain;
            let o = (a - b0) as usize + i;
            mix[o * 2] += v[i * 2] * g;
            mix[o * 2 + 1] += v[i * 2 + 1] * g;
        }
    }
    Ok(mix)
}

fn db(v: f64) -> f32 {
    10f64.powf(v / 20.0) as f32
}

/// One-pole smoothing coefficient for a time constant of `ms`.
fn coef(ms: f64) -> f32 {
    if ms <= 0.0 {
        1.0
    } else {
        (1.0 - (-1.0 / (ms * RATE as f64 / 1000.0)).exp()) as f32
    }
}

/// The music bed: one looping decoder read in step with the soundtrack, ducked by a sidechain
/// on the narration, faded at the very start and end of the video.
struct Bed {
    child: Child,
    pcm: BufReader<ChildStdout>,
    gain: f32,
    duck: f32,
    attack: f32,
    release: f32,
    detect_release: f32,
    threshold: f32,
    hold: u64,
    /// Narration level (short-term peak).
    env: f32,
    /// Current duck gain (1 = full bed).
    g: f32,
    /// Samples of hold left.
    held: u64,
    /// Sample of the whole video (not just the rendered range).
    pos: u64,
    fade: u64,
    end: u64,
    bytes: Vec<u8>,
}

impl Bed {
    fn open(spec: &MusicSpec, path: &Path, start_ms: f64, video_ms: f64) -> Result<Bed> {
        if !path.exists() {
            return Err(anyhow!("music bed {} not found", path.display()));
        }
        let dur = duration_ms(path);
        let seek = if dur > 0.0 { start_ms.rem_euclid(dur) } else { 0.0 };
        let mut child = Command::new("ffmpeg")
            .args(["-nostdin", "-v", "error", "-stream_loop", "-1", "-ss", &format!("{:.6}", seek / 1000.0), "-i"])
            .arg(path)
            .args(["-vn", "-ac", "2", "-ar", &RATE.to_string(), "-f", "f32le", "-"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning ffmpeg for the music bed")?;
        let pcm = BufReader::with_capacity(1 << 20, child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?);
        Ok(Bed {
            child,
            pcm,
            gain: db(spec.gain_db),
            duck: db(-spec.duck_db.abs()),
            attack: coef(spec.attack_ms),
            release: coef(spec.release_ms),
            detect_release: coef(DETECT_RELEASE_MS),
            threshold: db(spec.threshold_db),
            hold: sample(DUCK_HOLD_MS),
            env: 0.0,
            g: 1.0,
            held: 0,
            pos: sample(start_ms),
            fade: sample(spec.fade_ms).max(1),
            end: sample(video_ms),
            bytes: vec![],
        })
    }

    /// Add the next `mix.len() / 2` samples of the bed to `mix` (the narration), ducked under it.
    fn mix_into(&mut self, mix: &mut [f32]) -> Result<()> {
        let n = mix.len();
        self.bytes.resize(n * 4, 0);
        let got = read_full(&mut self.pcm, &mut self.bytes)?;
        self.bytes[got..].fill(0);
        for (i, f) in mix.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            // sidechain: the narration level decides the duck
            let lvl = f[0].abs().max(f[1].abs());
            self.env = if lvl > self.env { lvl } else { self.env + (lvl - self.env) * self.detect_release };
            if self.env > self.threshold {
                self.held = self.hold;
            } else {
                self.held = self.held.saturating_sub(1);
            }
            let target = if self.held > 0 { self.duck } else { 1.0 };
            self.g += (target - self.g) * if target < self.g { self.attack } else { self.release };
            let p = self.pos + i as u64;
            let fade_in = (p as f32 / self.fade as f32).min(1.0);
            let fade_out = (self.end.saturating_sub(p) as f32 / self.fade as f32).min(1.0);
            let k = self.gain * self.g * fade_in * fade_out;
            let o = i * 8;
            let l = f32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap());
            let r = f32::from_le_bytes(self.bytes[o + 4..o + 8].try_into().unwrap());
            f[0] += l * k;
            f[1] += r * k;
        }
        self.pos += (n / 2) as u64;
        Ok(())
    }
}

impl Drop for Bed {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Read until `buf` is full or the stream ends; returns the bytes read.
fn read_full(r: &mut impl Read, buf: &mut [u8]) -> Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match r.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(k) => got += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(got)
}

/// Render the soundtrack of output range `range` (ms) to `out` (AAC .m4a).
pub fn render(plan: &Plan, src: &Source, range: (f64, f64), out: &Path, opts: &AudioOpts) -> Result<()> {
    let total = sample(range.1 - range.0);
    let narration = narration_track(src);
    let mut all = vec![];
    if let Some(t) = narration {
        all.extend(parts(plan, src, range, t, 1.0));
    }
    if opts.system_gain > 0.0 && narration != Some(Track::System) {
        all.extend(parts(plan, src, range, Track::System, opts.system_gain));
    }
    let mut bed = match &opts.music {
        Some((spec, path)) => Some(Bed::open(spec, path, range.0, plan.duration_ms())?),
        None => None,
    };

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-nostdin", "-v", "error", "-y", "-f", "f32le", "-ar", &RATE.to_string(), "-ac", "2", "-i", "-"]);
    if opts.loudnorm {
        cmd.args(["-af", "loudnorm=I=-14:TP=-1.5:LRA=11"]);
    }
    cmd.args(["-c:a", "aac", "-b:a", "256k", "-ar", &RATE.to_string()]).arg(out).stdin(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().context("spawning ffmpeg for audio")?;
    // drain stderr on a thread so a chatty encoder can never block the pipe
    let mut stderr = child.stderr.take().ok_or_else(|| anyhow!("no stderr"))?;
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let fed = (|| -> Result<()> {
        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let block = sample(BLOCK_MS).max(1);
        let blocks: Vec<(u64, u64)> = (0..total.div_ceil(block)).map(|k| (k * block, ((k + 1) * block).min(total))).collect();
        let mut bytes = Vec::with_capacity(1 << 20);
        for batch in blocks.chunks(BATCH) {
            let mixed: Vec<Vec<f32>> = batch.par_iter().map(|&(b0, b1)| mix_block(&all, b0, b1)).collect::<Result<_>>()?;
            for mut m in mixed {
                if let Some(b) = bed.as_mut() {
                    b.mix_into(&mut m)?;
                }
                for chunk in m.chunks(1 << 18) {
                    bytes.clear();
                    bytes.extend(chunk.iter().flat_map(|s| s.clamp(-1.0, 1.0).to_le_bytes()));
                    stdin.write_all(&bytes)?;
                }
            }
        }
        Ok(())
    })();
    drop(bed);
    let st = child.wait()?;
    let err = err_reader.join().unwrap_or_default();
    fed?;
    if !st.success() {
        return Err(anyhow!("audio encode failed: {err}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coefficients() {
        assert_eq!(coef(0.0), 1.0);
        let c = coef(60.0);
        assert!(c > 0.0 && c < 0.001, "{c}");
        assert!((db(-6.0) - 0.501).abs() < 0.001);
    }
}
