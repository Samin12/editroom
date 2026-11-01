//! The Borumi canvas (dark fill + drifting dot grid), dot stamps for surfaces drawn on top of
//! it, and the raised panel surface stage beats sit on.

use crate::pal::{mix, Pal, Rgb};
use crate::raster::{blit_mask, fill_rrect, stroke_rrect, IRect, Img, Round, Sprite};
use tiny_skia::Pixmap;

/// Subpixel phases of the diagonal drift.
pub const PHASES: usize = 8;

/// One row-strip per drift phase: `pitch` rows of `width + pitch` pixels, so a frame row is a
/// single slice copy at an offset.
pub struct Strips {
    pub w: usize,
    pub p: usize,
    pub phases: Vec<Vec<u8>>,
}

pub fn build_strips(w: usize, p: usize, radius: f32, canvas: Rgb, dot: Rgb) -> Strips {
    let p = p.max(2);
    let sw = w + p;
    let phases = (0..PHASES)
        .map(|k| {
            let f = k as f32 / PHASES as f32;
            // One cell, then tile it across the strip.
            let mut cell = vec![0u8; p * p * 4];
            for r in 0..p {
                for c in 0..p {
                    let d = |v: f32| {
                        let m = (v - f).rem_euclid(p as f32);
                        m.min(p as f32 - m)
                    };
                    let (dx, dy) = (d(c as f32 + 0.5), d(r as f32 + 0.5));
                    let cov = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                    let px = mix(canvas, dot, cov);
                    cell[(r * p + c) * 4..(r * p + c) * 4 + 4].copy_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
            let mut strip = vec![0u8; sw * p * 4];
            for r in 0..p {
                for c in 0..sw {
                    let s = (r * p + c % p) * 4;
                    strip[(r * sw + c) * 4..(r * sw + c) * 4 + 4].copy_from_slice(&cell[s..s + 4]);
                }
            }
            strip
        })
        .collect();
    Strips { w, p, phases }
}

/// Fill the whole pixmap from the strips, the grid shifted by `off` px down-right.
pub fn draw_strips(pm: &mut Pixmap, s: &Strips, off: f32) {
    let w = pm.width() as usize;
    if w != s.w {
        return;
    }
    let (i, k) = split_phase(off);
    let strip = &s.phases[k];
    let sw = s.w + s.p;
    let p = s.p as i64;
    let col0 = (-i).rem_euclid(p) as usize;
    let h = pm.height() as usize;
    let data = pm.data_mut();
    for y in 0..h {
        let r = (y as i64 - i).rem_euclid(p) as usize;
        let src = &strip[(r * sw + col0) * 4..(r * sw + col0 + w) * 4];
        data[y * w * 4..(y + 1) * w * 4].copy_from_slice(src);
    }
}

fn split_phase(off: f32) -> (i64, usize) {
    let mut i = off.floor() as i64;
    let mut k = ((off - i as f32) * PHASES as f32).round() as usize;
    if k >= PHASES {
        i += 1;
        k = 0;
    }
    (i, k)
}

/// A dot stamp with its centre at subpixel offset (fx, fy) quarters, anchored at the centre's
/// integer pixel.
pub fn dot_sprite(radius: f32, fx: u8, fy: u8) -> Sprite {
    let c = (radius + 2.0).ceil() as i32;
    let n = (2 * c + 1) as u32;
    let (cx, cy) = (c as f32 + fx as f32 / 4.0, c as f32 + fy as f32 / 4.0);
    let mut data = vec![0u8; (n * n) as usize];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let cov = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            data[(y * n + x) as usize] = (cov * 255.0).round() as u8;
        }
    }
    Sprite { w: n, h: n, ox: -c, oy: -c, data }
}

/// Stamp a drifting dot grid over `area` (dots at multiples of `pitch` from the frame origin,
/// shifted by `off`), skipping dots outside `round`.
#[allow(clippy::too_many_arguments)]
pub fn stamp_dots(pm: &mut Pixmap, sprites: &[std::sync::Arc<Sprite>], area: IRect, pitch: f32, off: f32, rgb: Rgb, alpha: f32, round: Option<Round>) {
    if pitch < 2.0 || alpha <= 0.0 {
        return;
    }
    let k0 = ((area.x0 as f32 - off) / pitch).floor() as i64 - 1;
    let k1 = ((area.x1 as f32 - off) / pitch).ceil() as i64 + 1;
    let m0 = ((area.y0 as f32 - off) / pitch).floor() as i64 - 1;
    let m1 = ((area.y1 as f32 - off) / pitch).ceil() as i64 + 1;
    for m in m0..=m1 {
        let cy = m as f32 * pitch + off;
        for k in k0..=k1 {
            let cx = k as f32 * pitch + off;
            if let Some(r) = round {
                if r.cov(cx as i32, cy as i32) < 256 || r.cov(cx as i32 - 3, cy as i32 - 3) < 256 || r.cov(cx as i32 + 3, cy as i32 + 3) < 256 {
                    continue;
                }
            }
            let (ix, iy) = (cx.floor(), cy.floor());
            let fx = (((cx - ix) * 4.0).round() as usize).min(3);
            let fy = (((cy - iy) * 4.0).round() as usize).min(3);
            blit_mask(pm, &sprites[fy * 4 + fx], ix as i32, iy as i32, rgb, alpha, area);
        }
    }
}

/// Add a soft elliptical glow of `rgb` (peak strength `a0`, smooth falloff to 0 at the
/// ellipse edge) to the fully opaque pixels of `pm`, dithered so 8-bit steps never band.
pub fn add_glow(pm: &mut Pixmap, cx: f32, cy: f32, rx: f32, ry: f32, rgb: Rgb, a0: f32) {
    let w = pm.width() as usize;
    let h = pm.height() as usize;
    let y0 = ((cy - ry).floor().max(0.0)) as usize;
    let y1 = ((cy + ry).ceil().max(0.0) as usize).min(h);
    let x0 = ((cx - rx).floor().max(0.0)) as usize;
    let x1 = ((cx + rx).ceil().max(0.0) as usize).min(w);
    let data = pm.data_mut();
    for y in y0..y1 {
        let dy = (y as f32 + 0.5 - cy) / ry;
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let d2 = dx * dx + dy * dy;
            if d2 >= 1.0 {
                continue;
            }
            let i = (y * w + x) * 4;
            if data[i + 3] != 255 {
                continue;
            }
            let f = (1.0 - d2) * (1.0 - d2) * a0;
            for c in 0..3 {
                let v = data[i + c] as f32;
                let out = v + (rgb[c] as f32 - v) * f + dither(x as u32, y as u32, c as u32);
                data[i + c] = out.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Deterministic noise in [-0.5, 0.5).
fn dither(x: u32, y: u32, c: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ c.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65536.0 - 0.5
}

/// The raised panel surface: rounded fill, a faint warm glow from the top, a hairline edge.
pub fn surface_layer(w: u32, h: u32, r: f32, pal: &Pal, glow: f32) -> Img {
    let mut pm = match Pixmap::new(w.max(1), h.max(1)) {
        Some(p) => p,
        None => return Img::default(),
    };
    let (wf, hf) = (w as f32, h as f32);
    fill_rrect(&mut pm, 0.0, 0.0, wf, hf, r, pal.surface, 1.0);
    if glow > 0.0 {
        add_glow(&mut pm, wf * 0.5, hf * 0.3, wf * 0.62, hf * 0.75, pal.accent, glow);
    }
    stroke_rrect(&mut pm, 0.5, 0.5, wf - 1.0, hf - 1.0, (r - 0.5).max(0.0), 1.0, pal.white, 0.07);
    Img::from_pixmap(pm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tile_seamlessly_and_drift() {
        let (canvas, dot) = ([18, 18, 18], [200, 200, 200]);
        let s = build_strips(120, 40, 1.6, canvas, dot);
        let mut a = Pixmap::new(120, 80).unwrap();
        draw_strips(&mut a, &s, 0.0);
        // A dot centre sits on the origin grid at offset 0.
        let p = a.pixel(40, 40).unwrap();
        assert!(p.red() > 100, "dot at grid point");
        assert_eq!(a.pixel(20, 20).unwrap().red(), 18);
        // Period: shifting by one pitch gives the same frame.
        let mut b = Pixmap::new(120, 80).unwrap();
        draw_strips(&mut b, &s, 40.0);
        assert_eq!(a.data(), b.data());
        let mut c = Pixmap::new(120, 80).unwrap();
        draw_strips(&mut c, &s, 3.0);
        assert!(c.pixel(43, 43).unwrap().red() > 100);
    }
}
