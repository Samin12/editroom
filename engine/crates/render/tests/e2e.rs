//! End to end on synthetic media: generate a camera, a screen recording and a mic track with
//! ffmpeg, cut a retake, add chapters and every kind of beat, render in parallel chunks, then
//! check the file (duration, frames, audio) and pixels (the privacy mosaic, layouts).

use cutroom_core::compose::segments;
use cutroom_core::model::*;
use cutroom_core::resolve::build;
use cutroom_core::spec;
use cutroom_render::{render, Codec, RenderOpts};
use std::path::{Path, PathBuf};
use std::process::Command;

fn ff(args: &[&str]) {
    let st = Command::new("ffmpeg").args(["-nostdin", "-v", "error", "-y"]).args(args).status().expect("ffmpeg");
    assert!(st.success(), "ffmpeg {args:?}");
}

fn probe(path: &Path, entries: &str, stream: &str) -> String {
    let o = Command::new("ffprobe").args(["-v", "error", "-select_streams", stream, "-count_frames", "-show_entries", entries, "-of", "default=nw=1:nk=1"]).arg(path).output().unwrap();
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn fixture(dir: &Path) -> Source {
    let dur = 30.0;
    let cam = dir.join("cam.mp4");
    let scr = dir.join("scr.mp4");
    let mic = dir.join("mic.wav");
    if !cam.exists() {
        ff(&["-f", "lavfi", "-i", &format!("testsrc2=s=1920x1080:r=30:d={dur}"), "-c:v", "libx264", "-preset", "ultrafast", "-g", "30", cam.to_str().unwrap()]);
        // screen: white page with a red box at a known place (x 0.10-0.30, y 0.10-0.20) holding "secret" stripes
        ff(&[
            "-f", "lavfi", "-i", &format!("color=c=white:s=2560x1440:r=60:d={dur}"),
            "-vf", "drawbox=x=256:y=144:w=512:h=144:color=red:t=fill,drawbox=x=256:y=200:w=512:h=10:color=black:t=fill,drawgrid=w=64:h=64:t=1:c=gray@0.5",
            "-c:v", "libx264", "-preset", "ultrafast", "-g", "60", scr.to_str().unwrap(),
        ]);
        ff(&["-f", "lavfi", "-i", &format!("sine=frequency=220:sample_rate=48000:d={dur}"), mic.to_str().unwrap()]);
    }
    let text = "So today we connect Claude to Poppy. Um so today we connect Claude to Poppy. First open the board and click new. Then paste your key here and save it. Now the board can read your notes. That is the whole setup and it takes two minutes. Next we make the first video together.";
    let words: Vec<Word> = text
        .split_whitespace()
        .enumerate()
        .map(|(i, t)| Word { text: t.into(), start_ms: 500.0 + i as f64 * 520.0, end_ms: 500.0 + i as f64 * 520.0 + 380.0 })
        .collect();
    let d = dur * 1000.0;
    Source {
        name: "synthetic".into(),
        origin: "test".into(),
        duration_ms: d,
        media: vec![
            Media { id: "cam".into(), path: cam, width: 1920, height: 1080, fps: 30.0, duration_ms: d },
            Media { id: "scr".into(), path: scr, width: 2560, height: 1440, fps: 60.0, duration_ms: d },
            Media { id: "mic".into(), path: mic, width: 0, height: 0, fps: 0.0, duration_ms: d },
        ],
        clips: vec![
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: d, media_in_ms: 0.0, take: 0 },
            // the screen starts 9 s in: before that the panel is empty -> camera fallback
            Clip { track: Track::Screen, media: "scr".into(), start_ms: 9000.0, dur_ms: d - 9000.0, media_in_ms: 9000.0, take: 0 },
            Clip { track: Track::Mic, media: "mic".into(), start_ms: 0.0, dur_ms: d, media_in_ms: 0.0, take: 0 },
        ],
        words,
        scenes: vec![Scene { name: "one".into(), start_ms: 0.0, end_ms: d }],
        version: 1,
    }
}

const EDIT: &str = r#"
source: source.json
output: {height: 720, fps: 30, chapter_cards: insert, card_ms: 1500, badge: true}
cuts:
  - {from: w0, to: w8, why: retake}
chapters:
  - {at: "today we connect", title: Connect Claude}
  - {at: "now the board", title: Read your notes}
beats:
  - {at: "connect claude to poppy", until: "poppy.", kind: title, kicker: "STEP 1", text: "Connect Claude + Poppy"}
  - {at: "first open the board", kind: ring, rect: [0.1, 0.1, 0.2, 0.1], hold: 2s}
  - {at: "paste your key", until: "save it.", kind: blur, rects: [[0.09, 0.09, 0.22, 0.12]], lead: 300}
  - {at: "now the board", kind: zoom, rect: [0.05, 0.05, 0.4, 0.3], hold: 3s}
  - {at: "whole setup", kind: callout, text: "2 minutes", hold: 2s}
  - {at: "next we make", kind: layout, mode: camera, hold: 2s}
"#;

fn out_dir() -> PathBuf {
    let d = std::env::temp_dir().join("cutroom-e2e");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn frame_png(video: &Path, t_s: f64, out: &Path) -> tiny_skia::Pixmap {
    ff(&["-ss", &format!("{t_s:.3}"), "-i", video.to_str().unwrap(), "-frames:v", "1", out.to_str().unwrap()]);
    tiny_skia::Pixmap::load_png(out).unwrap()
}

#[test]
fn renders_synthetic_edit() {
    let dir = out_dir();
    let src = fixture(&dir);
    src.save(&dir.join("source.json")).unwrap();
    std::fs::write(dir.join("edit.yaml"), EDIT).unwrap();
    let loaded = spec::load(&dir.join("edit.yaml")).unwrap();
    let plan = build(&loaded, &src);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let segs = segments(&plan, &src);
    let out = dir.join("out.mp4");
    let opts = RenderOpts { out: out.clone(), jobs: 3, codec: Codec::VtH264, range: None, audio: true, loudnorm: false, system_gain: 0.0, work: None, quiet: true, hw_share: 0.0 };
    let r = render(&plan, &src, &segs, &dir, &opts).unwrap();
    eprintln!("{r:?}");

    // duration and frame count match the plan
    let frames: u64 = probe(&out, "stream=nb_read_frames", "v:0").parse().unwrap();
    assert_eq!(frames, plan.map.frames());
    let adur: f64 = probe(&out, "stream=duration", "a:0").parse().unwrap();
    assert!((adur * 1000.0 - plan.duration_ms()).abs() < 60.0, "audio {adur}s vs {}ms", plan.duration_ms());

    // the privacy mosaic: inside the blur window the stripe region is flattened (low variance)
    let blur = plan.beats.iter().find(|b| b.kind.name() == "blur").unwrap();
    let t = (blur.start_ms + blur.end_ms) / 2000.0;
    let pm = frame_png(&out, t, &dir.join("blur.png"));
    // the panel at 720p: x 7..1186, y 13..707; screen cover-cropped. Sample a column across the black stripe.
    let zoom = plan.beats.iter().find(|b| b.kind.name() == "zoom").unwrap();
    assert!(blur.end_ms < zoom.start_ms, "fixture: blur and zoom must not overlap");
    let ring = plan.beats.iter().find(|b| b.kind.name() == "ring").unwrap();
    let pm_clear = frame_png(&out, (ring.start_ms + ring.end_ms) / 2000.0, &dir.join("clear.png"));
    // darkest pixel inside the red box (screen x 300..700, y 150..280): the black stripe when clear
    let darkest = |p: &tiny_skia::Pixmap| {
        let (pw, ph) = (((0.7676 * 1280.0f64).round() as i32 - (0.01 * 1280.0f64).round() as i32) & !1, (((0.0178 + 0.9644) * 720.0f64).round() as i32 - (0.0178 * 720.0f64).round() as i32) & !1);
        let px0 = (0.01 * 1280.0f64).round() as f64;
        let py0 = (0.0178 * 720.0f64).round() as f64;
        let sc = (pw as f64 / 2560.0).max(ph as f64 / 1440.0);
        let cx = (2560.0 * sc - pw as f64) / 2.0;
        let cy = (1440.0 * sc - ph as f64) / 2.0;
        let mut m = u32::MAX;
        for sy in (150..280).step_by(2) {
            for sx in (300..700).step_by(8) {
                let c = p.pixel((px0 + sx as f64 * sc - cx) as u32, (py0 + sy as f64 * sc - cy) as u32).unwrap();
                m = m.min(c.red() as u32 + c.green() as u32 + c.blue() as u32);
            }
        }
        m
    };
    let (d_clear, d_blur) = (darkest(&pm_clear), darkest(&pm));
    eprintln!("darkest in the box: clear {d_clear}, blurred {d_blur}");
    assert!(d_clear < 60, "fixture: the stripe should be visible when clear ({d_clear})");
    assert!(d_blur > 120, "the mosaic must wash out the stripe ({d_blur})");

    // before the screen starts (source < 9 s, kept part) the panel is empty -> full camera layout:
    // testsrc2 fills the whole frame, so the top-left corner is not the dark canvas
    let early = plan.map.to_out(6000.0) / 1000.0;
    let pe = frame_png(&out, early, &dir.join("early.png"));
    let c = pe.pixel(3, 3).unwrap();
    assert!(c.red() as u32 + c.green() as u32 + c.blue() as u32 > 60, "expected the camera full frame, got canvas");

    // chapters and captions side outputs
    let ch = cutroom_render::outputs::chapters(&plan);
    assert!(ch.starts_with("0:00 "), "{ch}");
    assert!(cutroom_render::outputs::srt(&plan).contains("-->"));
}
