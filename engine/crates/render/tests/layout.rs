//! `cutroom plan --segments`: stretches of constant layout with the rects the compositor draws.

use cutroom_core::compose::segments;
use cutroom_core::model::*;
use cutroom_core::resolve::build;
use cutroom_core::spec;
use cutroom_render::frame::slots;
use cutroom_render::layout::{layout_segments, LayoutSeg};

const DUR: f64 = 30_000.0;

fn source() -> Source {
    let words: Vec<Word> = (0..60).map(|i| Word { text: format!("w{i}"), start_ms: 200.0 + i as f64 * 480.0, end_ms: 600.0 + i as f64 * 480.0 }).collect();
    Source {
        name: "layout".into(),
        origin: "test".into(),
        duration_ms: DUR,
        media: vec![
            Media { id: "cam".into(), path: "cam.mov".into(), width: 1920, height: 1080, fps: 30.0, duration_ms: DUR },
            Media { id: "scr".into(), path: "scr.mov".into(), width: 2560, height: 1440, fps: 60.0, duration_ms: DUR },
        ],
        clips: vec![
            Clip { track: Track::Camera, media: "cam".into(), start_ms: 0.0, dur_ms: DUR, media_in_ms: 0.0, take: 0 },
            // no screen for the first 4 s: the empty panel falls back to the full camera
            Clip { track: Track::Screen, media: "scr".into(), start_ms: 4_000.0, dur_ms: DUR - 4_000.0, media_in_ms: 0.0, take: 1 },
        ],
        words,
        scenes: vec![Scene { name: "one".into(), start_ms: 0.0, end_ms: DUR }],
        version: 1,
    }
}

const EDIT: &str = r#"
source: source.json
output: {height: 1080, fps: 30, chapter_cards: insert, card_ms: 1500}
chapters:
  - {at: w20, title: Two}
beats:
  - {at: w12, hold: 3s, kind: layout, mode: camera}
  - {at: w30, hold: 3s, kind: title, text: "A title"}
  - {at: w40, hold: 2s, kind: video, src: clip.mov, fit: frame}
  - {at: w48, hold: 2s, kind: layout, mode: screen}
"#;

fn at(segs: &[LayoutSeg], t: f64) -> &LayoutSeg {
    segs.iter().find(|s| s.start_ms <= t && t < s.end_ms).unwrap_or_else(|| panic!("no segment at {t}"))
}

#[test]
fn layout_segments_report_what_is_drawn() {
    let dir = std::env::temp_dir().join("cutroom-layout-segments");
    std::fs::create_dir_all(&dir).unwrap();
    let src = source();
    src.save(&dir.join("source.json")).unwrap();
    std::fs::write(dir.join("edit.yaml"), EDIT).unwrap();
    let plan = build(&spec::load(&dir.join("edit.yaml")).unwrap(), &src);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let segs = layout_segments(&plan, &src, &segments(&plan, &src));
    let (panel, tile) = slots(&plan);
    let full = Some([0, 0, 1920, 1080]);
    let out = |w: usize| plan.words.iter().find(|x| x.id == w).unwrap().start_ms;

    // contiguous over the whole output, and merged (no two neighbours alike)
    assert_eq!(segs[0].start_ms, 0.0);
    assert!((segs.last().unwrap().end_ms - plan.duration_ms()).abs() < 1e-6);
    for w in segs.windows(2) {
        assert!((w[0].end_ms - w[1].start_ms).abs() < 1e-6, "gap between {:?} and {:?}", w[0], w[1]);
        let same = w[0].mode == w[1].mode && w[0].camera_rect == w[1].camera_rect && w[0].screen_rect == w[1].screen_rect && w[0].stage == w[1].stage && w[0].frame_video == w[1].frame_video;
        assert!(!same, "unmerged neighbours {:?} {:?}", w[0], w[1]);
    }

    // empty panel before the screen starts: full-frame camera, no screen
    let s = at(&segs, 1_000.0);
    assert_eq!((s.mode, s.camera_rect, s.screen_rect), ("camera", full, None));
    assert_eq!(s.camera_media, ["cam"]);
    // normal panel: screen in the panel slot, camera in the tile
    let s = at(&segs, out(9) + 100.0);
    assert_eq!((s.mode, s.camera_rect, s.screen_rect), ("panel", Some(tile), Some(panel)));
    assert_eq!((s.screen_media.as_slice(), s.takes.as_slice()), (["scr".to_string()].as_slice(), [1u32, 0].as_slice()));
    // a `layout: camera` beat: the whole frame is the camera (not the tile rect)
    let s = at(&segs, out(13));
    assert_eq!((s.mode, s.camera_rect, s.screen_rect), ("camera", full, None));
    // the chapter card hides everything
    let card = plan.chapters[0].card.unwrap();
    let s = at(&segs, (card.0 + card.1) / 2.0);
    assert_eq!((s.mode, s.camera_rect, s.screen_rect, s.stage), ("card", None, None, None));
    // a stage beat replaces the screen; the camera tile stays
    let s = at(&segs, out(31));
    assert_eq!((s.mode, s.camera_rect, s.screen_rect, s.stage), ("panel", Some(tile), None, Some("title")));
    // a full-frame video is reported (and splits the stretch) without changing the layout
    let s = at(&segs, out(41));
    assert_eq!((s.mode, s.frame_video.as_deref()), ("panel", Some("clip.mov")));
    assert_eq!(at(&segs, out(46)).frame_video, None);
    // screen layout: the screen fills the frame, no camera
    let s = at(&segs, out(49));
    assert_eq!((s.mode, s.camera_rect, s.screen_rect), ("screen", None, full));
}

#[test]
fn overlays_keep_the_panel_and_boundaries_snap_to_frames() {
    // a transparent full-frame overlay that starts before the screen recording: the layout stays
    // panel (camera in its tile) instead of flashing the full-frame camera; and a beat that starts a
    // rounding error after a frame still owns that frame
    let dir = std::env::temp_dir().join("cutroom-layout-overlay");
    std::fs::create_dir_all(&dir).unwrap();
    let src = source();
    src.save(&dir.join("source.json")).unwrap();
    std::fs::write(
        dir.join("edit.yaml"),
        r#"
source: source.json
output: {height: 1080, fps: 30}
beats:
  - {at: "o:2", lead: 0, hold: 4s, kind: video, src: over.mov, fit: frame, alpha: true}
  - {at: "o:8.0000000000001", lead: 0, hold: 2s, kind: title, text: "Late by a hair"}
"#,
    )
    .unwrap();
    let plan = build(&spec::load(&dir.join("edit.yaml")).unwrap(), &src);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let segs = layout_segments(&plan, &src, &segments(&plan, &src));
    let (_, tile) = slots(&plan);
    // no overlay yet: the empty panel is the full camera, as before
    assert_eq!(at(&segs, 1_000.0).mode, "camera");
    // under the overlay, before the screen starts at 4 s: panel with the camera tile
    let s = at(&segs, 2_000.0);
    assert_eq!((s.mode, s.camera_rect, s.frame_video.as_deref()), ("panel", Some(tile), Some("over.mov")));
    let s = at(&segs, 3_500.0);
    assert_eq!((s.mode, s.camera_rect), ("panel", Some(tile)));
    // the frame at exactly 8000 ms shows the title
    assert_eq!(at(&segs, 8_000.0).stage, Some("title"));
}
