//! Encoder sink: raw RGBA frames into an ffmpeg child.

use anyhow::{anyhow, Context, Result};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    /// Apple VideoToolbox H.264 (hardware; fastest).
    VtH264,
    /// Apple VideoToolbox HEVC (hardware; smaller files).
    VtHevc,
    /// libx264 (software; best quality per bit, uses CPU the decoders also want).
    X264,
}

impl std::str::FromStr for Codec {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Codec> {
        match s {
            "vt" | "h264" | "vt-h264" => Ok(Codec::VtH264),
            "hevc" | "vt-hevc" => Ok(Codec::VtHevc),
            "x264" | "libx264" => Ok(Codec::X264),
            _ => Err(anyhow!("unknown codec {s:?} (h264, hevc, x264)")),
        }
    }
}

impl Codec {
    fn args(self, w: u32, h: u32, fps: f64) -> Vec<String> {
        // ~0.25 bit per pixel per frame: 1080p30 = 15.5 Mbit/s, 2160p30 = 62 Mbit/s
        let bitrate = (w as f64 * h as f64 * fps * 0.25 / 1000.0).round() as u64;
        let gop = (fps * 2.0).round() as u64;
        let mut a: Vec<String> = match self {
            Codec::VtH264 => vec!["-c:v", "h264_videotoolbox", "-profile:v", "high", "-allow_sw", "1"].into_iter().map(String::from).collect(),
            Codec::VtHevc => vec!["-c:v", "hevc_videotoolbox", "-tag:v", "hvc1", "-allow_sw", "1"].into_iter().map(String::from).collect(),
            Codec::X264 => vec!["-c:v", "libx264", "-preset", "veryfast", "-crf", "17"].into_iter().map(String::from).collect(),
        };
        if self != Codec::X264 {
            a.extend(["-b:v".into(), format!("{bitrate}k"), "-maxrate".into(), format!("{}k", bitrate * 3 / 2), "-bufsize".into(), format!("{}k", bitrate * 2)]);
        }
        a.extend(["-g".into(), gop.to_string(), "-pix_fmt".into(), "yuv420p".into()]);
        a
    }
}

/// Frames queued for the writer thread, so encoding overlaps compositing.
const QUEUE: usize = 4;

pub struct Encoder {
    child: Child,
    tx: Option<SyncSender<Vec<u8>>>,
    spent: Receiver<Vec<u8>>,
    writer: Option<std::thread::JoinHandle<std::io::Result<()>>>,
}

impl Encoder {
    pub fn open(path: &Path, w: u32, h: u32, fps: f64, codec: Codec) -> Result<Encoder> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-nostdin", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-r", &format!("{fps}"), "-i", "-"])
            .args(["-vf", "scale=out_color_matrix=bt709:out_range=tv"])
            .args(codec.args(w, h, fps))
            .args(["-color_primaries", "bt709", "-color_trc", "bt709", "-colorspace", "bt709", "-an"])
            .arg(path)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().context("spawning ffmpeg encoder")?;
        let mut stdin = BufWriter::with_capacity(1 << 23, child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?);
        let (tx, rx) = sync_channel::<Vec<u8>>(QUEUE);
        let (back, spent) = sync_channel::<Vec<u8>>(QUEUE + 2);
        let writer = std::thread::spawn(move || -> std::io::Result<()> {
            for buf in rx {
                stdin.write_all(&buf)?;
                let _ = back.try_send(buf);
            }
            stdin.flush()
        });
        Ok(Encoder { child, tx: Some(tx), spent, writer: Some(writer) })
    }

    pub fn write(&mut self, rgba: &[u8]) -> Result<()> {
        let mut buf = self.spent.try_recv().unwrap_or_default();
        buf.clear();
        buf.extend_from_slice(rgba);
        self.tx.as_ref().unwrap().send(buf).map_err(|_| anyhow!("encoder closed early (see ffmpeg error)"))?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        drop(self.tx.take());
        if let Some(w) = self.writer.take() {
            w.join().map_err(|_| anyhow!("encoder writer panicked"))?.context("writing frames to the encoder")?;
        }
        let mut err = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        let st = self.child.wait()?;
        if !st.success() {
            return Err(anyhow!("encoder failed: {err}"));
        }
        Ok(())
    }
}
