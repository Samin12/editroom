//! Throughput check on a real screen recording. Prints counts only, never what was found.
//!
//! ```sh
//! cargo run --release -p cutroom-privacy --example scan_media -- <media.mov> [seconds] [--from S] [--every MS] [--threads N] [--scale X] [--json out.json]
//! ```
//!
//! `--json` writes the findings (redacted previews, times, rects) for a spot check.

use cutroom_core::model::{Clip, Media, Source, Track};
use cutroom_privacy::{scan_with_stats, ScanOpts};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let media = PathBuf::from(args.next().ok_or_else(|| anyhow::anyhow!("usage: scan_media <media.mov> [seconds] [--from S] [--every MS] [--threads N] [--scale X] [--json out.json]"))?);
    let mut secs: Option<f64> = None;
    let mut from = 0.0;
    let mut opts = ScanOpts::default();
    let mut json: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        if a == "--json" {
            json = args.next().map(PathBuf::from);
            continue;
        }
        let mut val = || args.next().and_then(|v| v.parse::<f64>().ok()).ok_or_else(|| anyhow::anyhow!("{a} needs a number"));
        match a.as_str() {
            "--from" => from = val()?,
            "--every" => opts.every_ms = val()?,
            "--threads" => opts.threads = val()? as usize,
            "--scale" => opts.ocr_scale = Some(val()?),
            s => secs = Some(s.parse()?),
        }
    }
    let out = std::process::Command::new("ffprobe").args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"]).arg(&media).output()?;
    let total: f64 = String::from_utf8_lossy(&out.stdout).trim().parse()?;
    let len = secs.unwrap_or(total - from).min(total - from) * 1000.0;
    let src = Source {
        name: "scan_media".into(),
        duration_ms: len,
        media: vec![Media { id: "screen".into(), path: media.clone(), width: 0, height: 0, fps: 0.0, duration_ms: total * 1000.0 }],
        clips: vec![Clip { track: Track::Screen, media: "screen".into(), start_ms: 0.0, dur_ms: len, media_in_ms: from * 1000.0, take: 0 }],
        ..Default::default()
    };
    let (findings, st) = scan_with_stats(&src, &opts)?;
    if let Some(p) = &json {
        cutroom_privacy::write_json(p, &findings)?;
    }
    let wall_min = st.wall_ms / 60000.0;
    println!("media scanned   {:.1} min (from {:.0}s)", st.media_ms / 60000.0, from);
    println!("findings        {}", findings.len());
    for (k, n) in &st.per_kind {
        println!("  {k:<12}  {n}");
    }
    println!("frames sampled  {}", st.frames_sampled);
    println!("frames OCR'd    {} ({} whole; {:.0}% of sampled pixels)", st.frames_ocr, st.frames_ocr_full, st.ocr_area * 100.0);
    println!("keyframe mode   {}", st.keyframe_clips > 0);
    println!("workers         {}", st.workers);
    println!("time (summed)   decode+wait {:.0}s  ocr {:.0}s  detect {:.1}s", st.decode_ms / 1000.0, st.ocr_ms / 1000.0, st.detect_ms / 1000.0);
    println!("edges on frame  {} ({:.1}s wall)", st.edges_refined, st.refine_ms / 1000.0);
    println!("wall            {:.1}s", st.wall_ms / 1000.0);
    println!("throughput      {:.1} source min / wall min", st.media_ms / 60000.0 / wall_min.max(1e-9));
    Ok(())
}
