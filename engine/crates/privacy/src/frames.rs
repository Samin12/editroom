//! Frame sampling with ffmpeg: one process per chunk of media time, gray frames on a pipe.
//!
//! Screen recordings with dense keyframes (Borumi's HEVC has one every ~0.47 s) are sampled from
//! keyframes only (`-skip_frame nokey`), which skips decoding everything in between; otherwise
//! every frame is decoded and the `fps` filter picks the samples. Software decode beats
//! `-hwaccel videotoolbox` for this: the GPU copy-back of 5K frames costs more than it saves.

use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, sync_channel, Receiver};
use std::sync::{Arc, Mutex};

pub(crate) struct Frame {
    /// Media time (ms).
    pub t_ms: f64,
    pub px: Vec<u8>,
}

pub(crate) struct Probe {
    pub w: usize,
    pub h: usize,
    pub dur_ms: f64,
}

pub(crate) fn probe(ffprobe: &str, path: &Path) -> Result<Probe> {
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,duration:format=duration", "-of", "default=nw=1"])
        .arg(path)
        .output()
        .with_context(|| format!("running {ffprobe} (is ffmpeg installed?)"))?;
    if !out.status.success() {
        bail!("ffprobe {}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (mut w, mut h, mut dur) = (0, 0, None);
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("width=") {
            w = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("height=") {
            h = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = l.strip_prefix("duration=") {
            if dur.is_none() {
                dur = v.trim().parse::<f64>().ok();
            }
        }
    }
    if w == 0 || h == 0 {
        bail!("{}: no video stream", path.display());
    }
    Ok(Probe { w, h, dur_ms: dur.unwrap_or(0.0) * 1000.0 })
}

/// Keyframe times (media ms) in `a..b`, sorted. Reads packet headers only (fast).
pub(crate) fn keyframes(ffprobe: &str, path: &Path, a_ms: f64, b_ms: f64) -> Result<Vec<f64>> {
    let iv = format!("{:.3}%{:.3}", (a_ms / 1000.0).max(0.0), b_ms / 1000.0);
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0", "-read_intervals", &iv])
        .arg(path)
        .output()
        .with_context(|| format!("running {ffprobe}"))?;
    if !out.status.success() {
        bail!("ffprobe {}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    let mut k: Vec<f64> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let (t, flags) = l.split_once(',')?;
            flags.contains('K').then(|| t.trim().parse::<f64>().ok()).flatten().map(|s| s * 1000.0)
        })
        .collect();
    k.sort_by(f64::total_cmp);
    k.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    Ok(k)
}

/// A stretch of media to decode.
#[derive(Clone, Debug)]
pub(crate) struct Chunk {
    /// Seek here (media ms) and decode `len_ms`.
    pub a_ms: f64,
    pub len_ms: f64,
    /// Keyframe mode: decode keyframes only and keep these (media ms).
    pub keep: Option<Vec<f64>>,
    /// Otherwise: one frame every `every_ms` from `a_ms` (0: every frame).
    pub every_ms: f64,
    /// Only this box `[x, y, w, h]` of the (scaled) frame.
    pub crop: Option<[usize; 4]>,
}

/// Frames of one chunk, decoded ahead on a background thread.
pub(crate) struct Decoder {
    rx: Receiver<Frame>,
    child: Arc<Mutex<Child>>,
    log: Arc<Mutex<Vec<String>>>,
}

pub(crate) fn decode(ffmpeg: &str, path: &Path, c: &Chunk, w: usize, h: usize) -> Result<Decoder> {
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-hide_banner", "-nostats", "-loglevel", "info", "-nostdin"]);
    if c.keep.is_some() {
        cmd.args(["-skip_frame", "nokey"]);
    }
    cmd.args(["-ss", &format!("{:.3}", c.a_ms / 1000.0), "-t", &format!("{:.3}", c.len_ms / 1000.0), "-i"]).arg(path);
    let mut vf = String::new();
    if c.keep.is_none() && c.every_ms > 0.0 {
        vf.push_str(&format!("fps=fps={:.6}:round=near,", 1000.0 / c.every_ms));
    }
    vf.push_str(&format!("scale={w}:{h}:flags=area,"));
    if let Some([x, y, cw, ch]) = c.crop {
        vf.push_str(&format!("crop={cw}:{ch}:{x}:{y},"));
    }
    vf.push_str("format=gray,showinfo");
    cmd.args(["-map", "0:v:0", "-an", "-sn", "-dn", "-vf", &vf, "-fps_mode", "passthrough", "-f", "rawvideo", "-pix_fmt", "gray", "pipe:1"]);
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().with_context(|| format!("running {ffmpeg} (is ffmpeg installed?)"))?;
    let mut stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let child = Arc::new(Mutex::new(child));
    let log = Arc::new(Mutex::new(vec![]));

    // showinfo prints each frame's pts before the frame reaches the pipe
    let (pts_tx, pts_rx) = channel::<f64>();
    let log2 = log.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
            if line.contains("showinfo") {
                if let Some(i) = line.find("pts_time:") {
                    let v: String = line[i + 9..].chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
                    if let Ok(v) = v.parse::<f64>() {
                        let _ = pts_tx.send(v * 1000.0);
                    }
                }
            } else {
                let mut l = log2.lock().unwrap();
                l.push(line);
                if l.len() > 20 {
                    l.remove(0);
                }
            }
        }
    });

    let (tx, rx) = sync_channel::<Frame>(2);
    let size = c.crop.map_or(w * h, |r| r[2] * r[3]);
    let (a, keep) = (c.a_ms, c.keep.clone());
    // the fps filter can emit one frame past the chunk; the next chunk has it
    let end = if keep.is_some() { f64::MAX } else { c.a_ms + c.len_ms - c.every_ms / 2.0 };
    std::thread::spawn(move || loop {
        let mut px = vec![0u8; size];
        if stdout.read_exact(&mut px).is_err() {
            break;
        }
        let Ok(rel) = pts_rx.recv() else { break };
        let t_ms = a + rel;
        if t_ms >= end {
            continue;
        }
        if let Some(k) = &keep {
            let i = k.partition_point(|&x| x < t_ms - 2.0);
            if !k.get(i).is_some_and(|&x| (x - t_ms).abs() <= 2.0) {
                continue;
            }
        }
        if tx.send(Frame { t_ms, px }).is_err() {
            break;
        }
    });
    Ok(Decoder { rx, child, log })
}

impl Iterator for Decoder {
    type Item = Frame;
    fn next(&mut self) -> Option<Frame> {
        self.rx.recv().ok()
    }
}

impl Decoder {
    /// Wait for ffmpeg; an error when it failed.
    pub fn finish(self) -> Result<()> {
        let status = self.child.lock().unwrap().wait()?;
        if !status.success() {
            let log = self.log.lock().unwrap();
            return Err(anyhow!("ffmpeg failed ({status}): {}", log.iter().rev().take(3).rev().cloned().collect::<Vec<_>>().join(" | ")));
        }
        Ok(())
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        if let Ok(mut c) = self.child.lock() {
            if c.try_wait().ok().flatten().is_none() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}
