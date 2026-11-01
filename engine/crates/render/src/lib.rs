//! cutroom-render: draws a [`Plan`] to a finished video.
//!
//! Pipeline per worker: ffmpeg decoders (crop+scale while decoding) -> Rust compositor (canvas,
//! panel, camera tile, stage beats, overlays, privacy mosaic, badge, cards) -> ffmpeg encoder.
//! The output is split into `jobs` chunks rendered in parallel and joined without re-encoding;
//! the soundtrack is built once from the same time map.

pub mod audio;
pub mod blit;
pub mod encode;
pub mod feed;
pub mod frame;
pub mod outputs;

use anyhow::{anyhow, Context, Result};
use cutroom_core::compose::Seg;
use cutroom_core::model::{Source, Track};
use cutroom_core::spec::BeatKind;
use cutroom_core::Plan;
pub use encode::Codec;
use frame::{Compositor, Ctx, Stats};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct RenderOpts {
    pub out: PathBuf,
    pub jobs: usize,
    pub codec: Codec,
    /// Output range (ms); None = everything.
    pub range: Option<(f64, f64)>,
    pub audio: bool,
    pub loudnorm: bool,
    pub system_gain: f32,
    /// Scratch folder for chunks (default: next to the output).
    pub work: Option<PathBuf>,
    pub quiet: bool,
    /// Share of workers that decode big screen recordings on the hardware engines (0..1).
    pub hw_share: f64,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Report {
    pub frames: u64,
    pub output_s: f64,
    pub wall_s: f64,
    pub fps: f64,
    pub realtime_x: f64,
    pub jobs: usize,
    pub compose_ms_per_frame: f64,
    pub decode_wait_ms_per_frame: f64,
    pub feeds_opened: u64,
    pub audio_s: f64,
}

/// Probe every video media and beat file used, once.
pub fn dims(plan: &Plan, src: &Source, dir: &Path) -> Result<HashMap<String, (u32, u32)>> {
    let mut out = HashMap::new();
    for m in &src.media {
        let used = src.clips.iter().any(|c| c.media == m.id && c.track.is_video());
        if !used {
            continue;
        }
        if m.width > 0 && m.height > 0 {
            out.insert(m.id.clone(), (m.width, m.height));
        } else {
            let (w, h, _, _) = feed::probe(&m.path)?;
            out.insert(m.id.clone(), (w, h));
        }
    }
    for b in &plan.beats {
        if let BeatKind::Video { src: f, .. } = &b.kind {
            let p = dir.join(f);
            let (w, h, _, _) = feed::probe(&p)?;
            out.insert(p.display().to_string(), (w, h));
        }
    }
    Ok(out)
}

/// Draw one output frame (ms) to a pixmap.
pub fn still(plan: &Plan, src: &Source, segs: &[Seg], dir: &Path, t_ms: f64) -> Result<cutroom_gfx::Pixmap> {
    let ctx = Ctx { plan, src, segs, dir, dims: dims(plan, src, dir)? };
    let mut c = Compositor::new(&ctx)?;
    let k = (t_ms * plan.fps / 1000.0).floor().max(0.0) as u64;
    c.frame(k)?;
    Ok(c.pm.clone())
}

/// Several stills tiled into one image (cols x rows), each labelled by the caller's order.
pub fn sheet(plan: &Plan, src: &Source, segs: &[Seg], dir: &Path, times: &[f64], cols: u32, thumb_w: u32) -> Result<cutroom_gfx::Pixmap> {
    use rayon::prelude::*;
    let d = dims(plan, src, dir)?;
    let ctx = Ctx { plan, src, segs, dir, dims: d };
    let thumb_h = thumb_w * plan.height / plan.width;
    let frames: Vec<Vec<u8>> = times
        .par_iter()
        .map(|&t| -> Result<Vec<u8>> {
            let mut c = Compositor::new(&ctx)?;
            c.frame((t * plan.fps / 1000.0).floor().max(0.0) as u64)?;
            let mut out = vec![];
            blit::resample(c.pm.data(), plan.width, plan.height, [0.0, 0.0, plan.width as f32, plan.height as f32], thumb_w, thumb_h, &mut out);
            Ok(out)
        })
        .collect::<Result<_>>()?;
    let rows = (times.len() as u32).div_ceil(cols.max(1));
    let gap = 6;
    let mut pm = cutroom_gfx::Pixmap::new(cols * (thumb_w + gap) + gap, rows * (thumb_h + gap) + gap).ok_or_else(|| anyhow!("sheet too large"))?;
    pm.fill(tiny_skia::Color::from_rgba8(40, 40, 40, 255));
    for (i, f) in frames.iter().enumerate() {
        let x = gap + (i as u32 % cols) * (thumb_w + gap);
        let y = gap + (i as u32 / cols) * (thumb_h + gap);
        blit::blit_rounded(&mut pm, x as i32, y as i32, f, thumb_w, thumb_h, None);
    }
    Ok(pm)
}

/// Split [f0, f1) into `n` chunks of whole GOPs (2 s) so every chunk starts on a keyframe.
fn chunks(f0: u64, f1: u64, n: usize, fps: f64) -> Vec<(u64, u64)> {
    let gop = (fps * 2.0).round() as u64;
    let total = f1 - f0;
    let per = (total.div_ceil(n.max(1) as u64)).div_ceil(gop) * gop;
    let mut out = vec![];
    let mut a = f0;
    while a < f1 {
        let b = (a + per.max(gop)).min(f1);
        out.push((a, b));
        a = b;
    }
    out
}

pub fn render(plan: &Plan, src: &Source, segs: &[Seg], dir: &Path, opts: &RenderOpts) -> Result<Report> {
    let started = Instant::now();
    let fps = plan.fps;
    let (r0, r1) = opts.range.unwrap_or((0.0, plan.duration_ms()));
    let f0 = (r0 * fps / 1000.0).round() as u64;
    let f1 = ((r1.min(plan.duration_ms())) * fps / 1000.0).round() as u64;
    if f1 <= f0 {
        return Err(anyhow!("nothing to render"));
    }
    let work = opts.work.clone().unwrap_or_else(|| {
        let mut p = opts.out.clone();
        p.set_extension("parts");
        p
    });
    std::fs::create_dir_all(&work)?;
    let ctx = Ctx { plan, src, segs, dir, dims: dims(plan, src, dir)? };
    let parts = chunks(f0, f1, opts.jobs, fps);
    let total_frames = f1 - f0;
    let done = std::sync::atomic::AtomicU64::new(0);

    // soundtrack in parallel with the picture
    let audio_path = work.join("audio.m4a");
    let range_ms = (f0 as f64 * 1000.0 / fps, f1 as f64 * 1000.0 / fps);
    let has_mic = src.clips.iter().any(|c| c.track == Track::Mic);
    let (stats, audio_res): (Vec<Result<Stats>>, Result<f64>) = std::thread::scope(|s| {
        let audio = s.spawn(|| -> Result<f64> {
            if !opts.audio || !has_mic {
                return Ok(0.0);
            }
            let t = Instant::now();
            audio::render(plan, src, range_ms, &audio_path, opts.loudnorm, opts.system_gain)?;
            Ok(t.elapsed().as_secs_f64())
        });
        let handles: Vec<_> = parts
            .iter()
            .enumerate()
            .map(|(i, &(a, b))| {
                let ctx = &ctx;
                let work = &work;
                let done = &done;
                s.spawn(move || -> Result<Stats> {
                    let mut comp = Compositor::new(ctx)?;
                    comp.hw = opts.hw_share > 0.0 && ((i as f64 + 0.5) * opts.hw_share).floor() < (((i + 1) as f64 + 0.5) * opts.hw_share).floor();
                    let mut enc = encode::Encoder::open(&work.join(format!("chunk{i:03}.mp4")), plan.width, plan.height, fps, opts.codec)?;
                    for k in a..b {
                        comp.frame(k).with_context(|| format!("frame {k}"))?;
                        enc.write(comp.pm.data())?;
                        let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        if !opts.quiet && d.is_multiple_of(fps as u64 * 30) {
                            let el = started.elapsed().as_secs_f64();
                            eprintln!("  {:5.1}%  {:.0} fps", d as f64 * 100.0 / total_frames as f64, d as f64 / el);
                        }
                    }
                    enc.finish()?;
                    Ok(comp.stats)
                })
            })
            .collect();
        let st = handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(anyhow!("worker panicked")))).collect();
        (st, audio.join().unwrap_or_else(|_| Err(anyhow!("audio panicked"))))
    });
    let mut agg = Stats::default();
    for s in stats {
        let s = s?;
        agg.frames += s.frames;
        agg.compose_ns += s.compose_ns;
        agg.decode_ns += s.decode_ns;
        agg.feeds_opened += s.feeds_opened;
    }
    let audio_s = audio_res?;

    // join chunks (stream copy) and mux the soundtrack
    let list = work.join("chunks.txt");
    let mut txt = String::new();
    for i in 0..parts.len() {
        txt.push_str(&format!("file '{}'\n", work.join(format!("chunk{i:03}.mp4")).display()));
    }
    std::fs::write(&list, txt)?;
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-nostdin", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i"]).arg(&list);
    if opts.audio && has_mic {
        cmd.arg("-i").arg(&audio_path).args(["-map", "0:v", "-map", "1:a"]);
    }
    cmd.args(["-c", "copy", "-movflags", "+faststart"]).arg(&opts.out);
    let o = cmd.output().context("joining chunks")?;
    if !o.status.success() {
        return Err(anyhow!("join failed: {}", String::from_utf8_lossy(&o.stderr)));
    }
    let _ = std::fs::remove_dir_all(&work);
    let wall = started.elapsed().as_secs_f64();
    let output_s = total_frames as f64 / fps;
    Ok(Report {
        frames: total_frames,
        output_s,
        wall_s: wall,
        fps: total_frames as f64 / wall,
        realtime_x: output_s / wall,
        jobs: parts.len(),
        compose_ms_per_frame: agg.compose_ns as f64 / 1e6 / agg.frames.max(1) as f64,
        decode_wait_ms_per_frame: agg.decode_ns as f64 / 1e6 / agg.frames.max(1) as f64,
        feeds_opened: agg.feeds_opened,
        audio_s,
    })
}

/// PNG writer for stills and sheets.
pub fn save_png(pm: &cutroom_gfx::Pixmap, path: &Path) -> Result<()> {
    pm.save_png(path).map_err(|e| anyhow!("{}: {e}", path.display()))
}
