use cutroom_core::compose::segments;
use cutroom_core::lint::{check, Finding};
use cutroom_core::model::*;
use cutroom_core::resolve::{build, Severity};
use cutroom_core::spec;
use std::io::Write;

fn source() -> Source {
    let text = "So today we connect Claude to Poppy. Um so today we connect Claude to Poppy. First open the board. Then paste your key here. That is it.";
    let words: Vec<Word> = text
        .split_whitespace()
        .enumerate()
        .map(|(i, t)| Word { text: t.into(), start_ms: 1000.0 + i as f64 * 400.0, end_ms: 1000.0 + i as f64 * 400.0 + 300.0 })
        .collect();
    let dur = 1000.0 + words.len() as f64 * 400.0 + 1000.0;
    Source {
        name: "t".into(),
        duration_ms: dur,
        media: vec![
            Media { id: "cam".into(), path: "cam.mp4".into(), width: 1920, height: 1080, fps: 30.0, duration_ms: dur },
            Media { id: "scr".into(), path: "scr.mp4".into(), width: 3000, height: 2000, fps: 30.0, duration_ms: dur },
        ],
        clips: vec![
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: dur, media_in_ms: 0.0, take: 0 },
            // screen only covers the second half
            Clip { track: Track::Screen, media: "scr".into(), start_ms: dur / 2.0, dur_ms: dur / 2.0, media_in_ms: 0.0, take: 0 },
        ],
        words,
        ..Default::default()
    }
}

fn plan_for(yaml: &str) -> (cutroom_core::Plan, Source, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("cutroom-core-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("edit-{}.yaml", yaml.len()));
    std::fs::File::create(&p).unwrap().write_all(yaml.as_bytes()).unwrap();
    let loaded = spec::load(&p).unwrap();
    let src = source();
    (build(&loaded, &src), src, dir)
}

#[test]
fn retake_cut_keeps_beats_on_words() {
    let (plan, src, dir) = plan_for(
        r#"
cuts:
  - {from: w0, to: w8, why: retake}
chapters:
  - {at: "first open the board", title: Open the board}
beats:
  - {at: "connect claude to poppy", until: "poppy.", kind: title, text: "Connect Claude + Poppy"}
  - {at: "paste your key", kind: blur, rects: [[0.1, 0.1, 0.3, 0.05]], hold: 2s}
"#,
    );
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    // the phrase resolves to the KEPT second take (w12), not the cut first one (w4)
    let t = &plan.beats[0];
    let w11 = plan.words.iter().find(|w| w.id == 11).unwrap();
    assert!((t.start_ms - (w11.start_ms - 130.0)).abs() < 1e-6);
    // the retake is gone: output is shorter by the cut, plus one 1.8s card
    assert!(plan.duration_ms() < src.duration_ms + 1800.0 - 3000.0);
    let segs = segments(&plan, &src);
    // first half has no screen: the panel would be empty except where the title beat fills it
    let issues = check(&plan, &src, &segs, &dir, &[]);
    assert!(issues.iter().any(|i| i.msg.contains("empty panel")), "{issues:?}");
    // a chapter card gap exists and has no source
    assert!(segs.iter().any(|s| s.card.is_some() && s.src0.is_none()));
}

#[test]
fn privacy_finding_must_be_blurred() {
    let (plan, src, dir) = plan_for("beats: []\n");
    let segs = segments(&plan, &src);
    let f = Finding { src0_ms: src.duration_ms * 0.6, src1_ms: src.duration_ms * 0.7, rect: [0.1, 0.1, 0.3, 0.05], kind: "api_key".into(), preview: "sk-ab1...".into() };
    let issues = check(&plan, &src, &segs, &dir, &[f.clone()]);
    assert!(issues.iter().any(|i| i.sev == Severity::Error && i.msg.starts_with("privacy")), "{issues:?}");
    // a blur over the window clears it
    let o0 = plan.map.to_out(f.src0_ms) / 1000.0 - 0.2;
    let (plan2, src2, dir2) = plan_for(&format!("beats:\n  - {{at: \"o:{o0}\", kind: blur, rects: [[0.09, 0.09, 0.33, 0.07]], hold: 6s, lead: 0}}\n"));
    let segs2 = segments(&plan2, &src2);
    let issues2 = check(&plan2, &src2, &segs2, &dir2, &[f]);
    assert!(!issues2.iter().any(|i| i.msg.starts_with("privacy")), "{issues2:?}");
}

#[test]
fn typos_are_rejected() {
    let r: Result<spec::Spec, _> = serde_yaml::from_str("beats:\n  - {at: w1, kind: title, txt: hi}\n");
    assert!(r.is_err());
    let r: Result<spec::Spec, _> = serde_yaml::from_str("beats:\n  - {at: w1, kind: titel, text: hi}\n");
    assert!(r.is_err());
}

#[test]
fn scene_layers_follow_words_through_cuts() {
    let (plan, _src, dir) = plan_for(
        r#"
cuts:
  - {from: w0, to: w8, why: retake}
beats:
  - at: "connect claude to poppy"
    until: "the board."
    kind: scene
    layers:
      - {type: text, text: "Claude + Poppy", x: 0.1, y: 0.1, in: {at: "poppy"}}
      - {id: a, type: box, x: 0.1, y: 0.4, w: 0.3, h: 0.2, in: {at: w16}, keys: [{t: "+1s", scale: 1.1}]}
      - {type: arrow, from: a, to: [0.9, 0.5], in: {at: "open"}}
      - {type: text, text: "late", x: 0.1, y: 0.8, in: {at: "that is it."}}
      - {type: image, src: nope.png, x: 0.5, y: 0.1, w: 0.3, h: 0.3}
"#,
    );
    let b = &plan.beats[0];
    let cutroom_core::spec::BeatKind::Scene { layers, .. } = &b.kind else { panic!() };
    let t = cutroom_core::scene::unflatten(layers, &b.items_ms);
    let out = |id: usize| plan.words.iter().find(|w| w.id == id).unwrap().start_ms - 130.0;
    // "poppy" matches the kept take said during the beat (w14), not the cut one (w6)
    assert!((t[0].enter.unwrap() - out(14)).abs() < 1e-6, "{t:?}");
    assert!((t[1].enter.unwrap() - out(16)).abs() < 1e-6);
    assert!((t[1].keys[0] - (b.start_ms + 1000.0)).abs() < 1e-6);
    assert!((t[2].enter.unwrap() - out(16)).abs() < 1e-6);
    // said after the beat: clamped to its end, with a warning
    assert!((t[3].enter.unwrap() - b.end_ms).abs() < 1e-6);
    assert!(plan.issues.iter().any(|i| i.sev == Severity::Warn && i.msg.contains("layer 3") && i.msg.contains("after the beat")), "{:?}", plan.issues);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    // `check` lints the layers: the missing image is an error naming the file
    let src = source();
    let segs = segments(&plan, &src);
    let issues = check(&plan, &src, &segs, &dir, &[]);
    assert!(issues.iter().any(|i| i.sev == Severity::Error && i.msg.contains("scene (edit-") && i.msg.contains("nope.png")), "{issues:?}");
}

#[test]
fn bad_scene_layers_fail_to_load() {
    let dir = std::env::temp_dir().join(format!("cutroom-core-scene-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("edit.yaml");
    std::fs::write(&p, "beats:\n  - {at: w1, kind: scene, layers: [{type: text, text: hi, x: 0, y: 0, colour: red}]}\n").unwrap();
    let e = spec::load(&p).unwrap_err().to_string();
    assert!(e.contains("colour"), "{e}");
}
