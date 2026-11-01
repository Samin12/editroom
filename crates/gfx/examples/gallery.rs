//! Renders every beat kind, the chapter card, badge, captions and the canvas at a few local
//! times onto full 1920x1080 frames (a fake grey "screen" in the panel, a fake camera tile),
//! and writes PNGs plus a contact sheet to /private/tmp/cutroom-gfx-gallery/.
//!
//! `cargo run -p cutroom-gfx --release --example gallery`
//! Set CUTROOM_GALLERY_IMAGE=/path/to/screenshot.png to use a real screenshot for the image
//! beat (its highlight is then the top-left quarter).

use cutroom_core::resolve::{OutWord, ResolvedBeat, ResolvedChapter};
use cutroom_core::spec::{BeatKind, Item};
use cutroom_gfx::{Gfx, Pixmap, Rect, ScreenMap, Theme};
use std::collections::BTreeMap;
use std::path::Path;
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Mask, Paint, PathBuilder, PixmapPaint, Point, SpreadMode, Transform};

const W: u32 = 1920;
const H: u32 = 1080;
const OUT: &str = "/private/tmp/cutroom-gfx-gallery";

fn beat(kind: BeatKind, start: f64, end: f64) -> ResolvedBeat {
    ResolvedBeat { idx: 0, lane: kind.lane(), kind, start_ms: start, end_ms: end, items_ms: vec![], words: vec![], origin: "gallery".into() }
}

fn words(text: &str, start: f64, per: f64) -> Vec<OutWord> {
    text.split_whitespace().enumerate().map(|(i, w)| OutWord { id: i, text: w.to_string(), start_ms: start + i as f64 * per, end_ms: start + i as f64 * per + per * 0.8 }).collect()
}

fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    p
}

fn rr(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
    let mut pb = PathBuilder::new();
    let k = 0.5523 * r;
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish().unwrap()
}

/// A light, browser-like mock UI (the "screen recording").
fn fake_screen() -> Pixmap {
    let mut s = Pixmap::new(W, H).unwrap();
    s.fill(Color::from_rgba8(236, 236, 234, 255));
    let id = Transform::identity();
    let fill = |s: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: (u8, u8, u8)| {
        s.fill_path(&rr(x, y, w, h, r), &paint(Color::from_rgba8(c.0, c.1, c.2, 255)), FillRule::Winding, id, None);
    };
    fill(&mut s, 0.0, 0.0, 1920.0, 70.0, 0.0, (214, 214, 212));
    for (i, c) in [(237u8, 106u8, 94u8), (245, 191, 79), (98, 197, 84)].iter().enumerate() {
        s.fill_path(&PathBuilder::from_circle(30.0 + i as f32 * 26.0, 35.0, 8.0).unwrap(), &paint(Color::from_rgba8(c.0, c.1, c.2, 255)), FillRule::Winding, id, None);
    }
    fill(&mut s, 400.0, 18.0, 1120.0, 34.0, 17.0, (246, 246, 245));
    fill(&mut s, 0.0, 70.0, 300.0, 1010.0, 0.0, (226, 226, 224));
    for i in 0..9 {
        fill(&mut s, 32.0, 120.0 + i as f32 * 56.0, 200.0 - (i % 3) as f32 * 30.0, 18.0, 9.0, (190, 190, 188));
    }
    fill(&mut s, 360.0, 120.0, 900.0, 44.0, 10.0, (60, 60, 64));
    for i in 0..3 {
        let x = 360.0 + i as f32 * 420.0;
        fill(&mut s, x, 210.0, 390.0, 240.0, 16.0, (255, 255, 255));
        fill(&mut s, x + 28.0, 240.0, 160.0, 18.0, 9.0, (170, 170, 168));
        fill(&mut s, x + 28.0, 290.0, 240.0, 56.0, 10.0, (40, 40, 44));
        fill(&mut s, x + 28.0, 380.0, 300.0, 14.0, 7.0, (205, 205, 203));
    }
    fill(&mut s, 360.0, 490.0, 1230.0, 520.0, 16.0, (255, 255, 255));
    for i in 0..8 {
        let y = 530.0 + i as f32 * 58.0;
        fill(&mut s, 400.0, y, 520.0 - (i % 4) as f32 * 60.0, 20.0, 10.0, (120, 120, 118));
        fill(&mut s, 1100.0, y, 200.0, 20.0, 10.0, (190, 190, 188));
        fill(&mut s, 1380.0, y, 150.0, 20.0, 10.0, (98, 160, 120));
    }
    s
}

/// A dark "product dashboard" still for the image beat when no real screenshot is given.
fn fake_shot(out: &Path) {
    let (w, h) = (1600u32, 1000u32);
    let mut s = Pixmap::new(w, h).unwrap();
    s.fill(Color::from_rgba8(250, 250, 248, 255));
    let id = Transform::identity();
    let fill = |s: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: (u8, u8, u8)| {
        s.fill_path(&rr(x, y, w, h, r), &paint(Color::from_rgba8(c.0, c.1, c.2, 255)), FillRule::Winding, id, None);
    };
    fill(&mut s, 0.0, 0.0, 1600.0, 90.0, 0.0, (24, 24, 28));
    fill(&mut s, 40.0, 30.0, 220.0, 30.0, 15.0, (99, 91, 255));
    for (i, (v, c)) in [(0.82f32, (99u8, 91u8, 255u8)), (0.55, (0, 212, 170)), (0.68, (255, 170, 60))].iter().enumerate() {
        let x = 60.0 + i as f32 * 510.0;
        fill(&mut s, x, 140.0, 470.0, 260.0, 18.0, (255, 255, 255));
        fill(&mut s, x + 30.0, 170.0, 150.0, 22.0, 11.0, (160, 160, 170));
        fill(&mut s, x + 30.0, 220.0, 260.0, 64.0, 12.0, (30, 30, 36));
        fill(&mut s, x + 30.0, 330.0, 410.0 * v, 26.0, 13.0, *c);
    }
    fill(&mut s, 60.0, 440.0, 1490.0, 500.0, 18.0, (255, 255, 255));
    let mut pb = PathBuilder::new();
    pb.move_to(100.0, 880.0);
    for i in 0..14 {
        let x = 100.0 + i as f32 * 104.0;
        let y = 880.0 - (i as f32 * 26.0) - ((i * 37) % 5) as f32 * 30.0;
        pb.line_to(x, y);
    }
    let path = pb.finish().unwrap();
    s.stroke_path(&path, &paint(Color::from_rgba8(99, 91, 255, 255)), &tiny_skia::Stroke { width: 6.0, ..Default::default() }, id, None);
    s.save_png(out).unwrap();
}

/// A fake decoded video frame (warm gradient + shapes) at the size gfx asks for.
fn fake_video(w: u32, h: u32, t: f32) -> Pixmap {
    let mut v = Pixmap::new(w, h).unwrap();
    let grad = LinearGradient::new(
        Point::from_xy(0.0, 0.0),
        Point::from_xy(w as f32, h as f32),
        vec![GradientStop::new(0.0, Color::from_rgba8(36, 74, 92, 255)), GradientStop::new(1.0, Color::from_rgba8(214, 140, 92, 255))],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap();
    v.fill_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, w as f32, h as f32).unwrap(), &Paint { shader: grad, ..Default::default() }, Transform::identity(), None);
    let c = PathBuilder::from_circle(w as f32 * (0.3 + 0.1 * t), h as f32 * 0.55, h as f32 * 0.22).unwrap();
    v.fill_path(&c, &paint(Color::from_rgba8(250, 226, 190, 220)), FillRule::Winding, Transform::identity(), None);
    v
}

fn fake_camera() -> Pixmap {
    let (w, h) = (354u32, 621u32);
    let mut c = Pixmap::new(w, h).unwrap();
    let grad = LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(0.0, h as f32), vec![GradientStop::new(0.0, Color::from_rgba8(70, 62, 56, 255)), GradientStop::new(1.0, Color::from_rgba8(38, 34, 31, 255))], SpreadMode::Pad, Transform::identity()).unwrap();
    c.fill_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, w as f32, h as f32).unwrap(), &Paint { shader: grad, ..Default::default() }, Transform::identity(), None);
    let head = PathBuilder::from_circle(177.0, 250.0, 78.0).unwrap();
    c.fill_path(&head, &paint(Color::from_rgba8(176, 132, 104, 255)), FillRule::Winding, Transform::identity(), None);
    c.fill_path(&rr(62.0, 360.0, 230.0, 300.0, 100.0), &paint(Color::from_rgba8(30, 46, 60, 255)), FillRule::Winding, Transform::identity(), None);
    c
}

/// Draw `src` into the rounded `slot` with cover fit (stand-in for the renderer's video blit).
fn blit_slot(pm: &mut Pixmap, src: &Pixmap, slot: Rect, radius: f32) {
    let k = (slot.width() / src.width() as f32).max(slot.height() / src.height() as f32);
    let (dw, dh) = (src.width() as f32 * k, src.height() as f32 * k);
    let tr = Transform::from_row(k, 0.0, 0.0, k, slot.left() + (slot.width() - dw) * 0.5, slot.top() + (slot.height() - dh) * 0.5);
    let mut mask = Mask::new(pm.width(), pm.height()).unwrap();
    mask.fill_path(&rr(slot.left(), slot.top(), slot.width(), slot.height(), radius), FillRule::Winding, true, Transform::identity());
    pm.draw_pixmap(0, 0, src.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() }, tr, Some(&mask));
}

struct Scene {
    gfx: Gfx,
    panel: Rect,
    camera: Rect,
    badge: Rect,
    screen: Pixmap,
    cam: Pixmap,
    map: ScreenMap,
    chapter: ResolvedChapter,
}

impl Scene {
    fn base(&self, t: f64, stage: Option<(&ResolvedBeat, Option<&Pixmap>)>) -> Pixmap {
        let mut pm = Pixmap::new(W, H).unwrap();
        let g = &self.gfx;
        g.canvas(&mut pm, t);
        g.slot_shadow(&mut pm, self.panel, g.slot_radius());
        g.slot_shadow(&mut pm, self.camera, g.slot_radius());
        match stage {
            Some((b, m)) => g.stage(&mut pm, self.panel, b, t, m),
            None => blit_slot(&mut pm, &self.screen, self.panel, g.slot_radius()),
        }
        blit_slot(&mut pm, &self.cam, self.camera, g.slot_radius());
        g.badge(&mut pm, self.badge, &self.chapter, 8, t);
        pm
    }
}

fn main() -> anyhow::Result<()> {
    std::fs::create_dir_all(OUT)?;
    let assets = Path::new(OUT).join("assets");
    std::fs::create_dir_all(&assets)?;
    let gfx = Gfx::new(Theme::from_style(&BTreeMap::new()), W, H, &assets)?;
    let px = |r: [f32; 4]| Rect::from_xywh(r[0] * W as f32, r[1] * H as f32, r[2] * W as f32, r[3] * H as f32).unwrap();
    let panel = px([0.01, 0.0178, 0.7676, 0.9644]);
    let camera = px([0.7966, 0.2126, 0.1843, 0.5747]);
    let badge = Rect::from_xywh(camera.left(), camera.top() - 166.0, camera.width(), 150.0).unwrap();
    // Cover-fit of a 1920x1080 screen into the panel.
    let k = (panel.width() / W as f32).max(panel.height() / H as f32);
    let map = ScreenMap { x: panel.left() + (panel.width() - W as f32 * k) * 0.5, y: panel.top() + (panel.height() - H as f32 * k) * 0.5, sx: W as f32 * k, sy: H as f32 * k, clip: panel };
    let chapter = ResolvedChapter { n: 3, title: "Design your character".into(), short: "Design".into(), at_ms: 0.0, card: Some((0.0, 1800.0)) };
    let sc = Scene { gfx, panel, camera, badge, screen: fake_screen(), cam: fake_camera(), map, chapter };
    let g = &sc.gfx;

    let (img_src, hl) = match std::env::var("CUTROOM_GALLERY_IMAGE") {
        Ok(p) => (p, [0.0, 0.0, 0.48, 0.18]),
        Err(_) => {
            fake_shot(&assets.join("shot.png"));
            ("shot.png".to_string(), [0.04, 0.14, 0.3, 0.26])
        }
    };

    let mut frames: Vec<(String, Pixmap)> = vec![];
    let mut push = |name: &str, pm: Pixmap| frames.push((name.to_string(), pm));

    // Canvas alone.
    let mut pm = Pixmap::new(W, H).unwrap();
    g.canvas(&mut pm, 0.0);
    push("00-canvas", pm);
    push("01-screen", sc.base(4000.0, None));

    // Title.
    let mut title = beat(BeatKind::Title { text: "Connect *Claude* to your whole stack".into(), kicker: Some("Step 2".into()), sub: Some("One prompt wires up Poppy, Higgsfield and Gmail.".into()) }, 0.0, 4000.0);
    title.words = words("so we connect Claude to your whole stack today", 130.0, 260.0);
    for t in [250.0, 900.0, 2600.0] {
        push(&format!("10-title-{t:04}"), sc.base(t, Some((&title, None))));
    }
    let long = beat(BeatKind::Title { text: "This is an extremely long headline that has to shrink to fit inside the panel without ever overflowing it at all".into(), kicker: None, sub: None }, 0.0, 4000.0);
    push("11-title-long", sc.base(3000.0, Some((&long, None))));

    // Bullets.
    let mut bullets = beat(BeatKind::Bullets { title: Some("What the agent does for you".into()), items: vec![Item::Text("Finds the idea in your *comments*".into()), Item::Text("Writes the script in your voice".into()), Item::Text("Cuts the video and adds the graphics".into()), Item::Text("Publishes and reports back".into())] }, 0.0, 6000.0);
    bullets.items_ms = vec![400.0, 1200.0, 2000.0, 2800.0];
    push("20-bullets-1500", sc.base(1500.0, Some((&bullets, None))));
    push("21-bullets-4500", sc.base(4500.0, Some((&bullets, None))));

    // Number.
    let number = beat(BeatKind::Number { value: "$10,075".into(), label: Some("paid for a single AI video".into()) }, 0.0, 3500.0);
    push("30-number-0450", sc.base(450.0, Some((&number, None))));
    push("31-number-2500", sc.base(2500.0, Some((&number, None))));
    let compact = beat(BeatKind::Number { value: "4.76M".into(), label: Some("views on one post".into()) }, 0.0, 3500.0);
    push("32-number-compact", sc.base(2500.0, Some((&compact, None))));

    // Image.
    let image = beat(BeatKind::Image { src: img_src.clone(), caption: Some("The campaign list pays per video".into()), highlight: Some(hl), source: Some("launchpoint.app".into()) }, 0.0, 5000.0);
    push("40-image-0200", sc.base(200.0, Some((&image, None))));
    push("41-image-3000", sc.base(3000.0, Some((&image, None))));
    let missing = beat(BeatKind::Image { src: "nope/missing.png".into(), caption: None, highlight: None, source: None }, 0.0, 5000.0);
    push("42-image-missing", sc.base(2000.0, Some((&missing, None))));

    // Video.
    let video = beat(BeatKind::Video { src: "clip.mp4".into(), from: None, caption: Some("Gaston rides the wave".into()) }, 0.0, 5000.0);
    let (vw, vh) = g.stage_media_size(panel);
    let vf = fake_video(vw, vh, 0.5);
    push("50-video-2000", sc.base(2000.0, Some((&video, Some(&vf)))));

    // Code.
    let code = beat(BeatKind::Code { text: "$ claude mcp add poppy --transport http https://mcp.getpoppy.ai\n# then ask:\n> Find the three best-performing hooks in my last 20 videos and draft five new ones in my voice.".into(), title: Some("Terminal".into()) }, 0.0, 6000.0);
    push("60-code-1600", sc.base(1600.0, Some((&code, None))));
    push("61-code-5000", sc.base(5000.0, Some((&code, None))));

    // Quote.
    let quote = beat(BeatKind::Quote { text: "The best time to post was yesterday. The *second best* is today.".into(), by: Some("Every creator, ever".into()) }, 0.0, 4000.0);
    push("70-quote-2500", sc.base(2500.0, Some((&quote, None))));

    // Overlays on the fake screen.
    let mut pm = sc.base(1500.0, None);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Ring { rect: [0.185, 0.19, 0.205, 0.23] }, 0.0, 3000.0), 1500.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Highlight { rect: [0.205, 0.485, 0.27, 0.025], underline: false }, 0.0, 3000.0), 1500.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Highlight { rect: [0.205, 0.54, 0.24, 0.025], underline: true }, 0.0, 3000.0), 1500.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Arrow { from: [0.62, 0.62], to: [0.73, 0.505] }, 0.0, 3000.0), 1500.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Callout { text: "This is the number that matters".into(), pos: None }, 0.0, 3000.0), 1500.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Lower { title: "Samin Yasar".into(), sub: Some("Founder, AI Answers".into()) }, 0.0, 4000.0), 1500.0);
    push("80-overlays-settled", pm);
    let mut pm = sc.base(250.0, None);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Ring { rect: [0.185, 0.19, 0.205, 0.23] }, 0.0, 3000.0), 250.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Arrow { from: [0.62, 0.62], to: [0.73, 0.505] }, 0.0, 3000.0), 250.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Callout { text: "This is the number that matters".into(), pos: None }, 0.0, 3000.0), 150.0);
    g.overlay(&mut pm, &sc.map, &beat(BeatKind::Lower { title: "Samin Yasar".into(), sub: Some("Founder, AI Answers".into()) }, 0.0, 4000.0), 200.0);
    push("81-overlays-drawing", pm);

    // Chapter card.
    for t in [150.0, 700.0, 1300.0] {
        let mut pm = Pixmap::new(W, H).unwrap();
        g.chapter_card(&mut pm, &sc.chapter, 8, t);
        push(&format!("90-card-{t:04}"), pm);
    }
    let mut pm = Pixmap::new(W, H).unwrap();
    let ch0 = ResolvedChapter { n: 4, title: "A chapter without a tracker".into(), short: "Bonus".into(), at_ms: 0.0, card: Some((0.0, 1800.0)) };
    g.chapter_card(&mut pm, &ch0, 0, 1300.0);
    push("91-card-no-total", pm);

    // Badge pop + pill variant + captions.
    let mut pm = sc.base(1900.0, None);
    let ch5 = ResolvedChapter { n: 5, title: "Recreate viral videos".into(), short: "Go viral".into(), at_ms: 0.0, card: None };
    g.captions(&mut pm, panel, &words("and this is where it gets interesting because the agent does all of it", 1000.0, 230.0), 2000.0);
    g.badge(&mut pm, Rect::from_xywh(camera.left(), camera.bottom() + 20.0, camera.width(), 56.0).unwrap(), &ch5, 8, 5000.0);
    push("95-captions-badges", pm);
    // The badge popping in right after a chapter card ends (card 0..1800 ms).
    push("96-badge-pop", sc.base(1880.0, None));
    // Exit fades.
    push("97-title-exit", sc.base(3880.0, Some((&title, None))));
    push("98-code-typing", sc.base(900.0, Some((&code, None))));

    // One 2160p frame to check the scaling (saved on its own, not in the sheet).
    {
        let (w4, h4) = (3840u32, 2160u32);
        let g4 = Gfx::new(Theme::from_style(&BTreeMap::new()), w4, h4, &assets)?;
        let mut pm = Pixmap::new(w4, h4).unwrap();
        let p4 = Rect::from_xywh(0.01 * w4 as f32, 0.0178 * h4 as f32, 0.7676 * w4 as f32, 0.9644 * h4 as f32).unwrap();
        g4.canvas(&mut pm, 2600.0);
        g4.slot_shadow(&mut pm, p4, g4.slot_radius());
        g4.stage(&mut pm, p4, &bullets, 4500.0, None);
        let b4 = Rect::from_xywh(0.7966 * w4 as f32, 0.2126 * h4 as f32 - 332.0, 0.1843 * w4 as f32, 300.0).unwrap();
        g4.badge(&mut pm, b4, &sc.chapter, 8, 5000.0);
        pm.save_png(format!("{OUT}/zz-bullets-2160p.png"))?;
    }

    // Write PNGs and the contact sheet.
    for (name, pm) in &frames {
        pm.save_png(format!("{OUT}/{name}.png"))?;
    }
    let cols = 4u32;
    let (tw, th) = (480u32, 270u32);
    let rows = (frames.len() as u32).div_ceil(cols);
    let gap = 12u32;
    let mut sheet = Pixmap::new(cols * (tw + gap) + gap, rows * (th + gap) + gap).unwrap();
    sheet.fill(Color::from_rgba8(40, 40, 40, 255));
    for (i, (_, pm)) in frames.iter().enumerate() {
        let (c, r) = (i as u32 % cols, i as u32 / cols);
        let tr = Transform::from_row(tw as f32 / W as f32, 0.0, 0.0, th as f32 / H as f32, (gap + c * (tw + gap)) as f32, (gap + r * (th + gap)) as f32);
        sheet.draw_pixmap(0, 0, pm.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() }, tr, None);
    }
    sheet.save_png(format!("{OUT}/contact-sheet.png"))?;
    println!("wrote {} frames + contact-sheet.png to {OUT}", frames.len());
    Ok(())
}
