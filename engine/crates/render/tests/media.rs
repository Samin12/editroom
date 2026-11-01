//! Video beat fits (card, panel, frame), transparency, last-frame hold, the music bed with
//! ducking, and narration fallback, on synthetic media made with ffmpeg.

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

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const DUR: f64 = 10.0;

/// Transparent everywhere but two boxes; fully transparent pixels hold pure green (garbage a
/// straight-alpha bug would show). The big box is orange for the first second, red after; it has
/// a 12 px feathered edge. The small box sits where the camera tile goes.
fn alpha_clip(path: &Path) {
    // colour fills the whole feathered area; only fully transparent pixels are green
    let in_big = "between(X,181,971)*between(Y,637,983)";
    let feather = "clip(1-max(max(192-X,X-960),max(648-Y,Y-972))/12,0,1)*between(X,180,972)*between(Y,636,984)";
    let in_small = "between(X,1632,1824)*between(Y,432,648)";
    let r = format!("if({in_big}+{in_small},if(lt(T,1),226,230),0)");
    let g = format!("if({in_big}+{in_small},if(lt(T,1),119,20),255)");
    let b = format!("if({in_big}+{in_small},if(lt(T,1),78,30),0)");
    let a = format!("255*max({feather},{in_small})");
    ff(&[
        "-f", "lavfi", "-i", "color=c=black:s=1920x1080:r=30:d=2,format=rgba",
        "-vf", &format!("geq=r='{r}':g='{g}':b='{b}':a='{a}'"),
        "-c:v", "prores_ks", "-profile:v", "4444", "-pix_fmt", "yuva444p10le", path.to_str().unwrap(),
    ]);
}

fn fixture(d: &Path, with_mic: bool) -> Source {
    let cam = d.join("cam.mov");
    let scr = d.join("scr.mp4");
    let mic = d.join("mic.wav");
    if !cam.exists() {
        // camera: flat gray picture with its own 500 Hz audio (the narration when there is no mic)
        ff(&["-f", "lavfi", "-i", &format!("color=c=0x808080:s=1280x720:r=30:d={DUR}"), "-f", "lavfi", "-i", &format!("sine=frequency=500:sample_rate=48000:d={DUR}"), "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-shortest", cam.to_str().unwrap()]);
        ff(&["-f", "lavfi", "-i", &format!("color=c=0x2040C0:s=1920x1080:r=30:d={DUR}"), "-c:v", "libx264", "-preset", "ultrafast", scr.to_str().unwrap()]);
        // speech stand-in: 1 kHz bursts, on for 2 s of every 4
        ff(&["-f", "lavfi", "-i", &format!("aevalsrc='0.3*sin(2*PI*1000*t)*lt(mod(t\\,4)\\,2)':s=48000:d={DUR}"), mic.to_str().unwrap()]);
        ff(&["-f", "lavfi", "-i", "sine=frequency=300:sample_rate=48000:d=12", "-af", "volume=0.5", d.join("music.wav").to_str().unwrap()]);
        alpha_clip(&d.join("lt.mov"));
        ff(&["-f", "lavfi", "-i", "color=c=0xFF00FF:s=1280x720:r=30:d=3", "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", d.join("panel.mp4").to_str().unwrap()]);
    }
    let words: Vec<Word> = (0..16).map(|i| Word { text: format!("word{i}"), start_ms: 200.0 + i as f64 * 600.0, end_ms: 600.0 + i as f64 * 600.0 }).collect();
    let ms = DUR * 1000.0;
    let mut media = vec![
        Media { id: "cam".into(), path: cam, width: 1280, height: 720, fps: 30.0, duration_ms: ms },
        Media { id: "scr".into(), path: scr, width: 1920, height: 1080, fps: 30.0, duration_ms: ms },
    ];
    let mut clips = vec![
        Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: ms, media_in_ms: 0.0, take: 0 },
        Clip { track: Track::Screen, media: "scr".into(), start_ms: 0.0, dur_ms: ms, media_in_ms: 0.0, take: 0 },
    ];
    if with_mic {
        media.push(Media { id: "mic".into(), path: mic, width: 0, height: 0, fps: 0.0, duration_ms: ms });
        clips.push(Clip { track: Track::Mic, media: "mic".into(), start_ms: 0.0, dur_ms: ms, media_in_ms: 0.0, take: 0 });
    }
    Source { name: "media".into(), origin: "test".into(), duration_ms: ms, media, clips, words, scenes: vec![Scene { name: "one".into(), start_ms: 0.0, end_ms: ms }], version: 1 }
}

fn render_dir(d: &Path, edit: &str, with_mic: bool) -> (PathBuf, cutroom_core::Plan) {
    let src = fixture(d, with_mic);
    src.save(&d.join("source.json")).unwrap();
    std::fs::write(d.join("edit.yaml"), edit).unwrap();
    let loaded = spec::load(&d.join("edit.yaml")).unwrap();
    let plan = build(&loaded, &src);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let segs = segments(&plan, &src);
    let out = d.join("out.mp4");
    let opts = RenderOpts { out: out.clone(), jobs: 2, codec: Codec::VtH264, range: None, audio: true, loudnorm: false, system_gain: 0.0, no_music: false, work: None, quiet: true, hw_share: 0.0 };
    render(&plan, &src, &segs, d, &opts).unwrap();
    (out, plan)
}

fn frame(video: &Path, t_s: f64, png: &Path) -> tiny_skia::Pixmap {
    ff(&["-ss", &format!("{t_s:.3}"), "-i", video.to_str().unwrap(), "-frames:v", "1", png.to_str().unwrap()]);
    tiny_skia::Pixmap::load_png(png).unwrap()
}

fn rgb(p: &tiny_skia::Pixmap, x: f64, y: f64) -> [i32; 3] {
    let c = p.pixel((x * p.width() as f64) as u32, (y * p.height() as f64) as u32).unwrap();
    [c.red() as i32, c.green() as i32, c.blue() as i32]
}

fn near(a: [i32; 3], b: [i32; 3], tol: i32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

/// Mean amplitude of `hz` in stereo f32 `pcm` between `t0` and `t1` seconds (Goertzel, left).
fn tone(pcm: &[f32], hz: f64, t0: f64, t1: f64) -> f64 {
    let (a, b) = ((t0 * 48000.0) as usize, (t1 * 48000.0) as usize);
    let w = 2.0 * std::f64::consts::PI * hz / 48000.0;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for i in a..b {
        let s = pcm[i * 2] as f64 + 2.0 * w.cos() * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    let p = s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2;
    2.0 * p.sqrt() / (b - a) as f64
}

fn pcm(video: &Path) -> Vec<f32> {
    let o = Command::new("ffmpeg").args(["-nostdin", "-v", "error", "-i"]).arg(video).args(["-vn", "-ac", "2", "-ar", "48000", "-f", "f32le", "-"]).output().unwrap();
    o.stdout.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect()
}

const EDIT: &str = r#"
source: source.json
output: {height: 720, fps: 30, chapter_cards: off, badge: false,
         music: {src: music.wav, gain_db: -6, duck_db: 12, fade_ms: 200}}
beats:
  - {at: "o:1", hold: 4s, kind: video, src: lt.mov, fit: frame}
  - {at: "o:6", hold: 3s, kind: video, src: panel.mp4, fit: panel}
"#;

#[test]
fn video_fits_alpha_hold_and_music_bed() {
    let d = dir("cutroom-e2e-media");
    let (out, _plan) = render_dir(&d, EDIT, true);

    // before the clip: the reference picture
    let before = frame(&out, 0.5, &d.join("before.png"));
    // frame clip at 1.5 s: orange box over the panel, clear elsewhere (no green from the
    // transparent pixels), the camera tile stays on top of the clip's small box
    let p = frame(&out, 1.5, &d.join("alpha.png"));
    let orange = rgb(&p, 0.30, 0.75);
    assert!(near(orange, [226, 119, 78], 12), "box colour {orange:?}");
    for (x, y) in [(0.3, 0.2), (0.6, 0.3), (0.05, 0.5), (0.9, 0.1)] {
        assert!(near(rgb(&p, x, y), rgb(&before, x, y), 6), "clear area at ({x},{y}) changed: {:?} vs {:?}", rgb(&p, x, y), rgb(&before, x, y));
    }
    let tile = rgb(&p, 0.89, 0.5);
    assert!(near(tile, rgb(&before, 0.89, 0.5), 8), "camera tile must cover the clip: {tile:?}");
    // across the feathered edge (left side of the box): a blend of orange and the blue screen, never green
    for i in 0..24 {
        let x = (180.0 + i as f64) / 1920.0;
        let c = rgb(&p, x, 0.75);
        assert!(c[1] < c[0].max(c[2]) + 10, "green fringe at x={x:.4}: {c:?}");
    }
    // the 2 s clip does not loop: at 4 s (3 s in) it holds its last frame (red), not transparent
    let held = rgb(&frame(&out, 4.0, &d.join("held.png")), 0.30, 0.75);
    assert!(near(held, [230, 20, 30], 14), "last frame should hold: {held:?}");
    // after the beat, gone
    assert!(near(rgb(&frame(&out, 5.6, &d.join("after.png")), 0.30, 0.75), rgb(&before, 0.30, 0.75), 6));

    // panel clip: fills the panel slot, not the gap between panel and tile
    let pp = frame(&out, 7.5, &d.join("panel.png"));
    assert!(near(rgb(&pp, 0.38, 0.5), [255, 0, 255], 14), "panel {:?}", rgb(&pp, 0.38, 0.5));
    assert!(!near(rgb(&pp, 0.785, 0.5), [255, 0, 255], 60), "outside the panel {:?}", rgb(&pp, 0.785, 0.5));

    // music bed: the 300 Hz bed drops about duck_db (12 dB) under the 1 kHz speech bursts
    let a = pcm(&out);
    let talk = tone(&a, 300.0, 4.4, 5.8);
    let gap = tone(&a, 300.0, 7.3, 7.9);
    let drop = 20.0 * (gap / talk).log10();
    eprintln!("bed: talking {talk:.4}, pause {gap:.4}, duck {drop:.1} dB");
    assert!((9.0..15.0).contains(&drop), "duck {drop:.1} dB");
    assert!(tone(&a, 1000.0, 4.4, 5.8) > 0.2, "narration is there");
    // and it fades in from silence at the start
    assert!(tone(&a, 300.0, 0.0, 0.05) < 0.5 * tone(&a, 300.0, 2.6, 3.4).max(gap));
}

const EDIT_CARD: &str = r#"
source: source.json
output: {height: 720, fps: 30, chapter_cards: off, badge: false}
beats:
  - {at: "o:1", hold: 4s, kind: video, src: lt.mov, caption: "card with alpha"}
"#;

#[test]
fn card_clip_with_alpha_and_camera_narration() {
    let d = dir("cutroom-e2e-media-card");
    // no mic: the camera's own audio (500 Hz) is the narration
    let (out, _) = render_dir(&d, EDIT_CARD, false);
    let p = frame(&out, 3.5, &d.join("card.png"));
    // the card loops by default: at 3.5 s (2.5 s in) the clip is back in its orange first second
    let mut found = false;
    for y in 0..40 {
        for x in 0..60 {
            let c = rgb(&p, 0.1 + x as f64 * 0.01, 0.2 + y as f64 * 0.018);
            assert!(!(c[1] > 200 && c[0] < 60 && c[2] < 60), "green from transparent pixels at {x},{y}: {c:?}");
            found |= near(c, [226, 119, 78], 16);
        }
    }
    assert!(found, "the looping card should show the orange first second again");
    let a = pcm(&out);
    assert!(!a.is_empty(), "soundtrack without a mic");
    assert!(tone(&a, 500.0, 1.0, 3.0) > 0.05, "camera audio is the narration");
}
