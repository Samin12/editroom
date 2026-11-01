//! Decoders: one ffmpeg child per continuous run of a media file, producing raw RGBA frames at
//! exactly the size they are drawn at (ffmpeg crops and scales while decoding).

use anyhow::{anyhow, Context, Result};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

/// Frames decoded ahead of the compositor (per feed).
const PREFETCH: usize = 6;

/// Region of the source frame to show (pixels of the source) and the output size.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub crop: [u32; 4],
    pub w: u32,
    pub h: u32,
}

/// Cover-crop a source of `sw x sh` (optionally limited to `region`, normalized) to `w x h`.
pub fn cover(sw: u32, sh: u32, region: Option<[f64; 4]>, w: u32, h: u32) -> Geometry {
    let (rx, ry, rw, rh) = match region {
        Some(r) => (r[0] * sw as f64, r[1] * sh as f64, r[2] * sw as f64, r[3] * sh as f64),
        None => (0.0, 0.0, sw as f64, sh as f64),
    };
    let target = w as f64 / h as f64;
    let (cw, ch) = if rw / rh > target { (rh * target, rh) } else { (rw, rw / target) };
    let cx = rx + (rw - cw) / 2.0;
    let cy = ry + (rh - ch) / 2.0;
    let even = |v: f64| ((v / 2.0).floor() * 2.0).max(2.0) as u32;
    Geometry { crop: [cx.max(0.0) as u32, cy.max(0.0) as u32, even(cw).min(sw), even(ch).min(sh)], w, h }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeedKey {
    pub path: PathBuf,
    pub geo: Geometry,
    /// Letterbox instead of crop (contain fit): the frame is scaled to fit and padded.
    pub contain: bool,
    pub looped: bool,
    /// Decode on the VideoToolbox engines instead of the CPU.
    pub hw: bool,
}

pub struct Feed {
    pub key: FeedKey,
    child: Child,
    /// Decoded frames from the reader thread; spent buffers go back through `recycle`.
    rx: Option<Receiver<Vec<u8>>>,
    recycle: SyncSender<Vec<u8>>,
    reader: Option<std::thread::JoinHandle<()>>,
    frame: Vec<u8>,
    have: bool,
    eof: bool,
    /// Media time (ms) of the frame `next()` returns next.
    pub next_ms: f64,
    step_ms: f64,
    pub frames_read: u64,
}

impl Feed {
    pub fn open(key: FeedKey, media_ms: f64, fps: f64, max_frames: u64) -> Result<Feed> {
        let g = &key.geo;
        let vf = if key.contain {
            format!(
                "fps={fps}:round=near,crop={}:{}:{}:{},scale={}:{}:force_original_aspect_ratio=decrease:flags=bicubic,pad={}:{}:(ow-iw)/2:(oh-ih)/2:color=0x161616,format=rgba",
                g.crop[2], g.crop[3], g.crop[0], g.crop[1], g.w, g.h, g.w, g.h
            )
        } else {
            format!("fps={fps}:round=near,crop={}:{}:{}:{},scale={}:{}:flags=bicubic,format=rgba", g.crop[2], g.crop[3], g.crop[0], g.crop[1], g.w, g.h)
        };
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-nostdin", "-v", "error"]);
        if key.looped {
            cmd.args(["-stream_loop", "-1"]);
        }
        if key.hw {
            cmd.args(["-hwaccel", "videotoolbox"]);
        }
        if let Ok(v) = std::env::var("CUTROOM_THREADS") {
            cmd.args(["-threads", &v]);
        }
        cmd.args(["-ss", &format!("{:.4}", (media_ms / 1000.0).max(0.0))])
            .arg("-i")
            .arg(&key.path)
            .args(["-an", "-sn", "-vf", &vf, "-frames:v", &max_frames.max(1).to_string(), "-f", "rawvideo", "-"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().context("spawning ffmpeg (is it installed?)")?;
        let mut out = BufReader::with_capacity(1 << 22, child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?);
        let n = (g.w * g.h * 4) as usize;
        let (tx, rx) = sync_channel::<Vec<u8>>(PREFETCH);
        let (recycle, spent) = sync_channel::<Vec<u8>>(PREFETCH + 2);
        // read ahead on a thread so decoding overlaps compositing
        let reader = std::thread::spawn(move || loop {
            let mut buf = spent.try_recv().unwrap_or_else(|_| vec![0u8; n]);
            if out.read_exact(&mut buf).is_err() || tx.send(buf).is_err() {
                break;
            }
        });
        Ok(Feed { key, child, rx: Some(rx), recycle, reader: Some(reader), frame: vec![0; n], have: false, eof: false, next_ms: media_ms, step_ms: 1000.0 / fps, frames_read: 0 })
    }

    /// The next frame (RGBA). Past the end of the media the last frame is held.
    pub fn next_frame(&mut self) -> &[u8] {
        if !self.eof {
            match self.rx.as_ref().unwrap().recv() {
                Ok(buf) => {
                    let old = std::mem::replace(&mut self.frame, buf);
                    let _ = self.recycle.try_send(old);
                    self.have = true;
                    self.frames_read += 1;
                }
                Err(_) => {
                    self.eof = true;
                    if !self.have {
                        // nothing decoded (past the media end): dark frame
                        for px in self.frame.as_chunks_mut::<4>().0 {
                            px.copy_from_slice(&[0x16, 0x16, 0x16, 255]);
                        }
                    }
                }
            }
        }
        self.next_ms += self.step_ms;
        &self.frame
    }

    /// Skip ahead to media time `ms` by decoding and dropping frames, when close enough.
    /// Returns false when the caller should reopen instead (behind, or too far ahead).
    pub fn advance_to(&mut self, ms: f64, max_skip_ms: f64) -> bool {
        let d = ms - self.next_ms;
        if d < -self.step_ms / 2.0 || d > max_skip_ms {
            return false;
        }
        while ms - self.next_ms >= self.step_ms / 2.0 {
            self.next_frame();
        }
        true
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        let _ = self.child.kill();
        drop(self.rx.take());
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
    }
}

/// ffprobe: (width, height, fps, duration ms) of the first video stream.
pub fn probe(path: &Path) -> Result<(u32, u32, f64, f64)> {
    let o = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,avg_frame_rate:format=duration", "-of", "default=nw=1"])
        .arg(path)
        .output()
        .context("running ffprobe")?;
    let s = String::from_utf8_lossy(&o.stdout);
    let mut w = 0;
    let mut h = 0;
    let mut fps = 30.0;
    let mut dur = 0.0;
    for line in s.lines() {
        if let Some(v) = line.strip_prefix("width=") {
            w = v.parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("height=") {
            h = v.parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("avg_frame_rate=") {
            let mut it = v.split('/');
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                let (a, b): (f64, f64) = (a.parse().unwrap_or(30.0), b.parse().unwrap_or(1.0));
                if b > 0.0 && a > 0.0 {
                    fps = a / b;
                }
            }
        } else if let Some(v) = line.strip_prefix("duration=") {
            dur = v.parse::<f64>().unwrap_or(0.0) * 1000.0;
        }
    }
    if w == 0 || h == 0 {
        return Err(anyhow!("{}: no video stream", path.display()));
    }
    Ok((w, h, fps, dur))
}
