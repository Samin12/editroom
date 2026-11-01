//! `cutroom boxes`: where a phrase sits in a proof screenshot, measured on the glyphs (tesseract
//! word boxes), so a highlight lands on the text instead of being placed by eye.
//!
//! The phrase is matched against the OCR'd words in reading order with spaces and punctuation
//! ignored, so OCR splitting or joining words ("kicking-off" / "kicking off") does not matter. A
//! match that starts or ends inside a word is trimmed in proportion to the characters covered.
//! When nothing matches exactly, the closest stretch within a small edit distance is returned.

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// One phrase found in the image.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BoxMatch {
    /// The OCR'd words the match covers.
    pub text: String,
    /// Exact (punctuation and case aside) or fuzzy.
    pub exact: bool,
    /// Edit distance of the match (characters, 0 when exact).
    pub distance: usize,
    /// Mean tesseract confidence of the covered words (0..100).
    pub conf: f32,
    /// Union box, image pixels `[x, y, w, h]`.
    pub px: [f32; 4],
    /// Union box, image-normalized `[x, y, w, h]` (what `image` beats' `highlight` takes).
    pub norm: [f64; 4],
    /// One box per text line the match spans.
    pub lines: Vec<LineBox>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct LineBox {
    pub px: [f32; 4],
    pub norm: [f64; 4],
}

#[derive(Serialize, Clone, Debug)]
pub struct Boxes {
    pub image: String,
    pub width: u32,
    pub height: u32,
    pub phrase: String,
    pub matches: Vec<BoxMatch>,
}

/// A word from tesseract TSV with its line key.
#[derive(Clone, Debug)]
struct OWord {
    text: String,
    conf: f32,
    b: [f32; 4],
    line: (u32, u32, u32),
}

/// Lowercase letters, digits and the symbols that carry meaning in numbers and handles.
fn norm(s: &str) -> String {
    s.chars().flat_map(|c| c.to_lowercase()).filter(|c| c.is_alphanumeric() || "$%+#@&".contains(*c)).collect()
}

/// Find `phrase` in the PNG/JPEG at `path`. `all`: every match, else the best one. `pad`: grow the
/// boxes by this many image pixels on each side.
pub fn find(path: &Path, phrase: &str, all: bool, pad: f32) -> Result<Boxes> {
    let target = norm(phrase);
    if target.is_empty() {
        return Err(anyhow!("the phrase has no letters or digits"));
    }
    let (w, h, gray) = decode_gray(path)?;
    let mut best: Vec<BoxMatch> = vec![];
    // cheapest first: most proof captures are dark text on light, or dark mode
    let passes: [(u32, Pre); 4] = [(3, Pre::Polarity), (11, Pre::Polarity), (3, Pre::None), (3, Pre::Invert)];
    for (psm, pre) in passes {
        let mut img = gray.clone();
        match pre {
            Pre::Polarity => crate::ocr::normalize_polarity(&mut img, w, h),
            Pre::Invert => img.iter_mut().for_each(|p| *p = 255 - *p),
            Pre::None => {}
        }
        let words = ocr_words(&img, w, h, psm)?;
        let found = matches(&words, &target, w, h, pad);
        let exact = found.iter().any(|m| m.exact);
        if exact {
            best = found;
            break;
        }
        // keep the closest fuzzy result over all passes
        if let Some(m) = found.into_iter().min_by_key(|m| m.distance) {
            if best.first().is_none_or(|b| m.distance < b.distance) {
                best = vec![m];
            }
        }
    }
    best.sort_by(|a, b| b.exact.cmp(&a.exact).then(a.distance.cmp(&b.distance)).then(a.px[1].total_cmp(&b.px[1])).then(a.px[0].total_cmp(&b.px[0])));
    if !all {
        best.truncate(1);
    }
    Ok(Boxes { image: path.display().to_string(), width: w as u32, height: h as u32, phrase: phrase.to_string(), matches: best })
}

#[derive(Clone, Copy)]
enum Pre {
    None,
    Polarity,
    Invert,
}

fn decode_gray(path: &Path) -> Result<(usize, usize, Vec<u8>)> {
    let o = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x"])
        .arg(path)
        .output()
        .context("running ffprobe")?;
    let s = String::from_utf8_lossy(&o.stdout);
    let (w, h) = s.trim().split_once('x').and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))).ok_or_else(|| anyhow!("{}: not an image", path.display()))?;
    // flatten transparency onto white, then gray
    let o = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(path)
        .args(["-filter_complex", "[0:v]format=rgba[i];color=white:s=16x16[w];[w][i]scale2ref[w][i];[w][i]overlay=format=auto,format=gray", "-frames:v", "1", "-f", "rawvideo", "-"])
        .output()
        .context("decoding the image with ffmpeg")?;
    if o.stdout.len() != w * h {
        return Err(anyhow!("{}: decoded {} bytes, expected {}", path.display(), o.stdout.len(), w * h));
    }
    Ok((w, h, o.stdout))
}

fn ocr_words(img: &[u8], w: usize, h: usize, psm: u32) -> Result<Vec<OWord>> {
    let mut pgm = format!("P5\n{w} {h}\n255\n").into_bytes();
    pgm.extend_from_slice(img);
    let bin = std::env::var("CUTROOM_TESSERACT").unwrap_or_else(|_| "tesseract".into());
    let mut child = Command::new(&bin)
        .args(["stdin", "stdout", "-l", "eng", "--psm", &psm.to_string(), "tsv"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("running {bin} (is tesseract installed?)"))?;
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let tsv = std::thread::scope(|s| {
        s.spawn(move || {
            let _ = stdin.write_all(&pgm);
        });
        let mut out = String::new();
        stdout.read_to_string(&mut out).map(|_| out)
    })?;
    child.wait()?;
    Ok(parse(&tsv))
}

/// Level-5 TSV rows: `level page block par line word left top width height conf text`.
fn parse(tsv: &str) -> Vec<OWord> {
    let mut out = vec![];
    for line in tsv.lines().skip(1) {
        let f: Vec<&str> = line.splitn(12, '\t').collect();
        if f.len() < 12 || f[0] != "5" || f[11].trim().is_empty() {
            continue;
        }
        let n = |i: usize| f[i].trim().parse::<f32>().unwrap_or(0.0);
        let u = |i: usize| f[i].trim().parse::<u32>().unwrap_or(0);
        out.push(OWord { text: f[11].trim().to_string(), conf: n(10), b: [n(6), n(7), n(6) + n(8), n(7) + n(9)], line: (u(2), u(3), u(4)) });
    }
    out
}

/// Every place `target` (normalized) occurs in the words; fuzzy ones when there is no exact one.
fn matches(words: &[OWord], target: &str, w: usize, h: usize, pad: f32) -> Vec<BoxMatch> {
    // concatenated normalized text, with the word each char came from and its offset in the word
    let mut chars: Vec<char> = vec![];
    let mut owner: Vec<(usize, usize)> = vec![];
    let normed: Vec<Vec<char>> = words.iter().map(|x| norm(&x.text).chars().collect()).collect();
    for (i, n) in normed.iter().enumerate() {
        for (k, c) in n.iter().enumerate() {
            chars.push(*c);
            owner.push((i, k));
        }
    }
    let t: Vec<char> = target.chars().collect();
    let mut spans: Vec<(usize, usize, usize)> = vec![]; // (start char, end char exclusive, distance)
    if t.len() <= chars.len() {
        let mut i = 0;
        while i + t.len() <= chars.len() {
            if chars[i..i + t.len()] == t[..] {
                spans.push((i, i + t.len(), 0));
                i += t.len();
            } else {
                i += 1;
            }
        }
    }
    if spans.is_empty() {
        let k = (t.len() / 8).max(1);
        if let Some(s) = fuzzy(&chars, &t, k) {
            spans.push(s);
        }
    }
    spans.into_iter().map(|(a, b, d)| to_match(words, &normed, &owner, a, b, d, w, h, pad)).collect()
}

/// Best approximate occurrence of `t` in `text` with at most `k` edits (Sellers): (start, end, d).
fn fuzzy(text: &[char], t: &[char], k: usize) -> Option<(usize, usize, usize)> {
    let m = t.len();
    // column DP over the text, tracking where each alignment started
    let mut prev: Vec<(usize, usize)> = (0..=m).map(|i| (i, 0)).collect();
    let mut best: Option<(usize, usize, usize)> = None;
    for (j, &c) in text.iter().enumerate() {
        let mut cur = vec![(0usize, j + 1); m + 1];
        for i in 1..=m {
            let sub = (prev[i - 1].0 + usize::from(t[i - 1] != c), prev[i - 1].1);
            let del = (prev[i].0 + 1, prev[i].1);
            let ins = (cur[i - 1].0 + 1, cur[i - 1].1);
            cur[i] = [sub, del, ins].into_iter().min_by_key(|x| x.0).unwrap();
        }
        let (d, start) = cur[m];
        if d <= k && best.is_none_or(|b| d < b.2) {
            best = Some((start, j + 1, d));
        }
        prev = cur;
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn to_match(words: &[OWord], normed: &[Vec<char>], owner: &[(usize, usize)], a: usize, b: usize, d: usize, w: usize, h: usize, pad: f32) -> BoxMatch {
    let (wa, ka) = owner[a];
    let (wb, kb) = owner[b - 1];
    let mut lines: Vec<((u32, u32, u32), [f32; 4])> = vec![];
    let mut conf = 0.0;
    for i in wa..=wb {
        let x = &words[i];
        let n = normed[i].len().max(1) as f32;
        let (c0, c1) = (if i == wa { ka as f32 } else { 0.0 }, if i == wb { kb as f32 + 1.0 } else { n });
        let bw = x.b[2] - x.b[0];
        // trim partly covered words in proportion to the characters covered
        let bx = [x.b[0] + bw * c0 / n, x.b[1], x.b[0] + bw * c1 / n, x.b[3]];
        conf += x.conf;
        match lines.last_mut() {
            Some((k, r)) if *k == x.line => *r = [r[0].min(bx[0]), r[1].min(bx[1]), r[2].max(bx[2]), r[3].max(bx[3])],
            _ => lines.push((x.line, bx)),
        }
    }
    let (wf, hf) = (w as f32, h as f32);
    let padded = |r: [f32; 4]| [(r[0] - pad).max(0.0), (r[1] - pad).max(0.0), (r[2] + pad).min(wf), (r[3] + pad).min(hf)];
    let xywh = |r: [f32; 4]| [r[0], r[1], r[2] - r[0], r[3] - r[1]];
    let normd = |r: [f32; 4]| [(r[0] / wf) as f64, (r[1] / hf) as f64, ((r[2] - r[0]) / wf) as f64, ((r[3] - r[1]) / hf) as f64].map(|v| (v * 10000.0).round() / 10000.0);
    let mut u = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    let line_boxes: Vec<LineBox> = lines
        .iter()
        .map(|(_, r)| {
            let r = padded(*r);
            u = [u[0].min(r[0]), u[1].min(r[1]), u[2].max(r[2]), u[3].max(r[3])];
            LineBox { px: xywh(r), norm: normd(r) }
        })
        .collect();
    BoxMatch {
        text: words[wa..=wb].iter().map(|x| x.text.as_str()).collect::<Vec<_>>().join(" "),
        exact: d == 0,
        distance: d,
        conf: conf / (wb - wa + 1) as f32,
        px: xywh(u),
        norm: normd(u),
        lines: line_boxes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(t: &str, x: f32, line: u32) -> OWord {
        OWord { text: t.into(), conf: 90.0, b: [x, line as f32 * 40.0, x + 10.0 * t.len() as f32, line as f32 * 40.0 + 30.0], line: (1, 1, line) }
    }

    #[test]
    fn finds_across_words_lines_and_punctuation() {
        let ws = vec![word("Starting", 0.0, 1), word("to", 90.0, 1), word("experiment", 120.0, 1), word("with", 230.0, 1), word("agents", 280.0, 1), word("kicking", 0.0, 2), word("off", 80.0, 2), word("agents,", 120.0, 2)];
        let m = matches(&ws, &norm("agents kicking off agents"), 1000, 200, 0.0);
        assert_eq!(m.len(), 1);
        let x = &m[0];
        assert!(x.exact && x.distance == 0);
        assert_eq!(x.lines.len(), 2, "spans two lines");
        assert_eq!(x.lines[0].px, [280.0, 40.0, 60.0, 30.0]);
        // "agents," trimmed of nothing (the comma is not a normalized char, the word box stays whole)
        assert_eq!(x.lines[1].px[0], 0.0);
        assert_eq!(x.text, "agents kicking off agents,");
    }

    #[test]
    fn partial_words_are_trimmed_and_fuzzy_matches_work() {
        let ws = vec![word("subagents", 0.0, 1), word("rock", 100.0, 1)];
        let m = matches(&ws, &norm("agents rock"), 500, 100, 0.0);
        assert!(m[0].exact);
        // "agents" is the last 6 of 9 chars of "subagents" (90 px wide)
        assert!((m[0].px[0] - 30.0).abs() < 0.01, "{:?}", m[0].px);
        let ws = vec![word("Clauce", 0.0, 1), word("Code", 70.0, 1)];
        let m = matches(&ws, &norm("Claude Code"), 500, 100, 0.0);
        assert_eq!(m.len(), 1);
        assert!(!m[0].exact && m[0].distance == 1);
    }

    #[test]
    fn tsv_parsing() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t2\t1\t10\t20\t30\t40\t95.5\tHello\n4\t1\t1\t1\t2\t0\t10\t20\t300\t40\t-1\t\n";
        let w = parse(tsv);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].b, [10.0, 20.0, 40.0, 60.0]);
        assert_eq!(w[0].line, (1, 1, 2));
    }
}
