//! Pixel helpers for opaque RGBA video frames into a premultiplied tiny-skia pixmap.

use tiny_skia::Pixmap;

/// Antialiased rounded-corner coverage for a `w x h` slot with radius `r`: per corner row, the
/// alpha of each pixel within `r` of the edge. Rows outside the corners are fully opaque.
#[derive(Clone, Debug)]
pub struct RoundMask {
    pub w: u32,
    pub h: u32,
    pub r: u32,
    /// alpha[row][col] for rows 0..r and cols 0..r (top-left quadrant; mirrored for others).
    quad: Vec<u8>,
}

impl RoundMask {
    pub fn new(w: u32, h: u32, r: f32) -> RoundMask {
        let r = (r.round() as u32).min(w / 2).min(h / 2);
        let mut quad = vec![255u8; (r * r) as usize];
        let rf = r as f32;
        for y in 0..r {
            for x in 0..r {
                // distance from the corner circle centre, sampled 4x4 for antialiasing
                let mut cov = 0.0f32;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                        let dx = rf - px;
                        let dy = rf - py;
                        if dx * dx + dy * dy <= rf * rf || px >= rf || py >= rf {
                            cov += 1.0;
                        }
                    }
                }
                quad[(y * r + x) as usize] = (cov / 16.0f32 * 255.0).round() as u8;
            }
        }
        RoundMask { w, h, r, quad }
    }

    #[inline]
    fn alpha(&self, x: u32, y: u32) -> u8 {
        let r = self.r;
        let qx = if x < r { Some(x) } else if x >= self.w - r { Some(self.w - 1 - x) } else { None };
        let qy = if y < r { Some(y) } else if y >= self.h - r { Some(self.h - 1 - y) } else { None };
        match (qx, qy) {
            (Some(qx), Some(qy)) => self.quad[(qy * r + qx) as usize],
            _ => 255,
        }
    }
}

/// Copy an opaque `w x h` RGBA frame to (x, y) of `dst`, rounding the corners with `mask`.
pub fn blit_rounded(dst: &mut Pixmap, x: i32, y: i32, src: &[u8], w: u32, h: u32, mask: Option<&RoundMask>) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let data = dst.data_mut();
    for row in 0..h as i32 {
        let ty = y + row;
        if ty < 0 || ty >= dh {
            continue;
        }
        let x0 = x.max(0);
        let x1 = (x + w as i32).min(dw);
        if x1 <= x0 {
            continue;
        }
        let s_off = ((row as u32 * w) as i32 + (x0 - x)) as usize * 4;
        let d_off = (ty * dw + x0) as usize * 4;
        let n = (x1 - x0) as usize * 4;
        let corner_row = mask.map(|m| (row as u32) < m.r || (row as u32) >= m.h.saturating_sub(m.r)).unwrap_or(false);
        if !corner_row {
            data[d_off..d_off + n].copy_from_slice(&src[s_off..s_off + n]);
        } else {
            let m = mask.unwrap();
            for i in 0..(x1 - x0) as usize {
                let a = m.alpha((x0 - x) as u32 + i as u32, row as u32) as u32;
                let s = &src[s_off + i * 4..s_off + i * 4 + 4];
                let d = &mut data[d_off + i * 4..d_off + i * 4 + 4];
                if a == 255 {
                    d.copy_from_slice(s);
                } else if a > 0 {
                    for c in 0..3 {
                        d[c] = ((s[c] as u32 * a + d[c] as u32 * (255 - a) + 127) / 255) as u8;
                    }
                    d[3] = 255;
                }
            }
        }
    }
}

/// Bilinear resample of the source sub-rectangle `sr` (pixels, fractional) of an opaque `sw x sh`
/// frame into a `w x h` frame. Used for zooms.
pub fn resample(src: &[u8], sw: u32, sh: u32, sr: [f32; 4], w: u32, h: u32, out: &mut Vec<u8>) {
    out.resize((w * h * 4) as usize, 0);
    let fx = sr[2] / w as f32;
    let fy = sr[3] / h as f32;
    let maxx = (sw - 1) as f32;
    let maxy = (sh - 1) as f32;
    use rayon::prelude::*;
    out.par_chunks_exact_mut((w * 4) as usize).enumerate().for_each(|(y, row)| {
        let syf = (sr[1] + (y as f32 + 0.5) * fy - 0.5).clamp(0.0, maxy);
        let y0 = syf.floor() as u32;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = syf - y0 as f32;
        for x in 0..w {
            let sxf = (sr[0] + (x as f32 + 0.5) * fx - 0.5).clamp(0.0, maxx);
            let x0 = sxf.floor() as u32;
            let x1 = (x0 + 1).min(sw - 1);
            let tx = sxf - x0 as f32;
            let p = |xx: u32, yy: u32, c: usize| src[((yy * sw + xx) * 4) as usize + c] as f32;
            for c in 0..3 {
                let a = p(x0, y0, c) * (1.0 - tx) + p(x1, y0, c) * tx;
                let b = p(x0, y1, c) * (1.0 - tx) + p(x1, y1, c) * tx;
                row[(x * 4) as usize + c] = (a * (1.0 - ty) + b * ty + 0.5) as u8;
            }
            row[(x * 4 + 3) as usize] = 255;
        }
    });
}

/// Privacy mosaic: replace the pixel rect [x, y, w, h] of `dst` with `block`-sized averages, then
/// darken it a little so it reads as intentionally hidden.
pub fn mosaic(dst: &mut Pixmap, rect: [i32; 4], block: u32) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let x0 = rect[0].max(0);
    let y0 = rect[1].max(0);
    let x1 = (rect[0] + rect[2]).min(dw);
    let y1 = (rect[1] + rect[3]).min(dh);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let b = block.max(2) as i32;
    let data = dst.data_mut();
    let mut by = y0;
    while by < y1 {
        let ey = (by + b).min(y1);
        let mut bx = x0;
        while bx < x1 {
            let ex = (bx + b).min(x1);
            let mut sum = [0u64; 3];
            let mut n = 0u64;
            for yy in by..ey {
                for xx in bx..ex {
                    let o = ((yy * dw + xx) * 4) as usize;
                    for c in 0..3 {
                        sum[c] += data[o + c] as u64;
                    }
                    n += 1;
                }
            }
            let avg = [sum[0] / n, sum[1] / n, sum[2] / n].map(|v| (v * 3 / 4) as u8);
            for yy in by..ey {
                for xx in bx..ex {
                    let o = ((yy * dw + xx) * 4) as usize;
                    data[o..o + 3].copy_from_slice(&avg);
                    data[o + 3] = 255;
                }
            }
            bx = ex;
        }
        by = ey;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_blit_keeps_corners_transparent_to_dst() {
        let mut pm = Pixmap::new(100, 100).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(10, 10, 10, 255));
        let src = vec![200u8; 40 * 40 * 4];
        let m = RoundMask::new(40, 40, 10.0);
        blit_rounded(&mut pm, 30, 30, &src, 40, 40, Some(&m));
        let px = |x: u32, y: u32| pm.pixel(x, y).unwrap().red();
        assert_eq!(px(30, 30), 10); // corner stays background
        assert_eq!(px(50, 50), 200); // inside is copied
        assert_eq!(px(30, 50), 200); // edge middle is copied
    }

    #[test]
    fn resample_identity() {
        let mut src = vec![0u8; 8 * 8 * 4];
        for (i, p) in src.chunks_exact_mut(4).enumerate() {
            p.copy_from_slice(&[(i * 3) as u8, 0, 0, 255]);
        }
        let mut out = vec![];
        resample(&src, 8, 8, [0.0, 0.0, 8.0, 8.0], 8, 8, &mut out);
        assert_eq!(out, src);
    }

    #[test]
    fn mosaic_flattens_blocks() {
        let mut pm = Pixmap::new(32, 32).unwrap();
        for (i, p) in pm.data_mut().chunks_exact_mut(4).enumerate() {
            p.copy_from_slice(&[(i % 256) as u8, 0, 0, 255]);
        }
        mosaic(&mut pm, [0, 0, 16, 16], 8);
        let a = pm.pixel(0, 0).unwrap().red();
        assert_eq!(a, pm.pixel(7, 7).unwrap().red());
    }
}
