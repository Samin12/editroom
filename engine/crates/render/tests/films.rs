//! Section films (studio mode): an opaque full-frame `video` beat with `fade: false` placed on a
//! frame-aligned slot must show film frame k on output frame slot0 + k, across parallel chunks,
//! keep the camera a tile over a camera-only stretch, and be checked for length.

use cutroom_core::compose::segments;
use cutroom_core::model::*;
use cutroom_core::resolve::{build, Severity};
use cutroom_core::spec::{self, LayoutMode};
use cutroom_render::{film_issues, render, window_frames, Codec, RenderOpts};
use std::path::{Path, PathBuf};
use std::process::Command;

fn ff(args: &[&str]) {
    let st = Command::new("ffmpeg").args(["-nostdin", "-v", "error", "-y"]).args(args).status().expect("ffmpeg");
    assert!(st.success(), "ffmpeg {args:?}");
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("cutroom-films-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const W: u32 = 640;
const H: u32 = 360;
const FPS: f64 = 30.0;
/// 12 bits, 40 px blocks from x = 10, rows 100..140 (clear of the camera tile on the right).
const BITS: u32 = 12;

/// A film whose frame n shows n in binary blocks (white = 1) on the canvas colour.
fn barcode_film(path: &Path, frames: u64) {
    let geq = format!("geq=lum='if(between(Y\\,100\\,140)*between(X\\,10\\,{x1})\\, if(mod(floor(N/pow(2\\,floor((X-10)/40)))\\,2)\\,235\\,16)\\, 31)':cb=128:cr=128", x1 = 10 + 40 * BITS - 1);
    ff(&["-f", "lavfi", "-i", &format!("color=c=black:s={W}x{H}:r=30"), "-vf", &geq, "-frames:v", &frames.to_string(), "-c:v", "libx264", "-preset", "ultrafast", "-crf", "10", "-g", "30", "-pix_fmt", "yuv420p", path.to_str().unwrap()]);
}

fn fixture(d: &Path) -> Source {
    let cam = d.join("cam.mp4");
    if !cam.exists() {
        ff(&["-f", "lavfi", "-i", "testsrc2=s=640x360:r=30:d=20", "-c:v", "libx264", "-preset", "ultrafast", "-g", "30", cam.to_str().unwrap()]);
    }
    let text = "We start here and then we keep talking for a while about the films we are going to place";
    let words = text.split_whitespace().enumerate().map(|(i, t)| Word { text: t.into(), start_ms: 400.0 + i as f64 * 900.0, end_ms: 400.0 + i as f64 * 900.0 + 600.0 }).collect();
    Source {
        name: "films".into(),
        origin: "test".into(),
        duration_ms: 20_000.0,
        media: vec![Media { id: "cam".into(), path: cam, width: W, height: H, fps: 30.0, duration_ms: 20_000.0 }],
        // camera only: without a film the empty panel falls back to the full camera
        clips: vec![Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: 20_000.0, media_in_ms: 0.0, take: 0 }],
        words,
        scenes: vec![Scene { name: "one".into(), start_ms: 0.0, end_ms: 20_000.0 }],
        version: 1,
    }
}

/// The beat line `cutroom slots` writes for a film covering output frames [k0, k0 + n).
fn film_beat(src: &str, k0: u64, n: u64) -> String {
    let fd = 1000.0 / FPS;
    format!("  - {{at: \"o:{:.7}\", lead: 0, hold: \"{:.4}ms\", kind: video, src: {src}, fit: frame, alpha: false, fade: false}}\n", (k0 as f64 * fd - 0.001) / 1000.0, n as f64 * fd)
}

fn decode(video: &Path) -> Vec<Option<u64>> {
    let o = Command::new("ffmpeg").args(["-nostdin", "-v", "error", "-i"]).arg(video).args(["-vf", "format=gray", "-f", "rawvideo", "-"]).output().unwrap();
    o.stdout
        .as_chunks::<{ (W * H) as usize }>()
        .0
        .iter()
        .map(|f| {
            let bits: Vec<u8> = (0..BITS).map(|b| f[(120 * W + 10 + 40 * b + 20) as usize]).collect();
            bits.iter().all(|v| *v < 60 || *v > 190).then(|| bits.iter().enumerate().filter(|(_, v)| **v > 190).map(|(b, _)| 1u64 << b).sum())
        })
        .collect()
}

#[test]
fn films_are_frame_exact_across_chunks_and_keep_the_camera_tile() {
    let d = dir();
    let src = fixture(&d);
    src.save(&d.join("source.json")).unwrap();
    std::fs::create_dir_all(d.join("films")).unwrap();
    // k0 = 94: chunks start on multiples of 60 frames, so a chunk reopens the film at frame 26 = 866.667 ms, where a seek printed to 0.1 ms rounds UP past the frame
    let (k0, n) = (94u64, 200u64);
    barcode_film(&d.join("films/a.mp4"), n);
    barcode_film(&d.join("films/short.mp4"), n - 5);
    let edit = format!("source: source.json\noutput: {{height: {H}, fps: 30, chapter_cards: off, badge: false}}\nbeats:\n{}", film_beat("films/a.mp4", k0, n));
    std::fs::write(d.join("edit.yaml"), &edit).unwrap();
    let loaded = spec::load(&d.join("edit.yaml")).unwrap();
    let plan = build(&loaded, &src);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let b = &plan.beats[0];
    assert_eq!(window_frames(b.start_ms, b.end_ms, FPS), (k0, k0 + n), "the window covers exactly the slot's frames");
    assert!(film_issues(&plan, &d).is_empty(), "{:?}", film_issues(&plan, &d));

    // over a camera-only stretch the film keeps the panel layout (camera in the tile)
    let segs = segments(&plan, &src);
    let mid = b.start_ms + 1000.0;
    let s = segs.iter().find(|s| s.out0 <= mid && mid < s.out1).unwrap();
    assert_eq!(s.mode, LayoutMode::Panel);
    assert!(!s.empty_panel);
    let before = segs.iter().find(|s| s.out0 <= 1000.0 && 1000.0 < s.out1).unwrap();
    assert_eq!(before.mode, LayoutMode::Camera, "without a film the empty panel shows the full camera");

    // render in chunks so chunk boundaries fall inside the film
    let out = d.join("out.mp4");
    let opts = RenderOpts { out: out.clone(), jobs: 5, codec: Codec::X264, range: None, audio: false, loudnorm: false, system_gain: 0.6, no_music: false, work: None, quiet: true, hw_share: 0.0 };
    render(&plan, &src, &segs, &d, &opts).unwrap();
    let got = decode(&out);
    assert_eq!(got.len(), 600);
    for k in k0..k0 + n {
        assert_eq!(got[k as usize], Some(k - k0), "output frame {k} shows film frame {}", k - k0);
    }

    // a film 5 frames short of its window is an error
    let edit = format!("source: source.json\noutput: {{height: {H}, fps: 30, chapter_cards: off, badge: false}}\nbeats:\n{}", film_beat("films/short.mp4", k0, n));
    std::fs::write(d.join("edit.yaml"), &edit).unwrap();
    let plan = build(&spec::load(&d.join("edit.yaml")).unwrap(), &src);
    let iss = film_issues(&plan, &d);
    assert!(iss.iter().any(|i| i.sev == Severity::Error && i.msg.contains("195 frames")), "{iss:?}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_film_without_alpha_keeps_the_panel_in_plan_segments() {
    // the intro has no screen recording: without a film it would be the full camera; a section
    // film (alpha left out) keeps the panel layout, camera tile on top, and `plan --segments`
    // says so
    let d = dir().join("segs");
    std::fs::create_dir_all(&d).unwrap();
    let src = fixture(&d);
    src.save(&d.join("source.json")).unwrap();
    std::fs::write(d.join("edit.yaml"), format!("source: source.json\noutput: {{height: {H}, fps: 30, chapter_cards: off}}\nbeats:\n  - {{at: \"o:0.0\", lead: 0, hold: 5s, kind: video, src: films/intro.mp4, fit: frame, loop: false}}\n")).unwrap();
    let plan = build(&spec::load(&d.join("edit.yaml")).unwrap(), &src);
    let segs = segments(&plan, &src);
    let ls = cutroom_render::layout::layout_segments(&plan, &src, &segs);
    let at = |t: f64| ls.iter().find(|s| s.start_ms <= t && t < s.end_ms).unwrap();
    assert_eq!(at(2000.0).mode, "panel");
    assert_eq!(at(2000.0).frame_video.as_deref(), Some("films/intro.mp4"));
    assert!(at(2000.0).camera_rect.is_some_and(|r| r[2] < W as i32 / 2), "the camera is a tile, not the frame");
    assert_eq!(at(7000.0).mode, "camera", "after the film the empty panel shows the full camera again");
}
