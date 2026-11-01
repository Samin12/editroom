//! Pixel plumbing: coverage sprites, premultiplied layers, a fast bilinear scaler with rounded
//! corners, separable blur, and thin wrappers over tiny-skia paths. Every blit takes a clip rect;
//! nothing here ever writes outside it.

use crate::pal::Rgb;
use tiny_skia::{FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

/// Integer pixel rect, half-open: `[x0, x1) x [y0, y1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> IRect {
        IRect { x0, y0, x1, y1 }
    }
    /// Smallest integer rect containing the float rect.
    pub fn around(x: f32, y: f32, w: f32, h: f32) -> IRect {
        IRect::new(x.floor() as i32, y.floor() as i32, (x + w).ceil() as i32, (y + h).ceil() as i32)
    }
    /// Nearest-pixel rect (for clips that are already pixel aligned).
    pub fn round(r: tiny_skia::Rect) -> IRect {
        IRect::new(r.left().round() as i32, r.top().round() as i32, r.right().round() as i32, r.bottom().round() as i32)
    }
    pub fn of(pm: &Pixmap) -> IRect {
        IRect::new(0, 0, pm.width() as i32, pm.height() as i32)
    }
    pub fn w(&self) -> i32 {
        (self.x1 - self.x0).max(0)
    }
    pub fn h(&self) -> i32 {
        (self.y1 - self.y0).max(0)
    }
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
    pub fn intersect(&self, o: IRect) -> IRect {
        IRect::new(self.x0.max(o.x0), self.y0.max(o.y0), self.x1.min(o.x1), self.y1.min(o.y1))
    }
    pub fn inset(&self, d: i32) -> IRect {
        IRect::new(self.x0 + d, self.y0 + d, self.x1 - d, self.y1 - d)
    }
    pub fn outset(&self, d: i32) -> IRect {
        self.inset(-d)
    }
}

/// 8-bit coverage bitmap. `(ox, oy)` is the bitmap's top-left relative to the sprite's anchor
/// (for text: the pen origin on the baseline).
#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub w: u32,
    pub h: u32,
    pub ox: i32,
    pub oy: i32,
    pub data: Vec<u8>,
}

/// Premultiplied RGBA image with, per row, the span that is fully opaque (blits copy it).
#[derive(Clone, Debug, Default)]
pub struct Img {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
    pub spans: Vec<(u32, u32)>,
}

impl Img {
    pub fn from_pixmap(pm: Pixmap) -> Img {
        let (w, h) = (pm.width(), pm.height());
        let mut img = Img { w, h, data: pm.take(), spans: vec![] };
        img.compute_spans();
        img
    }

    pub fn view(&self) -> ImgRef<'_> {
        ImgRef { w: self.w, h: self.h, data: &self.data }
    }

    /// Every row fully opaque.
    pub fn is_opaque(&self) -> bool {
        self.spans.len() == self.h as usize && self.spans.iter().all(|&(a, b)| a == 0 && b == self.w)
    }

    fn compute_spans(&mut self) {
        let w = self.w as usize;
        self.spans = (0..self.h as usize)
            .map(|y| {
                let row = &self.data[y * w * 4..(y + 1) * w * 4];
                let a = |x: usize| row[x * 4 + 3];
                let first = (0..w).find(|&x| a(x) == 255);
                match first {
                    None => (0, 0),
                    Some(f) => {
                        let mut e = f;
                        while e < w && a(e) == 255 {
                            e += 1;
                        }
                        (f as u32, e as u32)
                    }
                }
            })
            .collect();
    }
}

/// A borrowed premultiplied RGBA image (an [`Img`] or a decoded video frame).
#[derive(Clone, Copy)]
pub struct ImgRef<'a> {
    pub w: u32,
    pub h: u32,
    pub data: &'a [u8],
}

#[inline(always)]
pub fn div255(x: u32) -> u32 {
    (x + 128 + ((x + 128) >> 8)) >> 8
}

#[inline(always)]
fn ld(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

#[inline(always)]
fn st(b: &mut [u8], v: u32) {
    b.copy_from_slice(&v.to_le_bytes());
}

/// Lerp two RGBA pixels, `w` in 0..=256 (weight of `b`).
#[inline(always)]
fn lerp_px(a: u32, b: u32, w: u32) -> u32 {
    let iw = 256 - w;
    let rb = (((a & 0x00FF_00FF) * iw + (b & 0x00FF_00FF) * w) >> 8) & 0x00FF_00FF;
    let ag = (((a >> 8) & 0x00FF_00FF) * iw + ((b >> 8) & 0x00FF_00FF) * w) & 0xFF00_FF00;
    rb | ag
}

/// Scale all four channels by `m` in 0..=256.
#[inline(always)]
fn scale_px(p: u32, m: u32) -> u32 {
    let rb = (((p & 0x00FF_00FF) * m) >> 8) & 0x00FF_00FF;
    let ag = (((p >> 8) & 0x00FF_00FF) * m) & 0xFF00_FF00;
    rb | ag
}

/// Scale RGB by `m` in 0..=256, keep alpha.
#[inline(always)]
fn scale_rgb(p: u32, m: u32) -> u32 {
    let a = p & 0xFF00_0000;
    let s = scale_px(p, m);
    (s & 0x00FF_FFFF) | a
}

/// Premultiplied source-over with exact /255 rounding.
#[inline(always)]
fn over(src: u32, dst: u32) -> u32 {
    let inv = 255 - (src >> 24);
    if inv == 0 {
        return src;
    }
    let t = (dst & 0x00FF_00FF) * inv + 0x0080_0080;
    let rb = ((t + ((t >> 8) & 0x00FF_00FF)) >> 8) & 0x00FF_00FF;
    let t2 = ((dst >> 8) & 0x00FF_00FF) * inv + 0x0080_0080;
    let ag = (t2 + ((t2 >> 8) & 0x00FF_00FF)) & 0xFF00_FF00;
    src + (rb | ag)
}

/// Opaque solid fill of an integer rect.
pub fn fill_rect(pm: &mut Pixmap, r: IRect, rgb: Rgb) {
    let r = r.intersect(IRect::of(pm));
    if r.is_empty() {
        return;
    }
    let pw = pm.width() as usize;
    let px = [rgb[0], rgb[1], rgb[2], 255];
    let data = pm.data_mut();
    for y in r.y0..r.y1 {
        let row = &mut data[(y as usize * pw + r.x0 as usize) * 4..(y as usize * pw + r.x1 as usize) * 4];
        for d in row.as_chunks_mut::<4>().0 {
            *d = px;
        }
    }
}

/// Paint `rgb` through a coverage sprite anchored at (x, y).
pub fn blit_mask(pm: &mut Pixmap, s: &Sprite, x: i32, y: i32, rgb: Rgb, opacity: f32, clip: IRect) {
    blit_mask_skip(pm, s, x, y, rgb, opacity, clip, None)
}

/// `blit_mask` that leaves `skip` (frame px) untouched: used for shadows whose interior the
/// slot covers anyway.
#[allow(clippy::too_many_arguments)]
pub fn blit_mask_skip(pm: &mut Pixmap, s: &Sprite, x: i32, y: i32, rgb: Rgb, opacity: f32, clip: IRect, skip: Option<IRect>) {
    let op = (opacity.clamp(0.0, 1.0) * 256.0).round() as u32;
    if op == 0 || s.w == 0 || s.h == 0 {
        return;
    }
    let bx = x + s.ox;
    let by = y + s.oy;
    let r = IRect::new(bx, by, bx + s.w as i32, by + s.h as i32).intersect(clip).intersect(IRect::of(pm));
    if r.is_empty() {
        return;
    }
    let pw = pm.width() as usize;
    let sw = s.w as usize;
    let (cr, cg, cb) = (rgb[0] as u32, rgb[1] as u32, rgb[2] as u32);
    let data = pm.data_mut();
    for yy in r.y0..r.y1 {
        // Split the row around the skipped interior.
        let mut parts = [(r.x0, r.x1), (0, 0)];
        if let Some(k) = skip {
            if yy >= k.y0 && yy < k.y1 && k.x0 < r.x1 && k.x1 > r.x0 {
                parts = [(r.x0, k.x0.max(r.x0)), (k.x1.min(r.x1), r.x1)];
            }
        }
        for (xa, xb) in parts {
            if xb <= xa {
                continue;
            }
            let so = (yy - by) as usize * sw + (xa - bx) as usize;
            let srow = &s.data[so..so + (xb - xa) as usize];
            let doff = (yy as usize * pw + xa as usize) * 4;
            let drow = &mut data[doff..doff + (xb - xa) as usize * 4];
            for (c, d) in srow.iter().zip(drow.as_chunks_mut::<4>().0) {
                if *c == 0 {
                    continue;
                }
                let a = (*c as u32 * op) >> 8;
                if a >= 255 {
                    d[0] = cr as u8;
                    d[1] = cg as u8;
                    d[2] = cb as u8;
                    d[3] = 255;
                    continue;
                }
                let ia = 255 - a;
                d[0] = div255(cr * a + d[0] as u32 * ia) as u8;
                d[1] = div255(cg * a + d[1] as u32 * ia) as u8;
                d[2] = div255(cb * a + d[2] as u32 * ia) as u8;
                d[3] = div255(255 * a + d[3] as u32 * ia) as u8;
            }
        }
    }
}

/// Composite a premultiplied image at an integer offset.
pub fn blit_img(pm: &mut Pixmap, img: &Img, x: i32, y: i32, opacity: f32, clip: IRect) {
    let op = (opacity.clamp(0.0, 1.0) * 256.0).round() as u32;
    if op == 0 || img.w == 0 {
        return;
    }
    let r = IRect::new(x, y, x + img.w as i32, y + img.h as i32).intersect(clip).intersect(IRect::of(pm));
    if r.is_empty() {
        return;
    }
    let pw = pm.width() as usize;
    let iw = img.w as usize;
    let data = pm.data_mut();
    for yy in r.y0..r.y1 {
        let sy = (yy - y) as usize;
        let srow = &img.data[sy * iw * 4..(sy + 1) * iw * 4];
        let drow = &mut data[yy as usize * pw * 4..(yy as usize + 1) * pw * 4];
        let (mut ca, mut cb) = (0i32, 0i32);
        if op == 256 {
            if let Some(&(a, b)) = img.spans.get(sy) {
                ca = (a as i32 + x).max(r.x0);
                cb = (b as i32 + x).min(r.x1);
                if cb > ca {
                    let s0 = (ca - x) as usize * 4;
                    drow[ca as usize * 4..cb as usize * 4].copy_from_slice(&srow[s0..s0 + (cb - ca) as usize * 4]);
                } else {
                    cb = ca;
                }
            }
        }
        let segs = if cb > ca { [(r.x0, ca), (cb, r.x1)] } else { [(r.x0, r.x1), (0, 0)] };
        for (xa, xb) in segs {
            for xx in xa..xb {
                let si = (xx - x) as usize * 4;
                let s = ld(&srow[si..si + 4]);
                if s == 0 {
                    continue;
                }
                let s = if op == 256 { s } else { scale_px(s, op) };
                let di = xx as usize * 4;
                let d = ld(&drow[di..di + 4]);
                st(&mut drow[di..di + 4], over(s, d));
            }
        }
    }
}

/// Rounded-rect coverage (pixel-aligned bounds, radius in px).
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub r: f32,
}

impl Round {
    pub fn new(x: f32, y: f32, w: f32, h: f32, r: f32) -> Round {
        Round { x0: x, y0: y, x1: x + w, y1: y + h, r: r.min(w * 0.5).min(h * 0.5).max(0.0) }
    }
    pub fn irect(&self) -> IRect {
        IRect::new(self.x0.floor() as i32, self.y0.floor() as i32, self.x1.ceil() as i32, self.y1.ceil() as i32)
    }
    /// Coverage 0..=256 of pixel (x, y).
    #[inline]
    pub fn cov(&self, x: i32, y: i32) -> u32 {
        let cx = x as f32 + 0.5;
        let cy = y as f32 + 0.5;
        let ex = ((self.x0 - cx + 0.5).max(cx - self.x1 + 0.5)).clamp(0.0, 1.0);
        let ey = ((self.y0 - cy + 0.5).max(cy - self.y1 + 0.5)).clamp(0.0, 1.0);
        let mut c = (1.0 - ex) * (1.0 - ey);
        let dx = (self.x0 + self.r - cx).max(cx - (self.x1 - self.r));
        let dy = (self.y0 + self.r - cy).max(cy - (self.y1 - self.r));
        if dx > 0.0 && dy > 0.0 {
            let d = (dx * dx + dy * dy).sqrt();
            c = c.min((self.r - d + 0.5).clamp(0.0, 1.0));
        }
        (c * 256.0).round() as u32
    }
    /// Columns of row `y` that are certainly fully covered.
    #[inline]
    fn solid_span(&self, y: i32) -> (i32, i32) {
        let cy = y as f32 + 0.5;
        if cy - 0.5 < self.y0 || cy + 0.5 > self.y1 {
            return (0, 0);
        }
        let dy = (self.y0 + self.r - cy).max(cy - (self.y1 - self.r));
        let inset = if dy > 0.0 { self.r - (self.r * self.r - dy * dy).max(0.0).sqrt() + 1.0 } else { 0.0 };
        ((self.x0 + inset).ceil() as i32, (self.x1 - inset).floor() as i32)
    }
}

/// Darken everything outside a rounded rect (a spotlight on an image highlight).
#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub area: Round,
    /// Brightness kept outside the spot, 0..=256.
    pub keep: u32,
}

/// Draw `img` stretched to `dst` = [x, y, w, h] (frame px, fractional), bilinear, clipped to
/// `clip` and optionally to a rounded rect, with an optional spotlight.
pub fn draw_img(pm: &mut Pixmap, img: &Img, dst: [f32; 4], clip: IRect, opacity: f32, round: Option<Round>, spot: Option<Spot>) {
    draw_img_ref(pm, img.view(), dst, clip, opacity, round, spot)
}

/// [`draw_img`] for a borrowed image.
pub fn draw_img_ref(pm: &mut Pixmap, img: ImgRef<'_>, dst: [f32; 4], clip: IRect, opacity: f32, round: Option<Round>, spot: Option<Spot>) {
    let [dx, dy, dw, dh] = dst;
    let op = (opacity.clamp(0.0, 1.0) * 256.0).round() as u32;
    if dw <= 0.0 || dh <= 0.0 || img.w == 0 || img.h == 0 || op == 0 {
        return;
    }
    let mut r = IRect::around(dx, dy, dw, dh).intersect(clip).intersect(IRect::of(pm));
    if let Some(ro) = round {
        r = r.intersect(ro.irect());
    }
    if r.is_empty() {
        return;
    }
    let (iw, ih) = (img.w as usize, img.h as usize);
    let sx = img.w as f32 / dw;
    let sy = img.h as f32 / dh;
    let exact = (sx - 1.0).abs() < 1e-4 && (sy - 1.0).abs() < 1e-4 && dx.fract() == 0.0 && dy.fract() == 0.0;
    let n = r.w() as usize;
    let pw = pm.width() as usize;
    let data = pm.data_mut();
    let mut line = vec![0u32; n];
    if exact {
        let (ox, oy) = (dx as i32, dy as i32);
        for y in r.y0..r.y1 {
            let srow = (y - oy) as usize;
            let s0 = (srow * iw + (r.x0 - ox) as usize) * 4;
            let src = &img.data[s0..s0 + n * 4];
            let drow = &mut data[(y as usize * pw + r.x0 as usize) * 4..(y as usize * pw + r.x1 as usize) * 4];
            let (sa, sb) = solid_cols(round, y, r);
            let opaque = op >= 256 && spot.is_none() && src[(sa - r.x0) as usize * 4..(sb - r.x0) as usize * 4].as_chunks::<4>().0.iter().all(|p| p[3] == 255);
            if opaque {
                let (a, b) = ((sa - r.x0) as usize * 4, (sb - r.x0) as usize * 4);
                drow[a..b].copy_from_slice(&src[a..b]);
                for (l, p) in line.iter_mut().zip(src.as_chunks::<4>().0) {
                    *l = u32::from_le_bytes(*p);
                }
                blend_line(drow, &line, r.x0, y, op, round, Some((sa, sb)));
            } else {
                for (l, p) in line.iter_mut().zip(src.as_chunks::<4>().0) {
                    *l = u32::from_le_bytes(*p);
                }
                if let Some(sp) = spot {
                    apply_spot(&mut line, r.x0, y, sp);
                }
                blend_line(drow, &line, r.x0, y, op, round, None);
            }
        }
        return;
    }
    // Column table: left source column and weight of the right one (0..=256).
    let mut cols = Vec::with_capacity(n);
    for x in r.x0..r.x1 {
        let u = ((x as f32 + 0.5 - dx) * sx - 0.5).clamp(0.0, (iw - 1) as f32);
        let x0 = (u.floor() as usize).min(iw.saturating_sub(2));
        let w = (((u - x0 as f32) * 256.0) as u32).min(256);
        cols.push((x0 as u32, w));
    }
    let sc0 = cols[0].0 as usize;
    let sc1 = (cols[n - 1].0 as usize + 1).min(iw - 1);
    // Store byte offsets into the per-row temp line.
    for c in &mut cols {
        c.0 = (c.0 - sc0 as u32) * 4;
    }
    let span = sc1 - sc0 + 1;
    let mut tmp = vec![0u8; (span + 1) * 4];
    for y in r.y0..r.y1 {
        let v = ((y as f32 + 0.5 - dy) * sy - 0.5).clamp(0.0, (ih - 1) as f32);
        let y0 = (v.floor() as usize).min(ih - 1);
        let y1 = (y0 + 1).min(ih - 1);
        let fy = ((v - y0 as f32) * 256.0) as u32;
        let r0 = &img.data[(y0 * iw + sc0) * 4..(y0 * iw + sc1 + 1) * 4];
        let t = &mut tmp[..span * 4];
        if fy == 0 || y1 == y0 {
            t.copy_from_slice(r0);
        } else {
            // Byte-wise so it vectorizes: a * (256 - w) + b * w fits in u16.
            let r1 = &img.data[(y1 * iw + sc0) * 4..(y1 * iw + sc1 + 1) * 4];
            let (wa, wb) = ((256 - fy) as u16, fy as u16);
            for ((t, &a), &b) in t.iter_mut().zip(r0).zip(r1) {
                *t = ((a as u16 * wa + b as u16 * wb) >> 8) as u8;
            }
        }
        let n4 = tmp.len();
        tmp.copy_within(n4 - 8..n4 - 4, n4 - 4);
        for (l, &(c, w)) in line.iter_mut().zip(&cols) {
            let c = c as usize;
            let pair: [u8; 8] = tmp[c..c + 8].try_into().unwrap_or([0; 8]);
            let a = u32::from_le_bytes([pair[0], pair[1], pair[2], pair[3]]);
            let b = u32::from_le_bytes([pair[4], pair[5], pair[6], pair[7]]);
            *l = lerp_px(a, b, w);
        }
        if let Some(sp) = spot {
            apply_spot(&mut line, r.x0, y, sp);
        }
        let drow = &mut data[(y as usize * pw + r.x0 as usize) * 4..(y as usize * pw + r.x1 as usize) * 4];
        blend_line(drow, &line, r.x0, y, op, round, None);
    }
}

/// Columns of row `y` inside `r` that the rounded mask covers fully.
fn solid_cols(round: Option<Round>, y: i32, r: IRect) -> (i32, i32) {
    match round {
        Some(ro) => {
            let (a, b) = ro.solid_span(y);
            let (a, b) = (a.max(r.x0), b.min(r.x1));
            if b > a {
                (a, b)
            } else {
                (r.x0, r.x0)
            }
        }
        None => (r.x0, r.x1),
    }
}

/// Darken a line of pixels outside the spotlight.
fn apply_spot(line: &mut [u32], x0: i32, y: i32, sp: Spot) {
    let a = sp.area;
    let cy = y as f32 + 0.5;
    if sp.keep >= 256 {
        return;
    }
    if cy + 0.5 <= a.y0 || cy - 0.5 >= a.y1 {
        for p in line.iter_mut() {
            *p = scale_rgb(*p, sp.keep);
        }
        return;
    }
    let (pa, pb) = a.solid_span(y);
    let (ea, eb) = (a.x0.floor() as i32 - 1, a.x1.ceil() as i32 + 1);
    for (i, p) in line.iter_mut().enumerate() {
        let x = x0 + i as i32;
        if x >= pa && x < pb {
            continue;
        }
        let m = if x < ea || x >= eb {
            sp.keep
        } else {
            let c = a.cov(x, y);
            (sp.keep * (256 - c) + 256 * c) >> 8
        };
        *p = scale_rgb(*p, m);
    }
}

/// Composite a line of premultiplied pixels into `drow` (which starts at column `x0`) with
/// opacity `op` (0..=256) through the optional rounded mask. `done`: a column range already
/// written by the caller.
fn blend_line(drow: &mut [u8], line: &[u32], x0: i32, y: i32, op: u32, round: Option<Round>, done: Option<(i32, i32)>) {
    let r = IRect::new(x0, y, x0 + line.len() as i32, y + 1);
    let (sa, sb) = solid_cols(round, y, r);
    let (ia, ib) = ((sa - x0) as usize, (sb - x0) as usize);
    // Edges through the rounded mask.
    for i in (0..ia).chain(ib..line.len()) {
        let x = x0 + i as i32;
        let m = match round {
            Some(ro) => (op * ro.cov(x, y)) >> 8,
            None => op,
        };
        let s = line[i];
        if m == 0 || s == 0 {
            continue;
        }
        let d = &mut drow[i * 4..i * 4 + 4];
        let s = if m >= 256 { s } else { scale_px(s, m) };
        st(d, over(s, ld(d)));
    }
    if done.is_some() || ib <= ia {
        return;
    }
    let mid = &mut drow[ia * 4..ib * 4];
    if op >= 256 {
        for (d, &s) in mid.as_chunks_mut::<4>().0.iter_mut().zip(&line[ia..ib]) {
            if (s >> 24) == 255 {
                *d = s.to_le_bytes();
            } else if s != 0 {
                *d = over(s, u32::from_le_bytes(*d)).to_le_bytes();
            }
        }
    } else {
        for (d, &s) in mid.as_chunks_mut::<4>().0.iter_mut().zip(&line[ia..ib]) {
            if s != 0 {
                *d = over(scale_px(s, op), u32::from_le_bytes(*d)).to_le_bytes();
            }
        }
    }
}

/// A tiny-skia paint of a solid colour with alpha 0..1.
pub fn paint(rgb: Rgb, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(rgb[0], rgb[1], rgb[2], (alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

/// Rounded-rect path (cubic corners).
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let k = 0.552_284_8 * r;
    let mut pb = PathBuilder::new();
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
    pb.finish()
}

#[allow(clippy::too_many_arguments)]
pub fn fill_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: Rgb, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    if let Some(p) = rrect(x, y, w, h, r) {
        pm.fill_path(&p, &paint(rgb, alpha), FillRule::Winding, Transform::identity(), None);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn stroke_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, rgb: Rgb, alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    if let Some(p) = rrect(x, y, w, h, r) {
        let st = Stroke { width, ..Stroke::default() };
        pm.stroke_path(&p, &paint(rgb, alpha), &st, Transform::identity(), None);
    }
}

/// Perimeter of a rounded rect.
pub fn rrect_len(w: f32, h: f32, r: f32) -> f32 {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    2.0 * (w + h) - 8.0 * r + std::f32::consts::TAU * r
}

/// Stroke the first `p` (0..1) of a path whose length is `len` (draw-on), round caps.
#[allow(clippy::too_many_arguments)]
pub fn stroke_partial(pm: &mut Pixmap, path: &Path, len: f32, p: f32, width: f32, rgb: Rgb, alpha: f32, tr: Transform) {
    if p <= 0.001 || alpha <= 0.0 || width <= 0.0 {
        return;
    }
    let mut st = Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    if p < 0.999 {
        st.dash = StrokeDash::new(vec![(len * p).max(0.01), len + width * 4.0], 0.0);
    }
    pm.stroke_path(path, &paint(rgb, alpha), &st, tr, None);
}

pub fn fill_circle(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, rgb: Rgb, alpha: f32) {
    if alpha <= 0.0 || r <= 0.0 {
        return;
    }
    if let Some(p) = PathBuilder::from_circle(cx, cy, r) {
        pm.fill_path(&p, &paint(rgb, alpha), FillRule::Winding, Transform::identity(), None);
    }
}

/// Gaussian-ish blur of an 8-bit buffer: three box passes each way.
pub fn blur_a8(buf: &mut [u8], w: usize, h: usize, sigma: f32) {
    if sigma < 0.3 || w == 0 || h == 0 {
        return;
    }
    let n = 3.0f32;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let mi = (12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n) / (-4.0 * wl as f32 - 4.0);
    let m = mi.round() as i32;
    let mut tmp = vec![0u8; buf.len()];
    for i in 0..3 {
        let size = if i < m { wl } else { wu };
        let r = ((size - 1) / 2).max(0) as usize;
        if r == 0 {
            continue;
        }
        box_h(buf, &mut tmp, w, h, r);
        box_v(&tmp, buf, w, h, r);
    }
}

fn box_h(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let size = (2 * r + 1) as u32;
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut dst[y * w..(y + 1) * w];
        let mut sum: u32 = row.iter().take(r + 1).map(|&v| v as u32).sum();
        for x in 0..w {
            out[x] = ((sum + size / 2) / size) as u8;
            let add = x + r + 1;
            if add < w {
                sum += row[add] as u32;
            }
            if x >= r {
                sum -= row[x - r] as u32;
            }
        }
    }
}

fn box_v(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let size = (2 * r + 1) as u32;
    let mut sums = vec![0u32; w];
    for y in 0..=r.min(h - 1) {
        for x in 0..w {
            sums[x] += src[y * w + x] as u32;
        }
    }
    for y in 0..h {
        let out = &mut dst[y * w..(y + 1) * w];
        for x in 0..w {
            out[x] = ((sums[x] + size / 2) / size) as u8;
        }
        let add = y + r + 1;
        if add < h {
            let row = &src[add * w..(add + 1) * w];
            for x in 0..w {
                sums[x] += row[x] as u32;
            }
        }
        if y >= r {
            let row = &src[(y - r) * w..(y - r + 1) * w];
            for x in 0..w {
                sums[x] -= row[x] as u32;
            }
        }
    }
}

/// A soft drop shadow: blurred rounded-rect coverage plus the interior (relative to the slot's
/// top-left) that the slot itself covers.
#[derive(Clone, Debug)]
pub struct Shadow {
    pub sprite: Sprite,
    pub inner: IRect,
}

pub fn make_shadow(w: u32, h: u32, r: f32, blur: f32, spread: f32) -> Shadow {
    let pad = (blur * 3.0).ceil().max(0.0) as u32 + spread.max(0.0).ceil() as u32 + 2;
    let sw = (w as f32 + 2.0 * spread).max(1.0);
    let sh = (h as f32 + 2.0 * spread).max(1.0);
    let (mw, mh) = (w + 2 * pad, h + 2 * pad);
    let mut s = Sprite { w: mw, h: mh, ox: -(pad as i32), oy: -(pad as i32), data: vec![0; (mw * mh) as usize] };
    if let (Some(mut m), Some(p)) = (Mask::new(mw, mh), rrect(pad as f32 - spread, pad as f32 - spread, sw, sh, (r + spread).max(0.0))) {
        m.fill_path(&p, FillRule::Winding, true, Transform::identity());
        s.data.copy_from_slice(m.data());
    }
    blur_a8(&mut s.data, mw as usize, mh as usize, blur / 2.0);
    let ri = r.ceil() as i32 + 1;
    Shadow { sprite: s, inner: IRect::new(ri, ri, w as i32 - ri, h as i32 - ri) }
}

/// Draw with tiny-skia into a transparent scratch covering `region` (frame px); the closure
/// gets the translation to use. Returns the layer to blit at `(region.x0, region.y0)`.
pub fn scratch(region: IRect, f: impl FnOnce(&mut Pixmap, Transform)) -> Option<Img> {
    if region.is_empty() {
        return None;
    }
    let mut s = Pixmap::new(region.w() as u32, region.h() as u32)?;
    f(&mut s, Transform::from_translate(-region.x0 as f32, -region.y0 as f32));
    Some(Img::from_pixmap(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_is_exact_for_opaque_and_clear() {
        assert_eq!(over(0xFF11_2233, 0xFF44_5566), 0xFF11_2233);
        assert_eq!(over(0, 0xFF44_5566), 0xFF44_5566);
        // 50% white over black.
        let half = 0x8080_8080;
        let r = over(half, 0xFF00_0000);
        assert_eq!(r >> 24, 255);
        assert!(((r & 0xFF) as i32 - 128).abs() <= 1);
    }

    #[test]
    fn draw_img_identity_copies() {
        let mut src = Pixmap::new(8, 6).unwrap();
        src.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 255));
        let img = Img::from_pixmap(src);
        let mut pm = Pixmap::new(20, 20).unwrap();
        let full = IRect::of(&pm);
        draw_img(&mut pm, &img, [3.0, 4.0, 8.0, 6.0], full, 1.0, None, None);
        let px = pm.pixel(5, 6).unwrap();
        assert_eq!((px.red(), px.green(), px.blue(), px.alpha()), (200, 100, 50, 255));
        assert_eq!(pm.pixel(2, 4).unwrap().alpha(), 0);
        assert_eq!(pm.pixel(11, 4).unwrap().alpha(), 0);
    }

    #[test]
    fn blur_spreads_and_keeps_mass() {
        let (w, h) = (40usize, 40usize);
        let mut b = vec![0u8; w * h];
        for y in 15..25 {
            for x in 15..25 {
                b[y * w + x] = 255;
            }
        }
        let before: u32 = b.iter().map(|&v| v as u32).sum();
        blur_a8(&mut b, w, h, 3.0);
        let after: u32 = b.iter().map(|&v| v as u32).sum();
        assert!(b[10 * w + 20] > 0);
        assert!((before as i64 - after as i64).abs() < before as i64 / 20);
    }
}

#[cfg(test)]
mod perf {
    use super::*;

    /// `cargo test -p cutroom-gfx --release -- --ignored --nocapture perf`
    #[test]
    #[ignore]
    fn perf_draw_img() {
        let (w, h) = (1346u32, 862u32);
        let mut src = Pixmap::new(w, h).unwrap();
        src.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 255));
        let img = Img::from_pixmap(src);
        let mut pm = Pixmap::new(1920, 1080).unwrap();
        let clip = IRect::of(&pm);
        let round = Round::new(100.0, 100.0, 1270.0, 812.0, 21.0);
        let spot = Spot { area: Round::new(300.0, 300.0, 300.0, 100.0, 10.0), keep: 160 };
        for (name, sp, rd) in [("plain", None, None), ("round", None, Some(round)), ("round+spot", Some(spot), Some(round))] {
            let t = std::time::Instant::now();
            for i in 0..50 {
                let z = 1.0 + i as f32 * 0.001;
                draw_img(&mut pm, &img, [100.0 - 10.0 * z, 100.0 - 5.0 * z, 1270.0 * z, 812.0 * z], clip, 1.0, rd, sp);
            }
            println!("{name}: {:.3} ms", t.elapsed().as_secs_f64() * 1000.0 / 50.0);
        }
        let t = std::time::Instant::now();
        for _ in 0..50 {
            stroke_rrect(&mut pm, 100.5, 100.5, 1269.0, 811.0, 21.0, 1.2, [255, 255, 255], 0.1);
        }
        println!("edge stroke: {:.3} ms", t.elapsed().as_secs_f64() * 1000.0 / 50.0);
        let surf = {
            let mut s = Pixmap::new(1474, 1042).unwrap();
            fill_rrect(&mut s, 0.0, 0.0, 1474.0, 1042.0, 22.0, [24, 23, 23], 1.0);
            Img::from_pixmap(s)
        };
        let t = std::time::Instant::now();
        for _ in 0..50 {
            blit_img(&mut pm, &surf, 19, 19, 1.0, clip);
        }
        println!("surface blit: {:.3} ms", t.elapsed().as_secs_f64() * 1000.0 / 50.0);
        let sh = make_shadow(1270, 812, 21.0, 43.0, -10.0);
        let t = std::time::Instant::now();
        for _ in 0..50 {
            blit_mask_skip(&mut pm, &sh.sprite, 100, 123, [0, 0, 0], 0.6, clip, Some(IRect::new(100 + sh.inner.x0, 100 + sh.inner.y0, 100 + sh.inner.x1, 100 + sh.inner.y1)));
        }
        println!("card shadow: {:.3} ms", t.elapsed().as_secs_f64() * 1000.0 / 50.0);
    }
}
