//! Regression: a privacy finding that spans cuts must be fully covered by the blur beats
//! `propose_beats` writes, so `check` is clean without editing anything by hand.
//!
//! The bug: at the exact output instant of a cut, the previous piece's computed end rounded a hair
//! past the next piece's start, so the time map gave that instant to the previous piece. `compose`
//! then saw the source as continuous and merged two pieces into one segment that spanned the cut:
//! lint mapped the finding's end ~0.3 s too late (blur "ended early"), and the renderer showed the
//! removed footage for the rest of that segment.

use cutroom_core::compose::segments;
use cutroom_core::lint::{self, Finding};
use cutroom_core::model::*;
use cutroom_core::resolve::{build, Severity};
use cutroom_core::spec;
use cutroom_core::timemap::Piece;
use cutroom_privacy::propose_beats;
use std::path::Path;

const DUR: f64 = 120_000.0;

fn source(dir: &Path) -> Source {
    // media files are never opened by check; dimensions are given
    let words: Vec<Word> = (0..300).map(|i| Word { text: format!("w{i}"), start_ms: 100.0 + i as f64 * 390.0, end_ms: 400.0 + i as f64 * 390.0 }).collect();
    Source {
        name: "blur-cuts".into(),
        origin: "test".into(),
        duration_ms: DUR,
        media: vec![
            Media { id: "scr".into(), path: dir.join("scr.mov"), width: 2560, height: 1440, fps: 60.0, duration_ms: DUR },
            Media { id: "cam".into(), path: dir.join("cam.mov"), width: 1920, height: 1080, fps: 30.0, duration_ms: DUR },
        ],
        clips: vec![
            Clip { track: Track::Screen, media: "scr".into(), start_ms: 0.0, dur_ms: DUR, media_in_ms: 0.0, take: 0 },
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: DUR, media_in_ms: 0.0, take: 0 },
        ],
        words,
        scenes: vec![Scene { name: "one".into(), start_ms: 0.0, end_ms: DUR }],
        version: 1,
    }
}

#[test]
fn findings_across_cuts_are_fully_blurred() {
    let dir = std::env::temp_dir().join("cutroom-privacy-blur-cuts");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("beats")).unwrap();
    let src = source(&dir);
    src.save(&dir.join("source.json")).unwrap();
    // the cut ranges of the real project section where the bug showed (silences of a few hundred
    // ms, fractional ms edges), repeated so many boundaries round badly
    let mut cuts = String::new();
    for k in 0..12 {
        let o = k as f64 * 9_000.0;
        for (a, b) in [(300.0, 2_640.4), (3_180.9, 3_252.6), (6_352.6, 6_662.6), (8_929.3, 8_995.1)] {
            cuts.push_str(&format!("  - {{ms: [{:.1}, {:.1}], why: silence}}\n", o + a, o + b));
        }
    }
    let edit = format!("source: source.json\ninclude: [beats/*.yaml]\noutput: {{height: 1080, fps: 30, chapter_cards: insert, card_ms: 1800}}\nchapters:\n  - {{at: w40, title: Two}}\ncuts:\n{cuts}");
    std::fs::write(dir.join("edit.yaml"), edit).unwrap();

    // secrets on screen for long stretches that cross many cuts, plus one ending exactly near a cut
    let f = |a: f64, b: f64, x: f64| Finding { src0_ms: a, src1_ms: b, rect: [x, 0.60, 0.05, 0.024], kind: "model-name".into(), preview: "Opus...".into() };
    let findings = vec![f(2_111.15, 15_244.15, 0.71), f(2_111.15, 15_244.15, 0.17), f(40_500.0, 61_000.0, 0.30), f(70_010.0, 72_352.6, 0.50), f(97_200.0, 119_900.0, 0.08)];

    let load = || {
        let loaded = spec::load(&dir.join("edit.yaml")).unwrap();
        let plan = build(&loaded, &src);
        let segs = segments(&plan, &src);
        (plan, segs, loaded)
    };

    // without blur beats every finding is reported (the fixture really puts them on screen)
    let (plan, segs, loaded) = load();
    let before = lint::check(&plan, &src, &segs, &loaded.dir, &findings);
    assert_eq!(before.iter().filter(|i| i.sev == Severity::Error && i.msg.starts_with("privacy:")).count(), findings.len(), "{}", lint::format(&before));

    // no segment may span a cut: its linear source mapping must agree with the time map everywhere
    for s in segs.iter().filter(|s| s.src0.is_some()) {
        // just inside each end: the boundary instant itself is shared with the neighbour
        let e = ((s.out1 - s.out0) / 4.0).min(0.5);
        for t in [s.out0 + e, (s.out0 + s.out1) / 2.0, s.out1 - e] {
            let (a, b) = (s.src_at(t).unwrap(), plan.map.to_src(t));
            assert!(b.is_some_and(|b| (a - b).abs() < 0.5), "segment {:.3}-{:.3} maps {t:.3} to {a:.1}, the time map says {b:?}", s.out0, s.out1);
        }
    }
    // the fixture has boundaries where the computed piece end rounds past the next start
    let unlucky = plan.map.pieces.windows(2).filter(|w| matches!(w[1], Piece::Src { .. }) && w[0].out1() > w[1].out0()).count();
    assert!(unlucky > 0, "fixture should exercise rounding-unlucky cut boundaries");

    // propose_beats -> check is clean
    std::fs::write(dir.join("beats/privacy.yaml"), propose_beats(&findings)).unwrap();
    let (plan, segs, loaded) = load();
    let after = lint::check(&plan, &src, &segs, &loaded.dir, &findings);
    let errors: Vec<_> = after.iter().filter(|i| i.sev == Severity::Error).collect();
    assert!(errors.is_empty(), "{}", lint::format(&after));
}
