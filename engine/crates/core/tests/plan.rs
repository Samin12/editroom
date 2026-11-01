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
    // unique per call: tests run in parallel and two specs can have the same length
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let p = dir.join(format!("edit-{}.yaml", N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
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

#[test]
fn include_folder_tolerates_a_file_removed_after_listing() {
    // a beats/ file can vanish between listing the folder and reading it (the agent archives the
    // previous run while privacy or check loads the spec); the loader skips it instead of failing
    let d = std::env::temp_dir().join(format!("cutroom-vanish-{}", std::process::id()));
    std::fs::create_dir_all(d.join("beats")).unwrap();
    std::fs::write(d.join("edit.yaml"), "source: source.json\ninclude: [beats/*.yaml]\n").unwrap();
    std::fs::write(d.join("beats/a.yaml"), "beats: []\n").unwrap();
    std::fs::write(d.join("beats/b.yaml"), "beats: []\n").unwrap();
    let l = cutroom_core::spec::load(&d.join("edit.yaml")).unwrap();
    assert!(l.spec.beats.is_empty());
    assert_eq!(cutroom_core::spec::source_path(&d.join("edit.yaml")).unwrap(), d.join("source.json"));
    std::fs::remove_dir_all(&d).ok();
}

fn work_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cutroom-core-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("beats")).unwrap();
    source().save(&dir.join("source.json")).unwrap();
    dir
}

#[test]
fn output_keys_from_includes_apply_where_edit_yaml_is_silent() {
    let dir = work_dir("kicker");
    std::fs::write(dir.join("edit.yaml"), "source: source.json\ninclude: [beats/*.yaml]\noutput: {height: 720, chapter_cards: off}\n").unwrap();
    std::fs::write(dir.join("beats/00-chapters.yaml"), "output: {kicker: Tip, intro: Why Voice Wins, height: 1080}\nchapters:\n  - {at: \"first open the board\", title: Open the board}\n").unwrap();
    let l = spec::load(&dir.join("edit.yaml")).unwrap();
    assert_eq!(l.spec.output.kicker, "Tip");
    assert_eq!(l.spec.output.intro, "Why Voice Wins");
    assert_eq!(l.spec.output.height, 720, "edit.yaml wins over an include");
    let plan = build(&l, &source());
    assert_eq!(plan.chapters[0].kicker, "Tip");
    // the root can set it too, and an empty kicker is kept
    std::fs::write(dir.join("edit.yaml"), "source: source.json\ninclude: [beats/*.yaml]\noutput: {kicker: \"\"}\n").unwrap();
    assert_eq!(spec::load(&dir.join("edit.yaml")).unwrap().spec.output.kicker, "");
    // defaults without any output
    let (p, _, _) = plan_for("chapters:\n  - {at: w0, title: Hi}\n");
    assert_eq!((p.output.kicker.as_str(), p.output.intro.as_str(), p.chapters[0].kicker.as_str()), ("Step", "Intro", "Step"));
}

#[test]
fn counters_reject_digits_in_prefix_and_suffix() {
    let (plan, src, dir) = plan_for(
        r#"
beats:
  - {at: w0, kind: scene, layers: [{type: counter, to: 250, prefix: "250+ ", x: 0.5, y: 0.5}, {type: counter, to: 250, suffix: "+ wpm", x: 0.5, y: 0.7}]}
"#,
    );
    let issues = check(&plan, &src, &segments(&plan, &src), &dir, &[]);
    let bad: Vec<_> = issues.iter().filter(|i| i.sev == Severity::Error && i.msg.contains("has a digit")).collect();
    assert_eq!(bad.len(), 1, "{issues:?}");
    assert!(bad[0].msg.contains("prefix"));
}

#[test]
fn open_ended_lists_and_scenes_stay_until_their_last_item_reads() {
    // items on w16 and w22 (3.2 s and 5.6 s after w8); the default hold ends the list ~5.2 s in
    let (plan, src, dir) = plan_for(
        r#"
beats:
  - {at: w8, kind: bullets, items: [{text: one, at: w8}, {text: two, at: w16}, {text: three, at: w22}]}
  - {at: w0, kind: scene, layers: [{type: text, text: Hi, x: 0.1, y: 0.1}, {type: text, text: later, x: 0.1, y: 0.5, in: {at: w6}}]}
"#,
    );
    let list = plan.beats.iter().find(|b| b.kind.name() == "bullets").unwrap();
    let w22 = plan.words.iter().find(|w| w.id == 22).unwrap();
    assert!(list.end_ms >= w22.start_ms - 130.0 + 2400.0 && list.end_ms <= plan.duration_ms(), "{} vs {}", list.end_ms, w22.start_ms);
    assert!(list.items_ms[2] < list.end_ms - 2000.0);
    let scene = plan.beats.iter().find(|b| b.kind.name() == "scene").unwrap();
    let w6 = plan.words.iter().find(|w| w.id == 6).unwrap();
    assert!(scene.end_ms >= w6.start_ms - 130.0 + 2400.0, "the scene stays until its last layer reads");
    let issues = check(&plan, &src, &segments(&plan, &src), &dir, &[]);
    assert!(!issues.iter().any(|i| i.msg.contains("too late to read")), "{issues:?}");

    // with an explicit end, a late item is a warning instead
    let (plan, src, dir) = plan_for("beats:\n  - {at: w8, until: w20, kind: bullets, items: [{text: one, at: w8}, {text: two, at: w20}]}\n");
    let list = &plan.beats[0];
    let w20 = plan.words.iter().find(|w| w.id == 20).unwrap();
    assert!((list.end_ms - (w20.end_ms + 250.0)).abs() < 1.0, "an explicit until is kept");
    let issues = check(&plan, &src, &segments(&plan, &src), &dir, &[]);
    assert!(issues.iter().any(|i| i.sev == Severity::Warn && i.msg.contains("item \"two\"") && i.msg.contains("too late to read")), "{issues:?}");
}

#[test]
fn numbered_kickers_follow_the_chapter_order() {
    let yaml = |kicker: &str, title_kicker: &str| {
        format!(
            "output: {{kicker: \"{kicker}\", chapter_cards: off}}\nchapters:\n  - {{at: w0, title: One}}\n  - {{at: \"first open the board\", title: Two}}\nbeats:\n  - {{at: \"paste your key\", kind: title, kicker: \"{title_kicker}\", text: Paste it}}\n"
        )
    };
    let warns = |y: String| {
        let (plan, src, dir) = plan_for(&y);
        check(&plan, &src, &segments(&plan, &src), &dir, &[]).into_iter().filter(|i| i.msg.contains("follow the chapter order")).count()
    };
    assert_eq!(warns(yaml("Tip", "TIP 1")), 1, "TIP 1 inside chapter 2");
    assert_eq!(warns(yaml("Tip", "Tip #2")), 0);
    assert_eq!(warns(yaml("Tip", "TIPPING POINT")), 0);
    assert_eq!(warns(yaml("", "TIP 1")), 0, "no kicker, no numbering to check");
    assert_eq!(cutroom_core::lint::numbered("Step 3: paste", "Step"), Some(("step".into(), 3)));
    assert_eq!(cutroom_core::lint::numbered("Steps 3", "Step"), None);
    assert_eq!(cutroom_core::lint::numbered("TIP 12", "tip"), Some(("tip".into(), 12)));
}

#[test]
fn chapter_groups_put_cards_on_group_starts() {
    let (plan, _, _) = plan_for(
        r#"
output: {chapter_cards: insert, card_ms: 1500, kicker: Tip}
chapters:
  - {at: w11, title: One, group: The Easy Tips}
  - {at: "first open the board", title: Two, group: The Easy Tips}
  - {at: "then paste your key", title: Three, group: The Hard Tips}
"#,
    );
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let c = &plan.chapters;
    let g: Vec<_> = c.iter().map(|x| x.group.clone().unwrap()).collect();
    assert_eq!((g[0].n, g[0].of, g[0].first, g[0].last), (1, 2, 1, 2));
    assert_eq!((g[2].n, g[2].of, g[2].first, g[2].last), (2, 2, 3, 3));
    // cards only where a group starts, showing the group
    assert!(c[0].card.is_some() && g[0].card);
    assert!(c[1].card.is_none() && !g[1].card);
    assert!(c[2].card.is_some() && g[2].card);
    // only two inserted cards of 1.5 s lengthen the video
    let gaps: f64 = plan.map.pieces.iter().filter_map(|p| match *p {
        cutroom_core::timemap::Piece::Gap { dur, .. } => Some(dur),
        _ => None,
    }).sum();
    assert!((gaps - 3000.0).abs() < 1e-6, "{gaps}");

    // card_on: chapter keeps a card per chapter even with groups
    let (plan, _, _) = plan_for(
        r#"
output: {chapter_cards: insert, card_on: chapter}
chapters:
  - {at: w11, title: One, group: A}
  - {at: "first open the board", title: Two, group: A}
"#,
    );
    assert!(plan.chapters.iter().all(|c| c.card.is_some()));
    // no groups: defaults unchanged
    let (plan, _, _) = plan_for("chapters:\n  - {at: \"first open the board\", title: Two}\n");
    assert!(plan.chapters[0].group.is_none() && plan.chapters[0].card.is_some());
}

#[test]
fn opaque_films_fill_the_panel() {
    // the first half has no screen recording: an empty panel shows the full camera, unless an
    // opaque full-frame film covers it (then the camera stays a tile)
    let (plan, src, _) = plan_for(
        r#"
beats:
  - {at: "o:1.0", lead: 0, hold: 2s, kind: video, src: film.mp4, fit: frame, alpha: false, fade: false}
  - {at: "o:4.0", lead: 0, hold: 2s, kind: video, src: over.mov, fit: frame, alpha: true}
"#,
    );
    let segs = segments(&plan, &src);
    let at = |t: f64| segs.iter().find(|s| s.out0 <= t && t < s.out1).unwrap();
    assert_eq!(at(1500.0).mode, cutroom_core::spec::LayoutMode::Panel);
    assert!(!at(1500.0).empty_panel);
    assert_eq!(at(500.0).mode, cutroom_core::spec::LayoutMode::Camera);
    assert_eq!(at(4500.0).mode, cutroom_core::spec::LayoutMode::Camera, "a transparent overlay does not fill the panel");
    assert!(plan.beats.iter().any(|b| b.kind.is_film()));
    // a film that leaves `alpha` out counts too (only `alpha: true` marks an overlay)
    let (plan, src, _) = plan_for(
        r#"
beats:
  - {at: "o:0.0", lead: 0, hold: 3s, kind: video, src: intro.mp4, fit: frame, loop: false}
"#,
    );
    let segs = segments(&plan, &src);
    let s = segs.iter().find(|s| s.out0 <= 1500.0 && 1500.0 < s.out1).unwrap();
    assert_eq!(s.mode, cutroom_core::spec::LayoutMode::Panel);
}
