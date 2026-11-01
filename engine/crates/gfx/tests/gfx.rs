//! Public-API tests: determinism, every kind draws where it should and nowhere else.

use cutroom_core::resolve::{OutWord, ResolvedBeat, ResolvedChapter};
use cutroom_core::spec::{BeatKind, Item};
use cutroom_gfx::{Gfx, Pixmap, Rect, ScreenMap, Theme};
use std::collections::BTreeMap;
use std::path::PathBuf;

const W: u32 = 1280;
const H: u32 = 720;
const BG: [u8; 4] = [7, 9, 11, 255];

fn gfx() -> Gfx {
    Gfx::new(Theme::from_style(&BTreeMap::new()), W, H, &assets()).unwrap()
}

/// A temp folder with a test screenshot (shared by tests; written once per process).
fn assets() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cutroom-gfx-test-{}", std::process::id()));
    let png = dir.join("shot.png");
    if !png.exists() {
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = Pixmap::new(400, 250).unwrap();
        s.fill(tiny_skia::Color::from_rgba8(230, 230, 225, 255));
        let mut p = tiny_skia::Paint::default();
        p.set_color_rgba8(40, 90, 200, 255);
        s.fill_rect(tiny_skia::Rect::from_xywh(40.0, 40.0, 120.0, 60.0).unwrap(), &p, tiny_skia::Transform::identity(), None);
        let tmp = dir.join(format!("shot-{:?}.png", std::thread::current().id()));
        s.save_png(&tmp).unwrap();
        let _ = std::fs::rename(&tmp, &png);
    }
    dir
}

fn frame() -> Pixmap {
    let mut pm = Pixmap::new(W, H).unwrap();
    pm.fill(tiny_skia::Color::from_rgba8(BG[0], BG[1], BG[2], BG[3]));
    pm
}

fn beat(kind: BeatKind, start: f64, end: f64) -> ResolvedBeat {
    ResolvedBeat { idx: 0, lane: kind.lane(), kind, start_ms: start, end_ms: end, items_ms: vec![], words: vec![], origin: "test".into() }
}

fn panel() -> Rect {
    Rect::from_xywh(0.01 * W as f32, 0.0178 * H as f32, 0.7676 * W as f32, 0.9644 * H as f32).unwrap()
}

fn px(pm: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * pm.width() + x) * 4) as usize;
    let d = pm.data();
    [d[i], d[i + 1], d[i + 2], d[i + 3]]
}

/// Pixels changed from the background outside `r` (expanded by `slack` px).
fn changed_outside(pm: &Pixmap, r: Rect, slack: f32) -> usize {
    let mut n = 0;
    for y in 0..pm.height() {
        for x in 0..pm.width() {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = fx >= r.left() - slack && fx < r.right() + slack && fy >= r.top() - slack && fy < r.bottom() + slack;
            if !inside && px(pm, x, y) != BG {
                n += 1;
            }
        }
    }
    n
}

/// Pixels that differ between two frames inside `r`.
fn diff_inside(a: &Pixmap, b: &Pixmap, r: Rect) -> usize {
    let mut n = 0;
    for y in r.top().max(0.0) as u32..(r.bottom() as u32).min(a.height()) {
        for x in r.left().max(0.0) as u32..(r.right() as u32).min(a.width()) {
            let (p, q) = (px(a, x, y), px(b, x, y));
            if (0..3).any(|c| (p[c] as i32 - q[c] as i32).abs() > 24) {
                n += 1;
            }
        }
    }
    n
}

fn stage_beats() -> Vec<ResolvedBeat> {
    let mut title = beat(BeatKind::Title { text: "Connect *Claude* to Poppy".into(), kicker: Some("Step 2".into()), sub: Some("In one prompt".into()) }, 1000.0, 5000.0);
    title.words = vec![OutWord { id: 0, text: "connect".into(), start_ms: 1130.0, end_ms: 1400.0 }];
    let mut bullets = beat(BeatKind::Bullets { title: Some("Three things".into()), items: vec![Item::Text("One".into()), Item::Text("Two".into()), Item::Text("Three".into())] }, 1000.0, 5000.0);
    bullets.items_ms = vec![1200.0, 1600.0, 2000.0];
    vec![
        title,
        bullets,
        beat(BeatKind::Number { value: "$10,075".into(), label: Some("per video".into()) }, 1000.0, 5000.0),
        beat(BeatKind::Image { src: "shot.png".into(), caption: Some("A screenshot".into()), highlight: Some([0.1, 0.16, 0.3, 0.24]), source: Some("example.com".into()) }, 1000.0, 5000.0),
        beat(BeatKind::Image { src: "missing.png".into(), caption: None, highlight: None, source: None }, 1000.0, 5000.0),
        beat(BeatKind::Video { src: "clip.mp4".into(), from: None, caption: Some("Footage".into()) }, 1000.0, 5000.0),
        beat(BeatKind::Code { text: "$ cargo run\n# done".into(), title: Some("Terminal".into()) }, 1000.0, 5000.0),
        beat(BeatKind::Quote { text: "Ship it.".into(), by: Some("Someone".into()) }, 1000.0, 5000.0),
    ]
}

fn media(g: &Gfx) -> Pixmap {
    let (w, h) = g.stage_media_size(panel());
    let mut m = Pixmap::new(w, h).unwrap();
    m.fill(tiny_skia::Color::from_rgba8(200, 60, 60, 255));
    m
}

#[test]
fn stage_beats_are_deterministic_and_stay_in_the_panel() {
    let g = gfx();
    let fresh = gfx();
    let m = media(&g);
    let empty = beat(BeatKind::Zoom { rect: [0.0, 0.0, 1.0, 1.0] }, 1000.0, 5000.0);
    for b in stage_beats() {
        let mm = matches!(b.kind, BeatKind::Video { .. }).then_some(&m);
        for t in [1000.0, 1150.0, 2600.0, 4900.0] {
            let mut a = frame();
            g.stage(&mut a, panel(), &b, t, mm);
            let mut a2 = frame();
            g.stage(&mut a2, panel(), &b, t, mm);
            assert!(a.data() == a2.data(), "{} at {t}: same input, same bytes", b.kind.name());
            let mut c = frame();
            fresh.stage(&mut c, panel(), &b, t, mm);
            assert!(a.data() == c.data(), "{} at {t}: caches never change the picture", b.kind.name());
            assert_eq!(changed_outside(&a, panel(), 0.0), 0, "{} at {t} drew outside the panel", b.kind.name());
        }
        // Mid-beat the content is visible over the bare surface.
        let mut a = frame();
        g.stage(&mut a, panel(), &b, 3000.0, mm);
        let mut bare = frame();
        g.stage(&mut bare, panel(), &empty, 3000.0, None);
        let n = diff_inside(&a, &bare, panel());
        assert!(n > 300, "{} draws content ({n} px)", b.kind.name());
        // Outside its window only the surface remains.
        let mut late = frame();
        g.stage(&mut late, panel(), &b, 6000.0, mm);
        assert_eq!(diff_inside(&late, &bare, panel()), 0, "{} after its window", b.kind.name());
    }
}

#[test]
fn stage_handles_odd_inputs() {
    let g = gfx();
    // Zero-length beat, a video beat without a frame, a frame of another size, a tiny panel.
    let mut pm = frame();
    g.stage(&mut pm, panel(), &beat(BeatKind::Title { text: "x".into(), kicker: None, sub: None }, 1000.0, 1000.0), 1000.0, None);
    g.stage(&mut pm, panel(), &beat(BeatKind::Video { src: "v.mp4".into(), from: None, caption: None }, 0.0, 3000.0), 1500.0, None);
    let odd = Pixmap::new(37, 91).unwrap();
    g.stage(&mut pm, panel(), &beat(BeatKind::Video { src: "v.mp4".into(), from: None, caption: None }, 0.0, 3000.0), 1500.0, Some(&odd));
    let tiny = Rect::from_xywh(10.0, 10.0, 40.0, 30.0).unwrap();
    for b in stage_beats() {
        let mut t = frame();
        g.stage(&mut t, tiny, &b, 3000.0, None);
        assert_eq!(changed_outside(&t, tiny, 0.0), 0, "{} in a tiny panel", b.kind.name());
    }
    // A panel hanging off the frame edge.
    let off = Rect::from_xywh(-200.0, 600.0, 900.0, 400.0).unwrap();
    g.stage(&mut pm, off, &stage_beats()[0], 3000.0, None);
}

#[test]
fn overlays_are_clipped_to_the_screen() {
    let g = gfx();
    let clip = Rect::from_xywh(100.0, 80.0, 700.0, 500.0).unwrap();
    // The screen is larger than the visible clip (zoomed), so rects cross the clip edge.
    let map = ScreenMap { x: 0.0, y: 0.0, sx: 1000.0, sy: 700.0, clip };
    let kinds = vec![
        BeatKind::Ring { rect: [0.05, 0.05, 0.3, 0.3] },
        BeatKind::Highlight { rect: [0.5, 0.5, 0.6, 0.05], underline: false },
        BeatKind::Highlight { rect: [0.2, 0.4, 0.3, 0.05], underline: true },
        BeatKind::Arrow { from: [0.05, 0.9], to: [0.5, 0.5] },
    ];
    for k in kinds {
        let b = beat(k, 0.0, 3000.0);
        for t in [300.0, 1500.0] {
            let mut pm = frame();
            g.overlay(&mut pm, &map, &b, t);
            assert_eq!(changed_outside(&pm, clip, 0.0), 0, "{} at {t} outside the clip", b.kind.name());
            let n = changed_outside(&pm, Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(), 0.0);
            assert!(n > 50, "{} at {t} draws ({n} px)", b.kind.name());
            let mut again = frame();
            g.overlay(&mut again, &map, &b, t);
            assert!(pm.data() == again.data(), "{} deterministic", b.kind.name());
        }
        let mut done = frame();
        g.overlay(&mut done, &map, &b, 3500.0);
        assert!(done.data() == frame().data(), "{} after its window draws nothing", b.kind.name());
    }
}

#[test]
fn frame_level_overlays_stay_in_frame_and_near_their_spot() {
    let g = gfx();
    let map = ScreenMap { x: 12.8, y: 12.8, sx: 982.0, sy: 694.0, clip: panel() };
    let mut pm = frame();
    g.overlay(&mut pm, &map, &beat(BeatKind::Callout { text: "Look here".into(), pos: None }, 0.0, 3000.0), 1500.0);
    // Default: top centre of the panel.
    let top = Rect::from_xywh(panel().left(), panel().top(), panel().width(), panel().height() * 0.25).unwrap();
    assert_eq!(changed_outside(&pm, top, 0.0), 0, "callout sits at the top of the panel");
    assert!(diff_inside(&pm, &frame(), top) > 200);
    let mut pm = frame();
    g.overlay(&mut pm, &map, &beat(BeatKind::Callout { text: "Edge".into(), pos: Some([1.0, 1.0]) }, 0.0, 3000.0), 1500.0);
    assert!(diff_inside(&pm, &frame(), Rect::from_xywh(W as f32 * 0.6, H as f32 * 0.6, W as f32 * 0.4, H as f32 * 0.4).unwrap()) > 100, "clamped into the frame corner");
    let mut pm = frame();
    g.overlay(&mut pm, &map, &beat(BeatKind::Lower { title: "Samin Yasar".into(), sub: Some("Founder".into()) }, 0.0, 4000.0), 2000.0);
    let bottom_left = Rect::from_xywh(0.0, H as f32 * 0.6, W as f32 * 0.6, H as f32 * 0.4).unwrap();
    assert_eq!(changed_outside(&pm, bottom_left, 0.0), 0, "lower third sits bottom-left");
    assert!(diff_inside(&pm, &frame(), bottom_left) > 500);
}

#[test]
fn chapter_card_badge_and_captions() {
    let g = gfx();
    let ch = ResolvedChapter { n: 3, title: "Design your character".into(), short: "Design".into(), at_ms: 10_000.0, card: Some((10_000.0, 11_800.0)) };
    for t in [10_000.0, 10_700.0, 11_790.0] {
        let mut a = frame();
        g.chapter_card(&mut a, &ch, 8, t);
        let mut b = frame();
        g.chapter_card(&mut b, &ch, 8, t);
        assert!(a.data() == b.data());
        assert!(a.data().as_chunks::<4>().0.iter().all(|p| p[3] == 255), "the card is opaque");
    }
    let mut zero = frame();
    g.chapter_card(&mut zero, &ch, 0, 10_900.0);
    let long = ResolvedChapter { title: "A very long chapter title that goes on and on and on and on and on forever".into(), ..ch.clone() };
    g.chapter_card(&mut zero, &long, 20, 10_900.0);

    // Badge: inside its rect, both shapes, popping after the card.
    for rect in [Rect::from_xywh(1000.0, 40.0, 236.0, 100.0).unwrap(), Rect::from_xywh(1000.0, 40.0, 236.0, 40.0).unwrap()] {
        for t in [11_850.0, 20_000.0] {
            let mut pm = frame();
            g.badge(&mut pm, rect, &ch, 8, t);
            assert_eq!(changed_outside(&pm, rect, 0.0), 0, "badge stays in its rect");
            assert!(diff_inside(&pm, &frame(), rect) > 100);
        }
        let mut pop = frame();
        g.badge(&mut pop, rect, &ch, 8, 11_850.0);
        let mut settled = frame();
        g.badge(&mut settled, rect, &ch, 8, 20_000.0);
        assert!(pop.data() != settled.data(), "badge pops when the chapter starts");
    }

    // Captions: only around spoken words, inside the area.
    let words: Vec<OutWord> = "one two three four five six seven eight nine ten".split(' ').enumerate().map(|(i, w)| OutWord { id: i, text: w.into(), start_ms: i as f64 * 300.0, end_ms: i as f64 * 300.0 + 250.0 }).collect();
    let area = panel();
    let mut pm = frame();
    g.captions(&mut pm, area, &words, 700.0);
    assert_eq!(changed_outside(&pm, area, 0.0), 0);
    assert!(diff_inside(&pm, &frame(), area) > 200);
    let mut silent = frame();
    g.captions(&mut silent, area, &words, 9000.0);
    assert!(silent.data() == frame().data(), "no captions long after the last word");
    let mut before = frame();
    g.captions(&mut before, area, &words, -100.0);
    assert!(before.data() == frame().data());
}

#[test]
fn canvas_is_opaque_drifts_and_repeats() {
    let g = gfx();
    let mut a = frame();
    g.canvas(&mut a, 0.0);
    assert!(a.data().as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    assert_eq!(px(&a, 3, 3), [0x12, 0x12, 0x12, 255]);
    let mut b = frame();
    g.canvas(&mut b, 3000.0);
    assert!(a.data() != b.data(), "the grid drifts");
    // One full pitch of drift (40 px at 1080p = 26.67 -> 27 px at 720p) brings it back.
    let pitch = (40.0f64 * H as f64 / 1080.0).round();
    let mut c = frame();
    g.canvas(&mut c, pitch / (2.5 * H as f64 / 1080.0) * 1000.0);
    assert!(a.data() == c.data(), "periodic in the pitch");
    // Shadows only darken, outside the slot interior.
    let mut s = frame();
    g.canvas(&mut s, 0.0);
    let before = s.clone();
    let slot = panel();
    g.slot_shadow(&mut s, slot, g.slot_radius());
    let (cx, cy) = ((slot.left() + slot.width() * 0.5) as u32, (slot.bottom() + 6.0) as u32);
    assert!(px(&s, cx, cy)[0] < px(&before, cx, cy)[0], "shadow below the slot");
    assert_eq!(px(&s, cx, (slot.top() + slot.height() * 0.5) as u32), px(&before, cx, (slot.top() + slot.height() * 0.5) as u32));
}

#[test]
fn higher_resolutions_scale() {
    let g = Gfx::new(Theme::from_style(&BTreeMap::new()), 2560, 1440, &assets()).unwrap();
    assert!((g.scale - 1440.0 / 1080.0).abs() < 1e-6);
    let mut pm = Pixmap::new(2560, 1440).unwrap();
    g.canvas(&mut pm, 1234.0);
    let p = Rect::from_xywh(25.6, 25.6, 1965.0, 1388.7).unwrap();
    for b in stage_beats() {
        g.stage(&mut pm, p, &b, 3000.0, None);
    }
    let ch = ResolvedChapter { n: 1, title: "Intro".into(), short: "Intro".into(), at_ms: 0.0, card: None };
    g.chapter_card(&mut pm, &ch, 4, 500.0);
}
