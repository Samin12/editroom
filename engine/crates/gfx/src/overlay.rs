//! Overlays on top of the frame. Ring, highlight and arrow are SCREEN-normalized (they follow
//! the panel and zooms) and clipped to the visible screen; callout and lower thirds are
//! frame-level.

use crate::anim::{back_out, ease_in_out, ease_out, lerp, prog, Clock};
use crate::pal::Pal;
use crate::raster::{self, blit_img, draw_img, fill_rrect, paint, rrect, stroke_partial, IRect, Img};
use crate::text::{tokens, FontId, Style};
use crate::{hash_of, Gfx, ScreenMap};
use cutroom_core::resolve::ResolvedBeat;
use cutroom_core::spec::BeatKind;
use tiny_skia::{FillRule, LineCap, PathBuilder, Pixmap, Stroke, Transform};

pub(crate) fn draw(g: &Gfx, pm: &mut Pixmap, map: &ScreenMap, beat: &ResolvedBeat, t_ms: f64) {
    let ck = Clock::new(t_ms, beat.start_ms, beat.end_ms);
    if ck.lt < 0.0 || ck.lt > ck.dur {
        return;
    }
    let clip = IRect::round(map.clip).intersect(IRect::of(pm));
    let pal = g.pal();
    let s = g.scale;
    match &beat.kind {
        BeatKind::Ring { rect } => ring(pm, map, clip, &pal, s, ck, *rect),
        BeatKind::Highlight { rect, underline } => highlight(pm, map, clip, &pal, s, ck, *rect, *underline),
        BeatKind::Arrow { from, to } => arrow(pm, map, clip, &pal, s, ck, *from, *to),
        BeatKind::Callout { text, pos } => callout(g, pm, map, &pal, ck, text, *pos),
        BeatKind::Lower { title, sub } => lower(g, pm, &pal, ck, title, sub.as_deref()),
        _ => {}
    }
}

fn screen_rect(map: &ScreenMap, r: [f64; 4]) -> [f32; 4] {
    let (x0, y0) = map.px(r[0], r[1]);
    let (x1, y1) = map.px(r[0] + r[2], r[1] + r[3]);
    [x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()]
}

/// Draw with tiny-skia into a scratch layer over `bbox` (clipped), then composite.
fn layered(pm: &mut Pixmap, bbox: IRect, clip: IRect, opacity: f32, f: impl FnOnce(&mut Pixmap, Transform)) {
    let region = bbox.intersect(clip);
    if region.is_empty() || opacity <= 0.0 {
        return;
    }
    if let Some(img) = raster::scratch(region, f) {
        blit_img(pm, &img, region.x0, region.y0, opacity, clip);
    }
}

fn ring(pm: &mut Pixmap, map: &ScreenMap, clip: IRect, pal: &Pal, s: f32, ck: Clock, rect: [f64; 4]) {
    let [x, y, w, h] = screen_rect(map, rect);
    let pad = 10.0 * s;
    let (x, y, w, h) = (x - pad, y - pad, w + 2.0 * pad, h + 2.0 * pad);
    let aspect = w / h.max(1.0);
    let r = if (0.7..1.45).contains(&aspect) { w.min(h) * 0.5 } else { (w.min(h) * 0.32).min(28.0 * s) };
    let p = ease_in_out(prog(ck.lt, 0.0, 420.0));
    let settle = (ck.lt - 420.0).max(0.0);
    let pulse = 1.0 + 0.012 * ((settle / 1300.0 * std::f64::consts::TAU).sin() as f32) * prog(settle, 0.0, 300.0);
    let pop = lerp(1.06, 1.0, ease_out(prog(ck.lt, 0.0, 500.0))) * pulse;
    let (cx, cy) = (x + w * 0.5, y + h * 0.5);
    let (w2, h2) = (w * pop, h * pop);
    let (x2, y2) = (cx - w2 * 0.5, cy - h2 * 0.5);
    let width = 5.0 * s;
    let bbox = IRect::around(x2, y2, w2, h2).outset((14.0 * s).ceil() as i32);
    let len = raster::rrect_len(w2, h2, r);
    let exit = ck.exit();
    layered(pm, bbox, clip, exit, |sp, tr| {
        if let Some(path) = rrect(x2, y2, w2, h2, r) {
            stroke_partial(sp, &path, len, p, width + 5.0 * s, [0, 0, 0], 0.28, tr);
            stroke_partial(sp, &path, len, p, width + 12.0 * s, pal.accent, 0.16, tr);
            stroke_partial(sp, &path, len, p, width, pal.accent, 1.0, tr);
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn highlight(pm: &mut Pixmap, map: &ScreenMap, clip: IRect, pal: &Pal, s: f32, ck: Clock, rect: [f64; 4], underline: bool) {
    let [x, y, w, h] = screen_rect(map, rect);
    let p = ease_out(prog(ck.lt, 0.0, 480.0));
    let exit = ck.exit();
    if underline {
        let th = (7.0 * s).max(2.0);
        let uy = y + h + 4.0 * s;
        let bbox = IRect::around(x - th, uy - th, w + 2.0 * th, 3.0 * th);
        layered(pm, bbox, clip, exit, |sp, tr| {
            if let Some(path) = rrect(x, uy, (w * p).max(th), th, th * 0.5) {
                sp.fill_path(&path, &paint(pal.accent, 1.0), FillRule::Winding, tr, None);
            }
        });
    } else {
        let padx = 6.0 * s;
        let pady = 3.0 * s;
        let bbox = IRect::around(x - padx, y - pady, w + 2.0 * padx, h + 2.0 * pady).outset(2);
        layered(pm, bbox, clip, exit, |sp, tr| {
            if let Some(path) = rrect(x - padx, y - pady, (w + 2.0 * padx) * p, h + 2.0 * pady, 6.0 * s) {
                sp.fill_path(&path, &paint(pal.accent, 0.34), FillRule::Winding, tr, None);
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn arrow(pm: &mut Pixmap, map: &ScreenMap, clip: IRect, pal: &Pal, s: f32, ck: Clock, from: [f64; 2], to: [f64; 2]) {
    let (x0, y0) = map.px(from[0], from[1]);
    let (x1, y1) = map.px(to[0], to[1]);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 2.0 {
        return;
    }
    // A gentle arc: bend to the side that bulges upward.
    let (mut nx, mut ny) = (-dy / len, dx / len);
    if ny > 0.0 {
        nx = -nx;
        ny = -ny;
    }
    let bend = 0.14 * len;
    let (qx, qy) = ((x0 + x1) * 0.5 + nx * bend, (y0 + y1) * 0.5 + ny * bend);
    // Arc length by sampling.
    let mut arc = 0.0;
    let mut prev = (x0, y0);
    for i in 1..=24 {
        let t = i as f32 / 24.0;
        let (a, b) = ((1.0 - t) * (1.0 - t), 2.0 * (1.0 - t) * t);
        let pt = (a * x0 + b * qx + t * t * x1, a * y0 + b * qy + t * t * y1);
        arc += ((pt.0 - prev.0).powi(2) + (pt.1 - prev.1).powi(2)).sqrt();
        prev = pt;
    }
    let p = ease_in_out(prog(ck.lt, 0.0, 450.0));
    let hp = back_out(prog(ck.lt, 330.0, 300.0));
    let width = 6.0 * s;
    let head = 24.0 * s;
    // Tangent at the end of the curve.
    let (tx, ty) = (x1 - qx, y1 - qy);
    let tl = (tx * tx + ty * ty).sqrt().max(1e-3);
    let (ux, uy) = (tx / tl, ty / tl);
    let pad = head + 16.0 * s;
    let bx0 = x0.min(x1).min(qx) - pad;
    let by0 = y0.min(y1).min(qy) - pad;
    let bbox = IRect::around(bx0, by0, x0.max(x1).max(qx) + pad - bx0, y0.max(y1).max(qy) + pad - by0);
    let exit = ck.exit();
    layered(pm, bbox, clip, exit, |sp, tr| {
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y0);
        pb.quad_to(qx, qy, x1, y1);
        let Some(path) = pb.finish() else { return };
        stroke_partial(sp, &path, arc, p, width + 3.0 * s, [0, 0, 0], 0.16, tr);
        stroke_partial(sp, &path, arc, p, width, pal.accent, 1.0, tr);
        if let Some(dot) = PathBuilder::from_circle(x0, y0, 7.0 * s * ease_out(prog(ck.lt, 0.0, 200.0))) {
            sp.fill_path(&dot, &paint(pal.accent, 1.0), FillRule::Winding, tr, None);
        }
        if hp > 0.0 {
            let hl = head * hp;
            let ang = 0.52f32;
            let (c, sn) = (ang.cos(), ang.sin());
            let l = (x1 - hl * (ux * c - uy * sn), y1 - hl * (uy * c + ux * sn));
            let r = (x1 - hl * (ux * c + uy * sn), y1 - hl * (uy * c - ux * sn));
            let mut hb = PathBuilder::new();
            hb.move_to(l.0, l.1);
            hb.line_to(x1, y1);
            hb.line_to(r.0, r.1);
            if let Some(hpath) = hb.finish() {
                let st = Stroke { width: width + 3.0 * s, line_cap: LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Stroke::default() };
                sp.stroke_path(&hpath, &paint([0, 0, 0], 0.16), &st, tr, None);
                let st = Stroke { width, ..st };
                sp.stroke_path(&hpath, &paint(pal.accent, 1.0), &st, tr, None);
            }
        }
    });
}

/// A static chip layer: rounded pill with text, cached.
fn chip(g: &Gfx, pal: &Pal, text: &str, s: f32) -> std::sync::Arc<Img> {
    let key = hash_of(&("callout", text, s.to_bits(), pal));
    g.layer(key, || {
        let tx = g.text();
        let st = Style::new(FontId::SemiBold, 32.0 * s).ls(-0.005);
        let maxw = g.width as f32 * 0.6;
        let b = tx.fit(&tokens(text, false), st, maxw, 2, 200.0 * s, 24.0 * s);
        let (padx, pady) = (30.0 * s, 15.0 * s);
        let (w, h) = ((b.w + 2.0 * padx).ceil(), (b.h() + 2.0 * pady).ceil());
        let shadow = (14.0 * s).ceil();
        let Some(mut pm) = Pixmap::new((w + 2.0 * shadow) as u32, (h + 2.0 * shadow) as u32) else { return Img::default() };
        let sh = g.shadow(w as u32, h as u32, h * 0.5, 16.0 * s, -2.0 * s);
        let full = IRect::of(&pm);
        raster::blit_mask(&mut pm, &sh.sprite, shadow as i32, (shadow + 6.0 * s) as i32, [0, 0, 0], 0.5, full);
        let r = if b.lines > 1 { 22.0 * s } else { h * 0.5 };
        fill_rrect(&mut pm, shadow, shadow, w, h, r, pal.accent, 1.0);
        let mut b = b;
        let bw = b.w;
        b.center(bw);
        let clip = IRect::of(&pm);
        tx.draw(&mut pm, &b, shadow + padx, shadow + pady, &[pal.white], 1.0, clip);
        Img::from_pixmap(pm)
    })
}

fn callout(g: &Gfx, pm: &mut Pixmap, map: &ScreenMap, pal: &Pal, ck: Clock, text: &str, pos: Option<[f64; 2]>) {
    let s = g.scale;
    let img = chip(g, pal, text, s);
    if img.w == 0 {
        return;
    }
    let (fw, fh) = (pm.width() as f32, pm.height() as f32);
    let (w, h) = (img.w as f32, img.h as f32);
    let (mut cx, mut cy) = match pos {
        Some(p) => (p[0] as f32 * fw, p[1] as f32 * fh),
        None => {
            let c = map.clip;
            (c.left() + c.width() * 0.5, c.top() + 44.0 * s + h * 0.5)
        }
    };
    cx = cx.clamp(w * 0.5, (fw - w * 0.5).max(w * 0.5));
    cy = cy.clamp(h * 0.5, (fh - h * 0.5).max(h * 0.5));
    let t = prog(ck.lt, 0.0, 380.0);
    let sc = lerp(0.6, 1.0, back_out(t)) * lerp(0.96, 1.0, ck.exit());
    let op = (t * 2.5).min(1.0) * ck.exit();
    let dst = if (sc - 1.0).abs() < 1e-4 {
        [(cx - w * 0.5).round(), (cy - h * 0.5).round(), w, h]
    } else {
        [cx - w * sc * 0.5, cy - h * sc * 0.5, w * sc, h * sc]
    };
    draw_img(pm, &img, dst, IRect::of(pm), op, None, None);
}

fn lower(g: &Gfx, pm: &mut Pixmap, pal: &Pal, ck: Clock, title: &str, sub: Option<&str>) {
    let s = g.scale;
    let key = hash_of(&("lower", title, sub, s.to_bits(), pal));
    let img = g.layer(key, || {
        let tx = g.text();
        let maxw = 900.0 * s;
        let tb = tx.fit(&tokens(title, false), Style::new(FontId::Bold, 44.0 * s).ls(-0.015).lh(1.15), maxw, 2, 120.0 * s, 30.0 * s);
        let sb = sub.filter(|x| !x.trim().is_empty()).map(|x| tx.fit(&tokens(x, false), Style::new(FontId::Regular, 30.0 * s).lh(1.25), maxw, 2, 90.0 * s, 24.0 * s));
        let (padl, padr, pady) = (40.0 * s, 36.0 * s, 24.0 * s);
        let gap = 6.0 * s;
        let w = (tb.w.max(sb.as_ref().map(|b| b.w).unwrap_or(0.0)) + padl + padr).ceil();
        let h = (tb.h() + sb.as_ref().map(|b| b.h() + gap).unwrap_or(0.0) + 2.0 * pady).ceil();
        let shadow = (24.0 * s).ceil();
        let Some(mut lp) = Pixmap::new((w + 2.0 * shadow) as u32, (h + 2.0 * shadow) as u32) else { return Img::default() };
        let sh = g.shadow(w as u32, h as u32, 18.0 * s, 28.0 * s, -6.0 * s);
        let full = IRect::of(&lp);
        raster::blit_mask(&mut lp, &sh.sprite, shadow as i32, (shadow + 12.0 * s) as i32, [0, 0, 0], 0.55, full);
        fill_rrect(&mut lp, shadow, shadow, w, h, 18.0 * s, pal.card, 0.96);
        raster::stroke_rrect(&mut lp, shadow + 0.5, shadow + 0.5, w - 1.0, h - 1.0, 18.0 * s, 1.0, pal.white, 0.08);
        fill_rrect(&mut lp, shadow + 16.0 * s, shadow + pady, 6.0 * s, h - 2.0 * pady, 3.0 * s, pal.accent, 1.0);
        let clip = IRect::of(&lp);
        tx.draw(&mut lp, &tb, shadow + padl, shadow + pady, &[pal.ink], 1.0, clip);
        if let Some(sb) = &sb {
            tx.draw(&mut lp, sb, shadow + padl, shadow + pady + tb.h() + gap, &[pal.muted], 1.0, clip);
        }
        Img::from_pixmap(lp)
    });
    if img.w == 0 {
        return;
    }
    let (fw, fh) = (pm.width() as f32, pm.height() as f32);
    let shadow = (24.0 * s).ceil();
    let x = 0.035 * fw - shadow;
    let y = fh - 0.075 * fh - img.h as f32 + shadow;
    let p = ease_out(prog(ck.lt, 0.0, 480.0));
    let op = (p * 1.6).min(1.0) * ck.exit();
    let dx = ((1.0 - p) * -60.0 * s).round();
    blit_img(pm, &img, (x + dx).round() as i32, y.round() as i32, op, IRect::of(pm));
}
