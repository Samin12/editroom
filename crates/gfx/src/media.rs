//! Still images for `image` beats: decoded once, pre-scaled once to the size the card needs
//! (including the push-in headroom), premultiplied, cached.

use crate::raster::Img;
use image::imageops::FilterType;
use std::path::Path;

/// Pixel size of an image file without decoding it.
pub fn dims(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// Decode `path` and resample it to exactly `w` x `h` (premultiplied RGBA).
pub fn load_scaled(path: &Path, w: u32, h: u32) -> Option<Img> {
    let img = image::open(path).ok()?.to_rgba8();
    let img = if img.width() == w && img.height() == h {
        img
    } else {
        // Lanczos for strong reductions (sharp screenshots), Catmull-Rom otherwise.
        let filter = if img.width() > w * 2 { FilterType::Lanczos3 } else { FilterType::CatmullRom };
        image::imageops::resize(&img, w.max(1), h.max(1), filter)
    };
    let (w, h) = img.dimensions();
    let mut data = img.into_raw();
    for px in data.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        if a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }
    }
    let pm = tiny_skia::Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h)?)?;
    Some(Img::from_pixmap(pm))
}
