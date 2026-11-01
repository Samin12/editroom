//! ms/frame for the canvas and every beat kind at 1080p and 2160p (single thread).
//!
//! `cargo run -p cutroom-gfx --release --example bench`

use cutroom_core::resolve::{OutWord, ResolvedBeat, ResolvedChapter};
use cutroom_core::spec::{BeatKind, Item};
use cutroom_gfx::{Gfx, Pixmap, Rect, ScreenMap, Theme};
use std::collections::BTreeMap;
use std::time::Instant;

fn beat(kind: BeatKind, start: f64, end: f64) -> ResolvedBeat {
    ResolvedBeat { idx: 0, lane: kind.lane(), kind, start_ms: start, end_ms: end, items_ms: vec![], words: vec![], origin: "bench".into() }
}

/// Average ms over `n` frames spread across `[t0, t1]`, after one cold frame (reported too).
fn time(n: usize, t0: f64, t1: f64, mut f: impl FnMut(f64)) -> (f64, f64) {
    let c = Instant::now();
    f(t0);
    let cold = c.elapsed().as_secs_f64() * 1000.0;
    let s = Instant::now();
    for i in 0..n {
        f(t0 + (t1 - t0) * i as f64 / n as f64);
    }
    (s.elapsed().as_secs_f64() * 1000.0 / n as f64, cold)
}

fn main() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join("cutroom-gfx-bench");
    std::fs::create_dir_all(&dir)?;
    // A 2400x1500 screenshot-like still for the image beat.
    let mut shot = Pixmap::new(2400, 1500).unwrap();
    shot.fill(tiny_skia::Color::from_rgba8(240, 240, 236, 255));
    for i in 0..30 {
        let r = tiny_skia::Rect::from_xywh(60.0 + (i % 5) as f32 * 460.0, 60.0 + (i / 5) as f32 * 240.0, 420.0, 200.0).unwrap();
        let mut p = tiny_skia::Paint::default();
        p.set_color_rgba8((i * 37 % 255) as u8, 120, 200, 255);
        shot.fill_rect(r, &p, tiny_skia::Transform::identity(), None);
    }
    shot.save_png(dir.join("shot.png"))?;

    let words: Vec<OutWord> = "so this is how the whole thing fits together and why it matters for you right now".split(' ').enumerate().map(|(i, w)| OutWord { id: i, text: w.into(), start_ms: i as f64 * 250.0, end_ms: i as f64 * 250.0 + 200.0 }).collect();
    let mut title = beat(BeatKind::Title { text: "Connect *Claude* to your whole stack".into(), kicker: Some("Step 2".into()), sub: Some("One prompt wires it all up.".into()) }, 0.0, 4000.0);
    title.words = words.clone();
    let mut bullets = beat(BeatKind::Bullets { title: Some("What the agent does".into()), items: vec![Item::Text("Finds the idea".into()), Item::Text("Writes the script".into()), Item::Text("Cuts the video".into()), Item::Text("Publishes it".into())] }, 0.0, 6000.0);
    bullets.items_ms = vec![300.0, 1100.0, 1900.0, 2700.0];
    let stage: Vec<(&str, ResolvedBeat)> = vec![
        ("title", title),
        ("bullets", bullets),
        ("number", beat(BeatKind::Number { value: "$10,075".into(), label: Some("paid for one video".into()) }, 0.0, 3500.0)),
        ("image", beat(BeatKind::Image { src: "shot.png".into(), caption: Some("The campaign list".into()), highlight: Some([0.1, 0.1, 0.3, 0.2]), source: Some("launchpoint.app".into()) }, 0.0, 5000.0)),
        ("video", beat(BeatKind::Video { src: "clip.mp4".into(), from: None, caption: Some("Real footage".into()) }, 0.0, 5000.0)),
        ("code", beat(BeatKind::Code { text: "$ claude mcp add poppy --transport http https://mcp.getpoppy.ai\n# then ask:\n> Find the three best hooks in my last 20 videos.".into(), title: Some("Terminal".into()) }, 0.0, 6000.0)),
        ("quote", beat(BeatKind::Quote { text: "The best time to post was yesterday.".into(), by: Some("Every creator".into()) }, 0.0, 4000.0)),
    ];
    let overlays: Vec<(&str, ResolvedBeat)> = vec![
        ("ring", beat(BeatKind::Ring { rect: [0.2, 0.2, 0.2, 0.2] }, 0.0, 3000.0)),
        ("highlight", beat(BeatKind::Highlight { rect: [0.2, 0.5, 0.3, 0.03], underline: false }, 0.0, 3000.0)),
        ("arrow", beat(BeatKind::Arrow { from: [0.6, 0.6], to: [0.75, 0.5] }, 0.0, 3000.0)),
        ("callout", beat(BeatKind::Callout { text: "This is the number that matters".into(), pos: None }, 0.0, 3000.0)),
        ("lower", beat(BeatKind::Lower { title: "Samin Yasar".into(), sub: Some("Founder, AI Answers".into()) }, 0.0, 4000.0)),
    ];
    let ch = ResolvedChapter { n: 3, title: "Design your character".into(), short: "Design".into(), at_ms: 0.0, card: Some((0.0, 1800.0)) };

    println!("{:<12} {:>10} {:>10} {:>10} {:>10}", "item", "1080p ms", "(cold)", "2160p ms", "(cold)");
    let mut rows: Vec<(String, [(f64, f64); 2])> = vec![];
    for (ri, h) in [1080u32, 2160u32].into_iter().enumerate() {
        let w = h * 16 / 9;
        let s = h as f32 / 1080.0;
        let g = Gfx::new(Theme::from_style(&BTreeMap::new()), w, h, &dir)?;
        let mut pm = Pixmap::new(w, h).unwrap();
        let panel = Rect::from_xywh(0.01 * w as f32, 0.0178 * h as f32, 0.7676 * w as f32, 0.9644 * h as f32).unwrap();
        let camera = Rect::from_xywh(0.7966 * w as f32, 0.2126 * h as f32, 0.1843 * w as f32, 0.5747 * h as f32).unwrap();
        let k = (panel.width() / w as f32).max(panel.height() / h as f32);
        let map = ScreenMap { x: panel.left() + (panel.width() - w as f32 * k) * 0.5, y: panel.top(), sx: w as f32 * k, sy: h as f32 * k, clip: panel };
        let (mw, mh) = g.stage_media_size(panel);
        let mut media = Pixmap::new(mw, mh).unwrap();
        media.fill(tiny_skia::Color::from_rgba8(90, 120, 140, 255));
        let mut put = |name: &str, v: (f64, f64)| {
            if let Some(r) = rows.iter_mut().find(|r| r.0 == name) {
                r.1[ri] = v;
            } else {
                let mut a = [(0.0, 0.0); 2];
                a[ri] = v;
                rows.push((name.to_string(), a));
            }
        };
        put("canvas", time(120, 0.0, 4000.0, |t| g.canvas(&mut pm, t)));
        put("shadows", time(120, 0.0, 4000.0, |_| {
            g.slot_shadow(&mut pm, panel, g.slot_radius());
            g.slot_shadow(&mut pm, camera, g.slot_radius());
        }));
        for (name, b) in &stage {
            let m = if *name == "video" { Some(&media) } else { None };
            put(name, time(120, b.start_ms, b.end_ms, |t| g.stage(&mut pm, panel, b, t, m)));
        }
        for (name, b) in &overlays {
            put(name, time(120, b.start_ms, b.end_ms, |t| g.overlay(&mut pm, &map, b, t)));
        }
        put("card", time(60, 0.0, 1800.0, |t| g.chapter_card(&mut pm, &ch, 8, t)));
        let badge = Rect::from_xywh(camera.left(), camera.top() - 166.0 * s, camera.width(), 150.0 * s).unwrap();
        put("badge", time(120, 0.0, 4000.0, |t| g.badge(&mut pm, badge, &ch, 8, t)));
        put("captions", time(120, 0.0, 4000.0, |t| g.captions(&mut pm, panel, &words, t)));
    }
    for (name, v) in rows {
        println!("{:<12} {:>10.3} {:>10.2} {:>10.3} {:>10.2}", name, v[0].0, v[0].1, v[1].0, v[1].1);
    }
    Ok(())
}
