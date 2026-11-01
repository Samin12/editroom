use super::*;
use cutroom_core::model::{Clip, Media, Track, Word};
use cutroom_core::spec::BeatKind;
use cutroom_cut::{Proposal, Why};

/// 400 ms per word; camera all along, screen recording over the second half, one mic take.
fn source(text: &str) -> Source {
    let words: Vec<Word> = text
        .split_whitespace()
        .enumerate()
        .map(|(i, t)| Word { text: t.into(), start_ms: 1000.0 + i as f64 * 400.0, end_ms: 1000.0 + i as f64 * 400.0 + 300.0 })
        .collect();
    let dur = 1000.0 + words.len() as f64 * 400.0 + 1000.0;
    Source {
        version: 1,
        name: "t".into(),
        duration_ms: dur,
        media: vec![
            Media { id: "cam".into(), path: "cam.mp4".into(), width: 1920, height: 1080, fps: 30.0, duration_ms: dur },
            Media { id: "scr".into(), path: "scr.mp4".into(), width: 3000, height: 2000, fps: 30.0, duration_ms: dur },
            Media { id: "mic".into(), path: "mic.wav".into(), width: 0, height: 0, fps: 0.0, duration_ms: dur },
        ],
        clips: vec![
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: dur, media_in_ms: 0.0, take: 0 },
            Clip { track: Track::Screen, media: "scr".into(), start_ms: dur / 2.0, dur_ms: dur / 2.0, media_in_ms: 0.0, take: 0 },
            Clip { track: Track::Mic, media: "mic".into(), start_ms: 0.0, dur_ms: dur, media_in_ms: 0.0, take: 0 },
        ],
        words,
        ..Default::default()
    }
}

const TEXT: &str = "So today we connect Claude to Poppy. So today we connect Claude to Poppy and it is fast. First open the board. Then paste your key here. That is it. Now click save and watch the board load. Done.";

/// A work folder with `src` and an edit that includes cuts.yaml and beats/*.
fn work(name: &str, src: &Source) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cutroom-agent-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("beats")).unwrap();
    src.save(&dir.join("source.json")).unwrap();
    std::fs::write(dir.join("edit.yaml"), "source: source.json\ninclude: [cuts.yaml, beats/*.yaml]\noutput: {chapter_cards: off}\n").unwrap();
    std::fs::write(dir.join("cuts.yaml"), "cuts: []\n").unwrap();
    dir
}

fn normw(src: &Source) -> Vec<String> {
    src.words.iter().map(|w| norm(&w.text)).collect()
}

#[test]
fn chunks_cover_everything_and_later_chunks_own_the_overlap() {
    let kept: Vec<usize> = (0..4000).map(|i| i * 2).collect();
    let cs = editorial::chunks(&kept, 1500, 225);
    assert!(cs.len() >= 3);
    assert_eq!(cs[0].lo, 0);
    assert_eq!(cs.last().unwrap().hi, kept.len());
    for w in cs.windows(2) {
        // overlapping views, owned ranges that tile
        assert!(w[1].lo < w[0].hi, "{w:?}");
        assert_eq!(w[0].own_hi, w[1].own_lo);
        assert_eq!(w[1].own_lo, kept[w[1].lo]);
    }
    assert_eq!(cs.last().unwrap().own_hi, usize::MAX);
    // a small transcript is one chunk
    let one = editorial::chunks(&kept[..900], 1500, 225);
    assert_eq!(one.len(), 1);
    assert_eq!((one[0].lo, one[0].hi, one[0].own_lo), (0, 900, 0));
}

#[test]
fn snap_finds_the_quoted_words() {
    let src = source(TEXT);
    let w = normw(&src);
    // exact text, ids off by two
    assert_eq!(editorial::snap(&w, 2, 9, "So today we connect Claude to Poppy.", 0, 40), Some((0, 6)));
    // head ... tail
    assert_eq!(editorial::snap(&w, 7, 15, "So today we ... it is fast.", 0, 40), Some((7, 17)));
    // the first take, not the second, when the ids point there
    assert_eq!(editorial::snap(&w, 0, 6, "So today we connect Claude to Poppy", 0, 40), Some((0, 6)));
    // words that are not there
    assert_eq!(editorial::snap(&w, 0, 3, "hello there friend", 0, 40), None);
}

fn plan_of(dir: &Path) -> (Source, Plan) {
    let (_, src, plan) = load(dir).unwrap();
    (src, plan)
}

#[test]
fn editorial_answers_are_validated_against_the_words() {
    let src = source(TEXT);
    let dir = work("validate", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let ch = editorial::chunks(&c.kept, 1500, 225).remove(0);
    let report = CutReport {
        auto: vec![],
        review: vec![Proposal { span: Span::Words(10, 11), why: Why::FalseStart, confidence: 0.6, reason: "x".into() }],
    };
    let w = normw(&src);
    let ans = "accept: [r1]\ncuts:\n  - {from: w1, to: w7, why: retake, text: \"So today we connect Claude to Poppy.\", note: re-said}\n  - {from: w20, to: w22, why: repeat, text: \"nothing like this\"}\npauses: []\n";
    let r = editorial::validate(ans, &ch, &c, &report, &w);
    assert_eq!(r.accept, vec![0]);
    assert_eq!(r.cuts, vec![editorial::EdCut { from: 0, to: 6, why: "retake".into(), note: Some("re-said".into()) }]);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("does not match"));
    assert!(!r.over_cap);

    // cutting most of the stretch trips the cap; unknown reasons become `edit`
    let greedy = "cuts:\n  - {from: w0, to: w30, why: tangent}\n";
    let r = editorial::validate(greedy, &ch, &c, &report, &w);
    assert!(r.over_cap);
    assert_eq!(r.cuts[0].why, "edit");

    // not YAML at all
    assert!(!editorial::validate("{{{", &ch, &c, &report, &w).errors.is_empty());
}

#[test]
fn ownership_keeps_items_that_start_in_the_chunk() {
    let ch = editorial::Chunk { k: 1, lo: 10, hi: 50, own_lo: 20, own_hi: 40 };
    let report = CutReport { auto: vec![], review: vec![Proposal { span: Span::Words(25, 26), why: Why::Retake, confidence: 0.5, reason: String::new() }, Proposal { span: Span::Words(45, 46), why: Why::Retake, confidence: 0.5, reason: String::new() }] };
    let r = editorial::EdResult {
        cuts: vec![editorial::EdCut { from: 12, to: 14, why: "retake".into(), note: None }, editorial::EdCut { from: 22, to: 24, why: "retake".into(), note: None }],
        accept: vec![0, 1],
        ..Default::default()
    };
    let o = editorial::owned(r, &ch, &report);
    assert_eq!(o.cuts.len(), 1);
    assert_eq!(o.cuts[0].from, 22);
    assert_eq!(o.accept, vec![0]);
}

#[test]
fn kept_pauses_give_time_back_to_silence_cuts() {
    let mut r = CutReport {
        auto: vec![
            Proposal { span: Span::Ms(1000.0, 5000.0), why: Why::Silence, confidence: 0.9, reason: String::new() },
            Proposal { span: Span::Ms(9000.0, 10_000.0), why: Why::Silence, confidence: 0.9, reason: String::new() },
        ],
        review: vec![],
    };
    let ps = vec![
        editorial::EdPause { cut: 0, after: 3, keep_ms: 1500.0, note: None },
        editorial::EdPause { cut: 0, after: 3, keep_ms: 2000.0, note: None },
        editorial::EdPause { cut: 1, after: 9, keep_ms: 900.0, note: None },
    ];
    assert_eq!(editorial::apply_pauses(&mut r, &ps), 2);
    assert_eq!(r.auto.len(), 1, "the short one is gone entirely");
    assert_eq!(r.auto[0].span, Span::Ms(3000.0, 5000.0));
}

#[test]
fn editorial_cuts_merge_and_round_trip_through_cuts_yaml() {
    let m = editorial::merge_cuts(vec![
        editorial::EdCut { from: 10, to: 12, why: "retake".into(), note: None },
        editorial::EdCut { from: 0, to: 3, why: "repeat".into(), note: Some("x".into()) },
        editorial::EdCut { from: 13, to: 15, why: "abandoned".into(), note: None },
    ]);
    assert_eq!(m.iter().map(|c| (c.from, c.to)).collect::<Vec<_>>(), vec![(0, 3), (10, 15)]);
    let human = vec![CutSpec { from: Some(Anchor("w30".into())), to: None, ms: None, why: None, by: Some("human".into()), note: None }];
    let text = cuts_file(&CutReport::default(), &m, &human);
    let dir = std::env::temp_dir().join(format!("cutroom-agent-cuts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cuts.yaml"), &text).unwrap();
    let s = spec::read_spec(&dir.join("cuts.yaml")).unwrap();
    assert_eq!(s.cuts.len(), 3, "{text}");
    assert_eq!(s.cuts[0].by.as_deref(), Some("agent"));
    assert_eq!(s.cuts[0].note.as_deref(), Some("x"));
    assert_eq!(kept_cuts(&dir, false).len(), 1);
    assert_eq!(kept_cuts(&dir, true).len(), 3);
}

#[test]
fn brief_marks_ids_review_items_and_pauses() {
    let src = source(TEXT);
    let dir = work("brief", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let mut marks = brief::Marks::default();
    marks.open.insert(7, 1);
    marks.close.insert(9, 1);
    marks.pause.insert(31, 2.5);
    let b = brief::brief(&c, 0, c.kept.len(), 3, Some(&marks), 40);
    let first = b.lines().next().unwrap();
    assert!(first.starts_with("w0 [talk] So today we w3 connect"), "{b}");
    assert!(b.contains("[[r1 So today w9 we]]") || b.contains("[[r1 So"), "{b}");
    assert!(b.contains("⏸2.5s"), "{b}");
    // the second half is the screen
    assert!(b.lines().any(|l| l.contains("[screen]")), "{b}");
    let plain = brief::brief(&c, 0, c.kept.len(), 0, None, 40);
    assert!(!plain.contains(" w3 "));
}

#[test]
fn prompts_offer_what_is_asked() {
    let all = prompts::beats_prompt(&prompts::BeatsOpts { scenes: true, screen_pass: true, images: vec!["proof/a.png".into()], videos: vec![], glossary: Some("Honcho; Grok Bot".into()) });
    for k in ["kind: title", "kind: bullets", "kind: number", "kind: code", "kind: quote", "kind: image", "proof: {claim", "kind: scene", "proof/a.png"] {
        assert!(all.contains(k), "missing {k}");
    }
    assert!(!all.contains("kind: callout"), "the screen pass owns [screen] lines");
    let lean = prompts::beats_prompt(&prompts::BeatsOpts { scenes: false, screen_pass: false, ..Default::default() });
    assert!(!lean.contains("kind: scene"));
    // scenes come with design rules and worked examples; every prompt that writes on-screen text
    // carries the names rule
    for k in ["Fill the panel", "type: arrow, from:", "style: marker", "type: counter", "keys: [{t:"] {
        assert!(all.contains(k), "scene guide misses {k}");
    }
    assert!(!lean.contains("Fill the panel"));
    assert!(all.contains("glossary: Honcho; Grok Bot.") && lean.contains("misspells"));
    assert!(prompts::chapter_prompt(Some("Honcho")).contains("glossary: Honcho."));
    assert!(prompts::SCENE_GUIDE.matches("kind: scene").count() >= 3);
    assert!(lean.contains("kind: callout"));
    assert!(prompts::system(Some("Never say guys.")).ends_with("Never say guys."));
    assert_eq!(prompts::system(Some("  ")), prompts::SYSTEM);
    let f = prompts::fix("P", "beats: x", &["a".into(), "b".into()]);
    assert!(f.starts_with('P') && f.contains("beats: x") && f.contains("a\nb"));
}

#[test]
fn lenient_parse_and_salvage_keep_what_is_valid() {
    let src = source(TEXT);
    let dir = work("salvage", &src);
    let (base, src, plan) = load(&dir).unwrap();
    let c = Ctx::new(&src, &plan);
    let text = r#"beats:
  - {at: w0, kind: title, text: "Connect Claude"}
  - {at: w2, kind: sparkle, layers: []}
  - {at: w9999, kind: number, value: "3x"}
  - {at: w30, kind: title, text: "On the screen"}
"#;
    let l = beats::parse_lenient(text).unwrap();
    assert_eq!(l.beats.len(), 3);
    assert_eq!(l.bad.len(), 1, "sparkle is not a kind");
    let (keep, dropped) = beats::salvage(&base, &src, Some(&c), "beats/ch01.yaml", l);
    assert_eq!(keep.beats.len(), 1, "{dropped:?}");
    assert_eq!(dropped.len(), 3);
    assert!(dropped.iter().any(|d| d.why.contains("[screen] line")), "{dropped:?}");
    // what we write reads back
    let out = beats::render_file(&keep);
    std::fs::write(dir.join("beats/ch01.yaml"), &out).unwrap();
    let back = spec::read_spec(&dir.join("beats/ch01.yaml")).unwrap();
    assert!(matches!(back.beats[0].kind, BeatKind::Title { .. }));
    assert_eq!(beats::render_file(&Spec::default()), "beats: []\n");
    assert!(beats::issues_for(&base, &src, "beats/ch01.yaml", &back).is_empty());
}

#[test]
fn screen_marks_are_checked_against_the_window() {
    let src = source(TEXT);
    let dir = work("screen", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let w = screen::Window { k: 0, lo: c.pos_at_or_after(25), hi: c.kept.len(), frames: vec![] };
    let mk = |y: &str| -> (spec::BeatSpec, String) { (serde_yaml::from_str(y).unwrap(), y.to_string()) };
    let (ok, bad) = screen::check_marks(
        &c,
        &w,
        vec![
            mk("{at: w30, until: w32, kind: ring, rect: [0.6, 0.1, 0.2, 0.1]}"),
            mk("{at: w30, kind: ring, rect: [0.9, 0.1, 0.2, 0.1]}"),
            mk("{at: w2, kind: callout, text: Save}"),
            mk("{at: w31, kind: zoom, rect: [0.1, 0.1, 0.1, 0.1]}"),
            mk("{at: w33, kind: title, text: nope}"),
            mk("{at: w34, kind: highlight, rect: [0.1, 0.5, 0.5, 0.05]}"),
            mk("{at: w35, kind: arrow, from: [0.1, 0.1], to: [0.4, 0.4]}"),
            mk("{at: w36, kind: callout, text: Click save}"),
        ],
    );
    assert_eq!(ok.len(), 3);
    assert_eq!(bad.len(), 5, "{bad:?}");
    assert!(bad.iter().any(|b| b.1.contains("more than 3")));
}

#[test]
fn screen_windows_follow_the_screen_recording() {
    let text = (0..200).map(|i| if i % 12 == 11 { "end." } else { "word" }).collect::<Vec<_>>().join(" ");
    let src = source(&text);
    let dir = work("windows", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let segs = segments(&plan, &src);
    let ws = screen::windows(&c, &segs);
    // 100 screen words at 400 ms = 40 s: one window, two frames, all on the screen
    assert_eq!(ws.len(), 1, "{ws:?}");
    assert_eq!(ws[0].frames.len(), 2);
    assert!(ws[0].frames.iter().all(|f| f.media.ends_with("scr.mp4")));
    assert!(c.screen[ws[0].lo] && !c.screen[ws[0].lo - 1]);
}

#[test]
fn proofs_list_pending_and_drop_missing_falls_back_to_text() {
    let src = source(TEXT);
    let dir = work("proofs", &src);
    std::fs::write(
        dir.join("beats/ch01.yaml"),
        "beats:\n  - {at: w0, kind: title, text: Hi}\n  - {at: w8, kind: image, src: proof/rates.png, caption: \"LaunchPoint pays $10,075.\", proof: {claim: \"The LaunchPoint campaign page lists $10,075 for one video\", find: launchpoint.app}}\n  - {at: w17, kind: image, src: proof/long.png, proof: {claim: \"one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four twenty-five twenty-six\"}}\n",
    )
    .unwrap();
    let p = crate::proofs::pending(&dir).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!((p[0].file.as_str(), p[0].index, p[0].src.as_str(), p[0].line), ("beats/ch01.yaml", 1, "proof/rates.png", "talk"));
    assert_eq!(p[0].find.as_deref(), Some("launchpoint.app"));
    // pending is a warning for check, but render refuses
    let (loaded, src2, plan) = load(&dir).unwrap();
    let segs = segments(&plan, &src2);
    let issues = lint::check(&plan, &src2, &segs, &loaded.dir, &[]);
    assert!(issues.iter().all(|i| i.sev != resolve::Severity::Error), "{issues:?}");
    assert!(issues.iter().any(|i| i.msg.contains("proof pending")));
    assert_eq!(lint::pending_proofs(&plan, &loaded.dir).len(), 2);

    // a captured file is no longer pending
    std::fs::create_dir_all(dir.join("proof")).unwrap();
    std::fs::write(dir.join("proof/rates.png"), b"x").unwrap();
    assert_eq!(crate::proofs::pending(&dir).unwrap().len(), 1);
    std::fs::remove_file(dir.join("proof/rates.png")).unwrap();

    let log = crate::proofs::drop_missing(&dir).unwrap();
    assert_eq!(log.len(), 2, "{log:?}");
    let s = spec::read_spec(&dir.join("beats/ch01.yaml")).unwrap();
    assert_eq!(s.beats.len(), 2, "the too-long claim is removed");
    assert!(matches!(&s.beats[1].kind, BeatKind::Title { text, .. } if text == "LaunchPoint pays $10,075"));
    assert!(crate::proofs::pending(&dir).unwrap().is_empty());
    assert!(std::fs::read_to_string(dir.join("agent/proofs.log")).unwrap().contains("now a title"));
}

#[test]
fn stash_moves_generated_beat_files_only() {
    let src = source(TEXT);
    let dir = work("stash", &src);
    for f in ["00-chapters.yaml", "ch01.yaml", "ch12.yaml", "screen.yaml", "privacy.yaml", "chapter-notes.yaml"] {
        std::fs::write(dir.join("beats").join(f), "beats: []\n").unwrap();
    }
    let left = |d: &Path| {
        let mut v: Vec<String> = std::fs::read_dir(d.join("beats")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    };
    // without a screen pass, its old marks stay
    stash_previous(&dir, &dir.join("agent"), false).unwrap();
    assert_eq!(left(&dir), vec!["chapter-notes.yaml", "privacy.yaml", "screen.yaml"]);
    stash_previous(&dir, &dir.join("agent"), true).unwrap();
    assert_eq!(left(&dir), vec!["chapter-notes.yaml", "privacy.yaml"]);
}

#[test]
fn base64_matches_the_standard() {
    assert_eq!(llm::base64(b""), "");
    assert_eq!(llm::base64(b"f"), "Zg==");
    assert_eq!(llm::base64(b"fo"), "Zm8=");
    assert_eq!(llm::base64(b"foobar"), "Zm9vYmFy");
    assert_eq!(llm::strip_fences("```yaml\nbeats: []\n```"), "beats: []\n");
    assert_eq!(llm::strip_fences("Here you go:\n```yaml\nbeats: []\n```\nDone"), "beats: []\n");
    assert_eq!(llm::strip_fences("beats:\n  - {at: w1, kind: code, text: \"```sh\"}"), "beats:\n  - {at: w1, kind: code, text: \"```sh\"}\n");
}

#[test]
fn par_map_keeps_order() {
    let v = llm::par_map((0..50).collect::<Vec<_>>(), 7, |x| x * 2);
    assert_eq!(v, (0..50).map(|x| x * 2).collect::<Vec<_>>());
}

#[test]
fn screen_marks_snap_to_kept_words_and_stay_small() {
    let src = source(TEXT);
    let dir = work("snap", &src);
    // cut w33-w34 so an anchor there has to move
    std::fs::write(dir.join("cuts.yaml"), "cuts:\n  - {from: w33, to: w34}\n").unwrap();
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let w = screen::Window { k: 0, lo: c.pos_at_or_after(25), hi: c.kept.len(), frames: vec![] };
    let mk = |y: &str| -> (spec::BeatSpec, String) { (serde_yaml::from_str(y).unwrap(), y.to_string()) };
    let (ok, bad) = screen::check_marks(
        &c,
        &w,
        vec![
            mk("{at: w33, until: w34, kind: ring, rect: [0.6, 0.1, 0.2, 0.1]}"),
            mk("{at: w30, until: w44, kind: highlight, rect: [0.1, 0.1, 0.5, 0.05]}"),
            mk("{at: w31, kind: ring, rect: [0.1, 0.1, 0.7, 0.7]}"),
            mk("{at: w32, kind: highlight, rect: [0.1, 0.1, 0.5, 0.5]}"),
        ],
    );
    assert_eq!(bad.len(), 2, "{bad:?}");
    assert_eq!(ok[0].at.0, "w35", "a cut start word moves forward");
    assert!(ok[0].until.is_none(), "an end before the start is dropped");
    assert_eq!(ok[1].until.as_ref().unwrap().0, format!("w{}", c.kept[w.hi - 1]), "an end past the stretch moves back");
}

#[test]
fn micro_cuts_are_left_out() {
    let mut src = source(TEXT);
    // w20 is a 200 ms word
    src.words[20].end_ms = src.words[20].start_ms + 200.0;
    let dir = work("micro", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let ch = editorial::chunks(&c.kept, 1500, 225).remove(0);
    let r = editorial::validate("cuts:\n  - {from: w20, to: w20, why: filler}\n  - {from: w21, to: w22, why: filler}\n", &ch, &c, &CutReport::default(), &normw(&src));
    assert_eq!(r.cuts.iter().map(|x| x.from).collect::<Vec<_>>(), vec![21]);
    assert!(r.errors.is_empty());
}

#[test]
fn names_glossary_reads_the_names_section_of_style_md() {
    let style = "# Creator style\n\nCuts\n- keep the last take\n\nNames\n- Honcho (memory layer), not Hancho\n- Grok Bot\n  - spelled with a space\n\nVisuals\n- big numbers\n";
    assert_eq!(prompts::names_glossary(Some(style)).as_deref(), Some("Honcho (memory layer), not Hancho; Grok Bot; spelled with a space"));
    assert_eq!(prompts::names_glossary(Some("## Names:\n- T3 Code\n## Other\n- x\n")).as_deref(), Some("T3 Code"));
    assert_eq!(prompts::names_glossary(Some("Cuts\n- a\n")), None);
    assert_eq!(prompts::names_glossary(Some("Names (correct spellings)\n- Honcho\n")).as_deref(), Some("Honcho"));
    assert_eq!(prompts::names_glossary(Some("- Names are hard\n- x\n")), None);
    assert_eq!(prompts::names_glossary(None), None);
}

#[test]
fn screen_prompt_carries_the_names_rule() {
    let src = source(TEXT);
    let dir = work("screen-names", &src);
    let (src, plan) = plan_of(&dir);
    let c = Ctx::new(&src, &plan);
    let w = screen::Window { k: 0, lo: c.pos_at_or_after(25), hi: c.kept.len(), frames: vec![] };
    let p = screen::prompt(&c, &w, Some("Claude Code"));
    assert!(p.contains("glossary: Claude Code.") && p.contains("never put a misspelled transcript word on screen"));
}
