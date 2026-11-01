//! End to end: a synthetic Retina-size screen recording with FAKE secrets that appear and
//! disappear at known times, plus decoys, a blinking cursor and a ticking clock. Every secret must
//! be found with the right span (± one sample) and a rect around its text, nothing else may be
//! found, and the proposed blur beats must satisfy `cutroom check`.
//!
//! Needs ffmpeg (libx264), tesseract and the macOS system fonts; skips (with a note) without them.

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use cutroom_core::compose::segments;
use cutroom_core::lint::{check, Finding};
use cutroom_core::model::*;
use cutroom_core::resolve::{build, Severity};
use cutroom_core::spec;
use cutroom_privacy::{propose_beats, scan_with_stats, ScanOpts};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const W: usize = 2880;
const H: usize = 1800;
const FPS: usize = 10;
const SECS: usize = 12;
/// Where the screen clip starts on the source timeline.
const CLIP_AT: f64 = 5000.0;

struct Canvas {
    px: Vec<u8>,
}

impl Canvas {
    fn fill(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, v: u8) {
        for y in y0..y1.min(H) {
            self.px[y * W + x0..y * W + x1.min(W)].fill(v);
        }
    }

    /// Draw `text` at baseline (x, y); returns the ink box of chars `from..` as [x0, y0, x1, y1].
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, font: &FontRef, size: f32, x: f32, y: f32, text: &str, fg: u8, from: usize) -> [f32; 4] {
        let sf = font.as_scaled(PxScale::from(size));
        let mut pen = x;
        let mut ink = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        let mut prev = None;
        for (i, c) in text.chars().enumerate() {
            let id = sf.glyph_id(c);
            if let Some(p) = prev {
                pen += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(PxScale::from(size), point(pen, y));
            if let Some(o) = font.outline_glyph(g) {
                let bb = o.px_bounds();
                let px = &mut self.px;
                o.draw(|gx, gy, cov| {
                    let (xx, yy) = (bb.min.x as i64 + gx as i64, bb.min.y as i64 + gy as i64);
                    if xx < 0 || yy < 0 || xx >= W as i64 || yy >= H as i64 || cov <= 0.0 {
                        return;
                    }
                    let p = &mut px[yy as usize * W + xx as usize];
                    *p = (*p as f32 * (1.0 - cov) + fg as f32 * cov).round() as u8;
                    if i >= from && cov > 0.2 {
                        ink = [ink[0].min(xx as f32), ink[1].min(yy as f32), ink[2].max(xx as f32 + 1.0), ink[3].max(yy as f32 + 1.0)];
                    }
                });
            }
            pen += sf.h_advance(id);
            prev = Some(id);
        }
        ink
    }
}

/// A line of text on screen during `[t0, t1)` seconds; the part from char `from` is the secret.
struct El {
    kind: Option<&'static str>,
    mono: bool,
    size: f32,
    x: f32,
    y: f32,
    fg: u8,
    text: String,
    from: usize,
    t0: f64,
    t1: f64,
}

#[allow(clippy::too_many_arguments)]
fn el(kind: Option<&'static str>, mono: bool, size: f32, x: f32, y: f32, fg: u8, prefix: &str, secret: &str, t0: f64, t1: f64) -> El {
    El { kind, mono, size, x, y, fg, text: format!("{prefix}{secret}"), from: prefix.chars().count(), t0, t1 }
}

fn have(bin: &str, arg: &str) -> bool {
    Command::new(bin).arg(arg).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn load_font(path: &str, index: u32) -> Option<FontRef<'static>> {
    let data: &'static [u8] = Box::leak(std::fs::read(path).ok()?.into_boxed_slice());
    FontRef::try_from_slice_and_index(data, index).ok()
}

/// The scene. FAKE values only, assembled at runtime so no source literal looks like a credential.
fn scene() -> Vec<El> {
    let sk = ["sk", "-proj-", "Q7f2Lx9mZ4rT1vB8nK3wY6hJ0cD5sAeP"].concat();
    let jwt = ["ey", "JhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", ".", "eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkphbmUifQ", ".", "Xq3vN8pK2mL7wR4tY9bC1dF6gH0jS5aZ"].concat();
    let email = ["jane.doe.test", "@", "examplemail.net"].concat();
    let small = ["jdoe.private", "@", "mailbox.test"].concat();
    let t = 220u8; // terminal text
    let d = 30u8; // page text
    vec![
        // terminal (dark)
        el(None, true, 28.0, 120.0, 200.0, t, "$ git commit -m \"fix tokenizer settings\"", "", 0.0, 99.0),
        el(Some("api_key"), true, 28.0, 120.0, 260.0, t, "$ export OPENAI_API_KEY=", &sk, 1.0, 5.0),
        el(None, true, 28.0, 120.0, 320.0, t, "Request ID 3f2b8c1e  build 4567  risk-free trial", "", 0.0, 99.0),
        el(Some("token"), true, 28.0, 120.0, 380.0, t, "Authorization: Bearer ", &jwt, 6.0, 10.0),
        // page (light)
        el(None, false, 34.0, 120.0, 840.0, d, "Settings     General     Notifications     Privacy & Security", "", 0.0, 99.0),
        el(Some("email"), false, 30.0, 120.0, 920.0, d, "From: Jane Doe <", &email, 3.0, 8.0),
        el(Some("card"), false, 30.0, 120.0, 1000.0, d, "Card number    ", "4242 4242 4242 4242", 2.0, 9.0),
        el(Some("phone"), false, 30.0, 120.0, 1080.0, d, "Support line:  ", "+1 (415) 555-0134", 4.0, 7.0),
        el(None, false, 30.0, 120.0, 1160.0, d, "Order #48213 shipped on 2024-10-31 - Total: $1,234.56", "", 0.0, 99.0),
        el(None, false, 30.0, 120.0, 1240.0, d, "Version 1.2.3 - The quick brown fox jumps over the lazy dog", "", 0.0, 99.0),
        el(None, false, 30.0, 120.0, 1320.0, d, "Contact us: hello@cutroom.dev", "", 0.0, 99.0),
        el(None, false, 30.0, 120.0, 1400.0, d, "Password reset link sent - skeleton key - secret santa", "", 0.0, 99.0),
        // small status-bar text, on screen the whole clip
        el(Some("email"), false, 22.0, 120.0, 1720.0, d, "Signed in as ", &small, 0.0, 99.0),
    ]
}

struct Truth {
    kind: &'static str,
    /// Normalized ink box.
    rect: [f64; 4],
    /// Source ms.
    s0: f64,
    s1: f64,
}

/// Render and encode the video; returns the expected findings.
fn make_video(path: &Path, mono: &FontRef, sans: &FontRef) -> Vec<Truth> {
    let els = scene();
    let mut ff = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "gray", "-s", &format!("{W}x{H}"), "-r", &FPS.to_string(), "-i", "-"])
        .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "16", "-g", "5", "-keyint_min", "5", "-sc_threshold", "0", "-pix_fmt", "yuv420p"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = ff.stdin.take().unwrap();
    let mut truth: Vec<Option<[f32; 4]>> = vec![None; els.len()];
    for f in 0..FPS * SECS {
        let t = f as f64 / FPS as f64;
        let mut c = Canvas { px: vec![245u8; W * H] };
        c.fill(0, 0, W, 80, 225);
        c.text(sans, 30.0, 40.0, 52.0, "Terminal - zsh - 160x48", 40, usize::MAX);
        // a clock that ticks every second
        let clock = format!("12:03:{:02}", 10 + t as usize);
        c.text(mono, 26.0, 2650.0, 52.0, &clock, 40, usize::MAX);
        c.fill(80, 140, 2800, 700, 28);
        for (i, e) in els.iter().enumerate() {
            if t >= e.t0 && t < e.t1 {
                let ink = c.text(if e.mono { mono } else { sans }, e.size, e.x, e.y, &e.text, e.fg, e.from);
                if e.kind.is_some() {
                    truth[i] = Some(ink);
                }
            }
        }
        // blinking cursor
        c.text(mono, 28.0, 120.0, 440.0, "$ ", 220, usize::MAX);
        if (f / 5) % 2 == 0 {
            c.fill(155, 415, 171, 447, 220);
        }
        stdin.write_all(&c.px).unwrap();
    }
    drop(stdin);
    assert!(ff.wait().unwrap().success(), "ffmpeg encode failed");
    els.iter()
        .zip(truth)
        .filter_map(|(e, ink)| {
            let ink = ink?;
            let (w, h) = (W as f64, H as f64);
            let r = [ink[0] as f64 / w, ink[1] as f64 / h, (ink[2] - ink[0]) as f64 / w, (ink[3] - ink[1]) as f64 / h];
            let end = (e.t1.min(SECS as f64)) * 1000.0;
            Some(Truth { kind: e.kind.unwrap(), rect: r, s0: CLIP_AT + e.t0 * 1000.0, s1: CLIP_AT + end })
        })
        .collect()
}

fn source(video: &Path) -> Source {
    let dur = CLIP_AT + SECS as f64 * 1000.0 + 2000.0;
    let words: Vec<Word> = (0..40)
        .map(|i| {
            let t = 500.0 + i as f64 * 450.0;
            Word { text: format!("word{i}"), start_ms: t, end_ms: t + 350.0 }
        })
        .collect();
    Source {
        name: "synthetic".into(),
        duration_ms: dur,
        media: vec![
            Media { id: "cam".into(), path: "cam.mp4".into(), width: 1920, height: 1080, fps: 30.0, duration_ms: dur },
            Media { id: "scr".into(), path: video.to_path_buf(), width: W as u32, height: H as u32, fps: FPS as f64, duration_ms: SECS as f64 * 1000.0 },
        ],
        clips: vec![
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: dur, media_in_ms: 0.0, take: 0 },
            Clip { track: Track::Screen, media: "scr".into(), start_ms: CLIP_AT, dur_ms: SECS as f64 * 1000.0, media_in_ms: 0.0, take: 0 },
        ],
        words,
        ..Default::default()
    }
}

fn contains(outer: &[f64; 4], inner: &[f64; 4]) -> bool {
    let e = 0.5 / W as f64;
    outer[0] <= inner[0] + e && outer[1] <= inner[1] + e && outer[0] + outer[2] >= inner[0] + inner[2] - e && outer[1] + outer[3] >= inner[1] + inner[3] - e
}

fn overlaps(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

/// Every truth found with its span and box; nothing else found. A finding may start up to `early`
/// ms before the text appears and `late` ms after (mirrored at the end). Failure messages show
/// kinds, redacted previews and numbers only.
fn verify(label: &str, findings: &[Finding], truth: &[Truth], early: f64, late: f64) {
    for t in truth {
        let hit = findings.iter().filter(|f| f.kind == t.kind && overlaps(&f.rect, &t.rect)).collect::<Vec<_>>();
        assert!(!hit.is_empty(), "{label}: {} at {:?} not found; got {:?}", t.kind, t.rect, findings.iter().map(|f| (&f.kind, &f.preview, f.src0_ms, f.src1_ms)).collect::<Vec<_>>());
        let f0 = hit.iter().map(|f| f.src0_ms).fold(f64::MAX, f64::min);
        let f1 = hit.iter().map(|f| f.src1_ms).fold(f64::MIN, f64::max);
        assert!(f0 >= t.s0 - early && f0 <= t.s0 + late, "{label}: {} starts {f0:.0}, text appears {:.0} (-{early}/+{late})", t.kind, t.s0);
        assert!(f1 <= t.s1 + early && f1 >= t.s1 - late, "{label}: {} ends {f1:.0}, text gone {:.0} (+{early}/-{late})", t.kind, t.s1);
        assert!(hit.iter().any(|f| contains(&f.rect, &t.rect)), "{label}: {} rect {:?} does not contain the text {:?}", t.kind, hit.iter().map(|f| f.rect).collect::<Vec<_>>(), t.rect);
    }
    for f in findings {
        assert!(truth.iter().any(|t| t.kind == f.kind && overlaps(&f.rect, &t.rect)), "{label}: false positive {} {} at {:?} {}-{}", f.kind, f.preview, f.rect, f.src0_ms, f.src1_ms);
        assert!(f.preview.ends_with("...") || f.preview.contains("***@"), "{label}: preview not redacted");
    }
}

/// The proposed beats parse as a spec, cover every finding, and make `cutroom check` pass.
fn check_beats(dir: &Path, src: &Source, findings: &[Finding]) {
    let src_path = dir.join("source.json");
    src.save(&src_path).unwrap();
    let privacy_errors = |yaml: &str, name: &str| -> usize {
        let p = dir.join(name);
        std::fs::write(&p, yaml).unwrap();
        let loaded = spec::load(&p).unwrap();
        let plan = build(&loaded, src);
        let segs = segments(&plan, src);
        check(&plan, src, &segs, dir, findings).iter().filter(|i| i.sev == Severity::Error && i.msg.starts_with("privacy:")).count()
    };
    let beats = propose_beats(findings);
    let parsed: spec::Spec = serde_yaml::from_str(&beats).expect("beats parse as a spec");
    assert!(!parsed.beats.is_empty());
    assert_eq!(privacy_errors("source: source.json\n", "bare.yaml"), findings.len(), "unblurred findings are errors");
    assert_eq!(privacy_errors(&format!("source: source.json\n{beats}"), "blurred.yaml"), 0, "proposed beats cover every finding:\n{beats}");
}

#[test]
fn finds_fake_secrets_in_a_synthetic_recording() {
    if !have("ffmpeg", "-version") || !have("tesseract", "--version") {
        eprintln!("skipping: ffmpeg and tesseract are needed");
        return;
    }
    let (Some(mono), Some(sans)) = (load_font("/System/Library/Fonts/Menlo.ttc", 0), load_font("/System/Library/Fonts/Helvetica.ttc", 0)) else {
        eprintln!("skipping: system fonts not found");
        return;
    };
    let dir: PathBuf = std::env::temp_dir().join(format!("cutroom-privacy-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let video = dir.join("synthetic.mp4");
    let truth = make_video(&video, &mono, &sans);
    assert_eq!(truth.len(), 6);
    let src = source(&video);

    let mut opts = ScanOpts { chunk_samples: 4, ..Default::default() };
    opts.allow.push("hello@cutroom.dev".into());
    let frame = 1000.0 / FPS as f64;
    // keyframe sampling, edges placed on the frame: blur from a frame or two before the text
    // appears until a frame or two after it is gone, never late
    let (kf, st) = scan_with_stats(&src, &opts).unwrap();
    eprintln!("keyframes: {} findings, {} sampled, {} OCR'd ({} whole, {:.0}% of pixels), {} edges on the frame, {:.0} ms", kf.len(), st.frames_sampled, st.frames_ocr, st.frames_ocr_full, st.ocr_area * 100.0, st.edges_refined, st.wall_ms);
    assert_eq!(st.keyframe_clips, 1, "keyframes every 0.5 s: keyframe sampling");
    assert_eq!(st.frames_sampled, SECS);
    assert!(st.ocr_area < 0.9, "unchanged regions are not re-read");
    assert!(st.edges_refined >= 8);
    verify("keyframes", &kf, &truth, 2.5 * frame, 0.5);

    // every frame decoded, the fps filter samples, plain half-sample padding: ± one sample
    let opts2 = ScanOpts { keyframes: false, refine_edges: false, ..opts.clone() };
    let (all, st2) = scan_with_stats(&src, &opts2).unwrap();
    eprintln!("all frames: {} findings, {} sampled, {} OCR'd, {:.0} ms", all.len(), st2.frames_sampled, st2.frames_ocr, st2.wall_ms);
    assert_eq!(st2.keyframe_clips, 0);
    verify("all frames", &all, &truth, opts.every_ms, opts.every_ms);

    check_beats(&dir, &src, &kf);
    let _ = std::fs::remove_dir_all(&dir);
}
