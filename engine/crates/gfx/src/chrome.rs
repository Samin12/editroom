//! Frame chrome: the full-frame chapter card, the persistent step badge, burned-in captions.

use crate::anim::{back_out, ease_in_out, ease_out, lerp, prog, Clock};
use crate::canvas;
use crate::pal::{Pal, Rgb};
use crate::raster::{self, blit_img, draw_img, fill_circle, fill_rrect, paint, stroke_rrect, IRect, Img};
use crate::text::{tokens, FontId, Style, Tok};
use crate::{hash_of, Gfx};
use cutroom_core::resolve::{OutWord, ResolvedChapter};
use tiny_skia::{Color, LineCap, LineJoin, PathBuilder, Pixmap, Rect, Stroke, Transform};

/// The card background: canvas colour with a soft warm glow (dots are stamped per frame).
fn card_bg(w: u32, h: u32, pal: &Pal) -> Img {
    let Some(mut pm) = Pixmap::new(w.max(1), h.max(1)) else { return Img::default() };
    pm.fill(Color::from_rgba8(pal.canvas[0], pal.canvas[1], pal.canvas[2], 255));
    let (wf, hf) = (w as f32, h as f32);
    canvas::add_glow(&mut pm, wf * 0.36, hf * 0.45, wf * 0.62, hf * 0.85, pal.accent, 0.075);
    Img::from_pixmap(pm)
}

pub(crate) fn chapter_card(g: &Gfx, pm: &mut Pixmap, ch: &ResolvedChapter, total: usize, t_ms: f64) {
    let (a, b) = ch.card.unwrap_or((ch.at_ms, ch.at_ms + 1800.0));
    let mut ck = Clock::new(t_ms, a, b);
    ck.lt = ck.lt.clamp(0.0, ck.dur);
    let pal = g.pal();
    let s = g.scale;
    let (w, h) = (pm.width(), pm.height());
    let full = IRect::of(pm);
    let bg = g.layer(hash_of(&("cardbg", w, h, &pal)), || card_bg(w, h, &pal));
    blit_img(pm, &bg, 0, 0, 1.0, full);
    let pitch = (g.theme.dot_pitch * s).round().max(4.0);
    canvas::stamp_dots(pm, &g.dot_sprites(1.6 * s), full, pitch, g.drift(t_ms), pal.dot, 1.0, None);

    let tx = g.text();
    let lt = ck.lt;
    let out = 1.0 - ease_in_out(prog(lt, ck.dur - 300.0, 300.0));
    let lift = (1.0 - out) * -14.0 * s;
    let n = ch.n.max(1);

    // Giant ghost step number.
    let ghost = ease_out(prog(lt, 120.0, 900.0));
    let gst = Style::new(FontId::Bold, 560.0 * s).ls(-0.06);
    let base = 120.0 * s + 0.863 * gst.size;
    tx.draw_line(pm, gst, &format!("{n:02}"), 60.0 * s - (1.0 - ghost) * 40.0 * s, base + lift, pal.accent_soft, ghost * out, full);

    // Kicker with its rule.
    let kp = ease_out(prog(lt, 200.0, 420.0));
    let kicker = if total > 0 { format!("STEP {n} OF {total}") } else { format!("CHAPTER {n}") };
    let kst = Style::new(FontId::Bold, 30.0 * s).ls(0.16);
    let ky = 300.0 * s + lift;
    fill_rrect(pm, 132.0 * s, ky + 15.0 * s - 2.0 * s, 48.0 * s * kp, 4.0 * s, 2.0 * s, pal.accent, out);
    tx.draw_line(pm, kst, &kicker, 132.0 * s + 66.0 * s - (1.0 - kp) * 16.0 * s, ky + 15.0 * s + 0.36 * kst.size, pal.accent, kp * out, full);

    // Title, building word by word.
    let title = ch.title.trim();
    let toks = tokens(title, true);
    let pref = if title.chars().count() > 26 { 104.0 } else { 124.0 };
    let tb = tx.fit(&toks, Style::new(FontId::Bold, pref * s).ls(-0.035).lh(1.04), 1560.0 * s, 3, 400.0 * s, 64.0 * s);
    let ty = 356.0 * s + lift;
    for (i, wd) in tb.words.iter().enumerate() {
        let p = ease_out(prog(lt, 300.0 + 80.0 * i as f64, 420.0));
        let dy = (1.0 - p) * 0.3 * tb.style.size;
        let col = if wd.tag == 1 { pal.accent } else { pal.ink };
        tx.draw_word(pm, &tb, wd, 128.0 * s, ty + dy, col, p * out, full);
    }

    if total > 0 {
        tracker(g, pm, &pal, n.min(total), total, lt, 150.0 * s, (828.0 * s).max(ty + tb.h() + 90.0 * s) + lift, 1180.0 * s, out);
    }

    // Accent wipe uncovering the card.
    let wipe = ease_in_out(prog(lt, 0.0, 450.0));
    if wipe < 1.0 {
        raster::fill_rect(pm, IRect::new(0, 0, (w as f32 * (1.0 - wipe)).round() as i32, h as i32), pal.accent);
    }
}

/// The step tracker: done steps filled with a check, the current one in accent with a pulse,
/// later ones hollow.
#[allow(clippy::too_many_arguments)]
fn tracker(g: &Gfx, pm: &mut Pixmap, pal: &Pal, cur: usize, total: usize, lt: f64, x: f32, y: f32, width: f32, out: f32) {
    let s = g.scale;
    let tx = g.text();
    let node = (50.0 * s).min(width / (total as f32 * 1.5)).max(10.0 * s);
    let gap = if total > 1 { width / (total - 1) as f32 } else { 0.0 };
    let cy = y + node * 0.5;
    let track_p = ease_out(prog(lt, 250.0, 450.0));
    let fp = ease_in_out(prog(lt, 520.0, 600.0));
    let start = cur.saturating_sub(1) as f32;
    let fill_pos = lerp((start - 1.0).max(0.0), (cur as f32 - 1.0).max(0.0), fp);
    let th = (8.0 * s).max(2.0);
    if total > 1 {
        fill_rrect(pm, x, cy - th * 0.5, width * track_p, th, th * 0.5, pal.line, out);
        fill_rrect(pm, x, cy - th * 0.5, gap * fill_pos * track_p, th, th * 0.5, pal.accent, out);
    }
    let full = IRect::of(pm);
    let nst = Style::new(FontId::Bold, node * 0.42).tnum();
    for i in 0..total {
        let step = i + 1;
        let rp = ease_out(prog(lt, 250.0 + 50.0 * i as f64, 350.0));
        if rp <= 0.0 {
            continue;
        }
        let reached = fill_pos >= i as f32 - 0.02;
        let is_cur = step == cur && fp > 0.85;
        let is_done = step < cur && reached;
        let nx = x + i as f32 * gap;
        let r = node * 0.5 * lerp(0.5, 1.0, rp) * if is_cur { 1.18 } else { 1.0 };
        let op = rp * out;
        if is_cur {
            let pulse = ((lt % 1500.0) / 1500.0) as f32;
            if let Some(c) = PathBuilder::from_circle(nx, cy, r * (1.0 + pulse * 0.7)) {
                let st = Stroke { width: 4.0 * s, ..Stroke::default() };
                pm.stroke_path(&c, &paint(pal.accent, 0.55 * (1.0 - pulse) * op), &st, Transform::identity(), None);
            }
            fill_circle(pm, nx, cy, r, pal.accent, op);
        } else if is_done {
            fill_circle(pm, nx, cy, r, pal.ink, op);
            let k = r * 2.0 * 0.56 / 24.0;
            let (ox, oy) = (nx - 12.0 * k, cy - 12.0 * k);
            let mut pb = PathBuilder::new();
            pb.move_to(ox + 5.0 * k, oy + 12.5 * k);
            pb.line_to(ox + 10.0 * k, oy + 17.5 * k);
            pb.line_to(ox + 19.0 * k, oy + 7.5 * k);
            if let Some(p) = pb.finish() {
                let st = Stroke { width: 3.2 * k, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
                pm.stroke_path(&p, &paint(pal.canvas, op), &st, Transform::identity(), None);
            }
            continue;
        } else {
            fill_circle(pm, nx, cy, r, pal.card, op);
            if let Some(c) = PathBuilder::from_circle(nx, cy, r - 2.0 * s) {
                let st = Stroke { width: 4.0 * s, ..Stroke::default() };
                pm.stroke_path(&c, &paint(pal.line, op), &st, Transform::identity(), None);
            }
        }
        let label = step.to_string();
        let lw = tx.width(nst, &label);
        let col = if is_cur { pal.white } else { pal.faint };
        tx.draw_line(pm, nst, &label, nx - lw * 0.5, cy + 0.36 * nst.size, col, op, full);
    }
}

/// The badge layer: a pill ("STEP n" + short title) for flat rects, a small card with a
/// segment bar for taller ones.
fn badge_layer(g: &Gfx, w: u32, h: u32, ch: &ResolvedChapter, total: usize, pal: &Pal) -> Img {
    let Some(mut pm) = Pixmap::new(w.max(1), h.max(1)) else { return Img::default() };
    let tx = g.text();
    let (wf, hf) = (w as f32, h as f32);
    let clip = IRect::of(&pm);
    let bg: Rgb = [38, 36, 33];
    let short = if ch.short.trim().is_empty() { ch.title.trim() } else { ch.short.trim() };
    if hf < wf * 0.3 || total == 0 {
        // Pill.
        let ph = hf.min(wf * 0.3);
        let py = (hf - ph) * 0.5;
        let k = ph / 64.0;
        fill_rrect(&mut pm, 1.0, py + 1.0, wf - 2.0, ph - 2.0, ph * 0.5, bg, 0.94);
        stroke_rrect(&mut pm, 1.5, py + 1.5, wf - 3.0, ph - 3.0, ph * 0.5, 1.5 * k.max(0.5), pal.white, 0.16);
        let padx = ph * 0.42;
        let mut kst = Style::new(FontId::Bold, 17.0 * k).ls(0.14);
        let mut tst = Style::new(FontId::SemiBold, 22.0 * k).ls(-0.01);
        let kick = format!("STEP {}", ch.n);
        let dot = 9.0 * k;
        let gap = 12.0 * k;
        loop {
            let need = padx * 2.0 + dot + gap * 2.0 + tx.width(kst, &kick) + tx.width(tst, short);
            if need <= wf || tst.size < 6.0 {
                break;
            }
            kst = kst.sized(kst.size * 0.95);
            tst = tst.sized(tst.size * 0.95);
        }
        let cy = py + ph * 0.5;
        fill_circle(&mut pm, padx + dot * 0.5, cy, dot * 0.5, pal.accent, 1.0);
        let mut x = padx + dot + gap;
        tx.draw_line(&mut pm, kst, &kick, x, cy + 0.36 * kst.size, pal.accent, 1.0, clip);
        x += tx.width(kst, &kick) + gap;
        tx.draw_line(&mut pm, tst, short, x, cy + 0.36 * tst.size, pal.ink, 1.0, clip);
    } else {
        // Card (authored at 354 x 150).
        let k = (wf / 354.0).min(hf / 150.0);
        let inset = 6.0 * k;
        let (cw, chh) = (wf - 2.0 * inset, hf - 2.0 * inset);
        fill_rrect(&mut pm, inset, inset, cw, chh, 22.0 * k, bg, 0.94);
        stroke_rrect(&mut pm, inset + 0.75 * k, inset + 0.75 * k, cw - 1.5 * k, chh - 1.5 * k, 22.0 * k, 1.5 * k, pal.white, 0.16);
        let (px, py) = (inset + 20.0 * k, inset + 16.0 * k);
        let inner_w = cw - 40.0 * k;
        let kst = Style::new(FontId::Bold, 17.0 * k).ls(0.16);
        let kick = format!("STEP {} OF {}", ch.n, total);
        fill_circle(&mut pm, px + 5.0 * k, py + 10.0 * k, 5.0 * k, pal.accent, 1.0);
        tx.draw_line(&mut pm, kst, &kick, px + 20.0 * k, py + 10.0 * k + 0.36 * kst.size, pal.accent, 1.0, clip);
        let seg_h = 7.0 * k;
        let seg_y = inset + chh - 16.0 * k - seg_h;
        let title_top = py + 30.0 * k;
        let avail = seg_y - 8.0 * k - title_top;
        let pref = if short.chars().count() > 24 { 23.0 } else { 26.0 };
        let tb = tx.fit(&tokens(short, false), Style::new(FontId::Bold, pref * k).ls(-0.02).lh(1.08), inner_w, 2, avail, 14.0 * k);
        tx.draw(&mut pm, &tb, px, title_top + (avail - tb.h()) * 0.5, &[pal.ink], 1.0, clip);
        let sgap = 6.0 * k;
        let sw = (inner_w - sgap * (total as f32 - 1.0)) / total as f32;
        for i in 0..total {
            let step = i + 1;
            let (col, a) = if step < ch.n {
                (pal.ink, 1.0)
            } else if step == ch.n {
                (pal.accent, 1.0)
            } else {
                (pal.white, 0.18)
            };
            fill_rrect(&mut pm, px + i as f32 * (sw + sgap), seg_y, sw.max(1.0), seg_h, seg_h * 0.5, col, a);
        }
    }
    Img::from_pixmap(pm)
}

pub(crate) fn badge(g: &Gfx, pm: &mut Pixmap, rect: Rect, ch: &ResolvedChapter, total: usize, t_ms: f64) {
    let r = IRect::round(rect);
    if r.is_empty() {
        return;
    }
    let pal = g.pal();
    let short = ch.short.clone();
    let key = hash_of(&("badge", r.w(), r.h(), ch.n, total, &short, &ch.title, &pal));
    let img = g.layer(key, || badge_layer(g, r.w() as u32, r.h() as u32, ch, total, &pal));
    let t0 = ch.card.map(|c| c.1).unwrap_or(ch.at_ms);
    let dt = t_ms - t0;
    let (sc, op) = if (0.0..500.0).contains(&dt) { (lerp(0.86, 1.0, back_out(prog(dt, 0.0, 450.0))), prog(dt, 0.0, 160.0)) } else { (1.0, 1.0) };
    let (w, h) = (img.w as f32, img.h as f32);
    if (sc - 1.0).abs() < 1e-4 && op >= 1.0 {
        blit_img(pm, &img, r.x0, r.y0, 1.0, IRect::of(pm));
    } else {
        let (cx, cy) = (r.x0 as f32 + w * 0.5, r.y0 as f32 + h * 0.5);
        draw_img(pm, &img, [cx - w * sc * 0.5, cy - h * sc * 0.5, w * sc, h * sc], IRect::of(pm), op, None, None);
    }
}

fn ends_sentence(t: &str) -> bool {
    let t = t.trim_end_matches(['"', '\'', ')', '\u{201D}']);
    t.ends_with('.') || t.ends_with('?') || t.ends_with('!')
}

fn ends_clause(t: &str) -> bool {
    t.ends_with(',') || t.ends_with(';') || t.ends_with(':')
}

/// The caption group (word index range) containing word `i`: groups of up to 6 words, broken
/// at pauses and sentence ends, anchored so the grouping never depends on `t`.
fn caption_group(words: &[OutWord], i: usize) -> (usize, usize) {
    let hard = |k: usize| k == 0 || k.is_multiple_of(64) || words[k].start_ms - words[k - 1].end_ms > 700.0 || ends_sentence(&words[k - 1].text);
    let mut a = i;
    while !hard(a) {
        a -= 1;
    }
    loop {
        let mut b = a + 1;
        while b < words.len() && b - a < 6 && !hard(b) && !(b - a >= 3 && ends_clause(&words[b - 1].text)) {
            b += 1;
        }
        if i < b {
            return (a, b);
        }
        a = b;
    }
}

pub(crate) fn captions(g: &Gfx, pm: &mut Pixmap, area: Rect, words: &[OutWord], t_ms: f64) {
    if words.is_empty() {
        return;
    }
    let idx = words.partition_point(|w| w.start_ms <= t_ms);
    if idx == 0 {
        return;
    }
    let cur = idx - 1;
    let (a, b) = caption_group(words, cur);
    if t_ms > words[b - 1].end_ms + 600.0 {
        return;
    }
    let s = g.scale;
    let pal = g.pal();
    let tx = g.text();
    let toks: Vec<Tok> = words[a..b].iter().map(|w| Tok { text: w.text.trim().to_string(), tag: 0, brk: false }).filter(|t| !t.text.is_empty()).collect();
    if toks.is_empty() {
        return;
    }
    let maxw = (area.width() * 0.86).min(1400.0 * s);
    let mut blk = tx.fit(&toks, Style::new(FontId::SemiBold, 42.0 * s).ls(-0.005).lh(1.22), maxw, 2, area.height() * 0.5, 26.0 * s);
    let bw = blk.w;
    blk.center(bw);
    let (padx, pady) = (24.0 * s, 12.0 * s);
    let (w, h) = (blk.w + 2.0 * padx, blk.h() + 2.0 * pady);
    let x = area.left() + (area.width() - w) * 0.5;
    let y = area.bottom() - 28.0 * s - h;
    let op = prog(t_ms, words[a].start_ms - 80.0, 120.0) * (1.0 - prog(t_ms, words[b - 1].end_ms + 450.0, 150.0));
    if op <= 0.0 {
        return;
    }
    fill_rrect(pm, x, y, w, h, 16.0 * s, [14, 14, 13], 0.88 * op);
    let clip = IRect::round(area).intersect(IRect::of(pm));
    // Toks may have dropped empty words; map block words back by order.
    let cur_in = words[a..=cur].iter().filter(|w| !w.text.trim().is_empty()).count().saturating_sub(1);
    for (k, wd) in blk.words.iter().enumerate() {
        let (col, a2) = if k == cur_in {
            (pal.accent, 1.0)
        } else if k < cur_in {
            (pal.ink, 1.0)
        } else {
            (pal.ink, 0.6)
        };
        tx.draw_word(pm, &blk, wd, x + padx, y + pady, col, op * a2, clip);
    }
}
