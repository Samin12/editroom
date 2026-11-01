//! Tesseract over a pipe: the image goes in on stdin as PGM and word boxes come back as TSV on
//! stdout, so no frame (and no secret) is ever written to disk.

use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

/// One OCR'd word, box in frame pixels `[x0, y0, x1, y1]`.
#[derive(Clone, PartialEq)]
pub(crate) struct Word {
    pub text: String,
    pub conf: f32,
    pub b: [f32; 4],
}

impl Word {
    pub fn yc(&self) -> f32 {
        (self.b[1] + self.b[3]) / 2.0
    }
    pub fn h(&self) -> f32 {
        self.b[3] - self.b[1]
    }
}

pub(crate) struct Tesseract {
    pub bin: String,
    pub lang: String,
    pub psm: u32,
}

/// White margin around a crop: tesseract misses text that touches the image edge.
const MARGIN: usize = 12;

impl Tesseract {
    /// OCR the box `x0..x1` x `y0..y1` of a gray frame `w` wide. Boxes come back in frame pixels.
    pub fn run(&self, px: &[u8], w: usize, x0: usize, x1: usize, y0: usize, y1: usize) -> Result<Vec<Word>> {
        let (bw, bh) = (x1 - x0, y1 - y0);
        let mut crop = vec![0u8; bw * bh];
        for y in y0..y1 {
            crop[(y - y0) * bw..(y - y0 + 1) * bw].copy_from_slice(&px[y * w + x0..y * w + x1]);
        }
        normalize_polarity(&mut crop, bw, bh);
        let (cw, ch) = (bw + 2 * MARGIN, bh + 2 * MARGIN);
        let mut pgm = format!("P5\n{cw} {ch}\n255\n").into_bytes();
        pgm.reserve(cw * ch);
        let white = vec![255u8; cw];
        for _ in 0..MARGIN {
            pgm.extend_from_slice(&white);
        }
        for y in 0..bh {
            pgm.extend_from_slice(&white[..MARGIN]);
            pgm.extend_from_slice(&crop[y * bw..(y + 1) * bw]);
            pgm.extend_from_slice(&white[..MARGIN]);
        }
        for _ in 0..MARGIN {
            pgm.extend_from_slice(&white);
        }
        drop(crop);

        let mut child = Command::new(&self.bin)
            .args(["stdin", "stdout", "-l", &self.lang, "--psm", &self.psm.to_string(), "-c", "tessedit_do_invert=0", "tsv"])
            // one thread per process: we run one process per core instead
            .env("OMP_THREAD_LIMIT", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("running {} (is tesseract installed?)", self.bin))?;
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (tsv, err) = std::thread::scope(|s| {
            let writer = s.spawn(move || {
                let r = stdin.write_all(&pgm);
                drop(stdin);
                r
            });
            let errs = s.spawn(move || {
                let mut e = String::new();
                let _ = stderr.read_to_string(&mut e);
                e
            });
            let mut out = String::new();
            let r = stdout.read_to_string(&mut out);
            let _ = writer.join();
            (r.map(|_| out), errs.join().unwrap_or_default())
        });
        let status = child.wait()?;
        let tsv = tsv?;
        if !status.success() {
            // tesseract's stderr holds diagnostics only, never recognized text
            return Err(anyhow!("tesseract failed ({status}): {}", err.lines().last().unwrap_or("")));
        }
        Ok(parse_tsv(&tsv, x0 as f32 - MARGIN as f32, y0 as f32 - MARGIN as f32))
    }
}

/// Level-5 rows of tesseract TSV: `level page block par line word left top width height conf text`,
/// shifted by (dx, dy).
pub(crate) fn parse_tsv(tsv: &str, dx: f32, dy: f32) -> Vec<Word> {
    let mut out = vec![];
    for line in tsv.lines().skip(1) {
        let f: Vec<&str> = line.splitn(12, '\t').collect();
        if f.len() < 12 || f[0] != "5" {
            continue;
        }
        let text = f[11].trim();
        if text.is_empty() {
            continue;
        }
        let n = |i: usize| f[i].trim().parse::<f32>().unwrap_or(0.0);
        let (l, t, w, h) = (n(6), n(7), n(8), n(9));
        out.push(Word { text: text.to_string(), conf: n(10), b: [l + dx, t + dy, l + w + dx, t + h + dy] });
    }
    out
}

/// Turn light-on-dark areas (terminals, dark mode) into dark-on-light, per 64 px tile, judged by
/// the tile's median: tesseract reads dark text on light far better.
pub(crate) fn normalize_polarity(img: &mut [u8], w: usize, h: usize) {
    const T: usize = 64;
    for ty in (0..h).step_by(T) {
        for tx in (0..w).step_by(T) {
            let (x1, y1) = ((tx + T).min(w), (ty + T).min(h));
            let mut hist = [0u32; 256];
            for y in ty..y1 {
                for &p in &img[y * w + tx..y * w + x1] {
                    hist[p as usize] += 1;
                }
            }
            let half = ((x1 - tx) * (y1 - ty)) as u32 / 2;
            let (mut acc, mut median) = (0, 0);
            for (v, &c) in hist.iter().enumerate() {
                acc += c;
                if acc > half {
                    median = v;
                    break;
                }
            }
            if median < 110 {
                for y in ty..y1 {
                    for p in &mut img[y * w + tx..y * w + x1] {
                        *p = 255 - *p;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_rows() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   1\t1\t0\t0\t0\t0\t0\t0\t100\t50\t-1\t\n\
                   5\t1\t1\t1\t1\t1\t22\t30\t40\t12\t91.5\tHello\n\
                   5\t1\t1\t1\t1\t2\t70\t30\t40\t12\t88\t \n";
        let w = parse_tsv(tsv, -12.0, 88.0);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].text, "Hello");
        assert_eq!(w[0].b, [10.0, 118.0, 50.0, 130.0]);
    }

    #[test]
    fn dark_tiles_flip() {
        let (w, h) = (128, 64);
        let mut img = vec![20u8; w * h];
        for p in &mut img[..64] {
            *p = 240;
        }
        for x in 64..128 {
            for y in 0..64 {
                img[y * w + x] = 230;
            }
        }
        normalize_polarity(&mut img, w, h);
        assert_eq!(img[3 * w + 5], 235, "dark tile inverted");
        assert_eq!(img[100], 230, "light tile kept");
    }
}
