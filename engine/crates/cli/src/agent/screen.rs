//! The screen pass: for each stretch where his screen recording shows, grab frames of the screen
//! recording itself (so coordinates are screen-normalized, as overlays expect), let the model look
//! at them next to what he says, and keep the rings / highlights / zooms / arrows / callouts that
//! validate.

use super::brief::{brief, Ctx};
use cutroom_core::compose::Seg;
use cutroom_core::spec::{BeatKind, BeatSpec};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Stretches shorter than this get no screen pass; longer ones are split into windows.
pub const MIN_WINDOW_MS: f64 = 4000.0;
pub const MAX_WINDOW_MS: f64 = 45_000.0;
/// Marks one window may keep.
pub const MAX_PER_WINDOW: usize = 3;

#[derive(Clone, Debug)]
pub struct Frame {
    pub out_ms: f64,
    pub media: PathBuf,
    pub media_ms: f64,
    /// Kept word he says around then.
    pub word: usize,
}

#[derive(Clone, Debug)]
pub struct Window {
    pub k: usize,
    /// Kept positions `[lo, hi)`.
    pub lo: usize,
    pub hi: usize,
    pub frames: Vec<Frame>,
}

/// Screen windows of a plan: runs of kept words where the screen shows, split at long gaps and
/// at `MAX_WINDOW_MS`.
pub fn windows(c: &Ctx, segs: &[Seg]) -> Vec<Window> {
    let mut runs: Vec<(usize, usize)> = vec![];
    let mut start: Option<usize> = None;
    for p in 0..=c.kept.len() {
        let on = p < c.kept.len() && c.screen[p];
        let gap = p > 0 && p < c.kept.len() && c.out_ms[p] - c.out_ms[p - 1] > 2500.0;
        match (start, on) {
            (Some(s), true) if gap => {
                runs.push((s, p));
                start = Some(p);
            }
            (Some(s), false) => {
                runs.push((s, p));
                start = None;
            }
            (None, true) => start = Some(p),
            _ => {}
        }
    }
    let mut out = vec![];
    for (a, b) in runs {
        let t0 = c.out_ms[a];
        let t1 = c.out_ms[b - 1] + 400.0;
        if t1 - t0 < MIN_WINDOW_MS || b - a < 5 {
            continue;
        }
        let parts = ((t1 - t0) / MAX_WINDOW_MS).ceil().max(1.0) as usize;
        let per = (t1 - t0) / parts as f64;
        let mut lo = a;
        for i in 0..parts {
            let end_t = t0 + per * (i + 1) as f64;
            let hi = if i + 1 == parts { b } else { lo + c.out_ms[lo..b].partition_point(|&t| t < end_t) };
            if hi > lo + 3 {
                let w0 = c.out_ms[lo];
                let w1 = c.out_ms[hi - 1] + 400.0;
                let at: Vec<f64> = if w1 - w0 < 12_000.0 { vec![0.5] } else { vec![0.3, 0.72] };
                let frames = at.iter().filter_map(|f| frame_at(c, segs, lo, hi, w0 + (w1 - w0) * f)).collect::<Vec<_>>();
                if !frames.is_empty() {
                    out.push(Window { k: out.len(), lo, hi, frames });
                }
            }
            lo = hi;
        }
    }
    out
}

fn frame_at(c: &Ctx, segs: &[Seg], lo: usize, hi: usize, t: f64) -> Option<Frame> {
    let s = segs.iter().find(|s| s.out0 <= t && t < s.out1 && s.screen.is_some())?;
    let src_ms = s.src_at(t)?;
    let clip = &c.src.clips[s.screen?];
    let media = c.src.media(&clip.media)?;
    let p = (lo + c.out_ms[lo..hi].partition_point(|&x| x <= t)).saturating_sub(1).max(lo);
    Some(Frame { out_ms: t, media: media.path.clone(), media_ms: clip.media_at(src_ms), word: c.kept[p] })
}

/// One JPEG of the screen recording, at most 1280 px wide.
pub fn extract(f: &Frame, out: &Path) -> anyhow::Result<()> {
    let st = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-ss", &format!("{:.3}", f.media_ms / 1000.0), "-i"])
        .arg(&f.media)
        .args(["-frames:v", "1", "-vf", "scale='min(1280,iw)':-2", "-q:v", "4"])
        .arg(out)
        .status()?;
    if !st.success() || !out.exists() {
        anyhow::bail!("ffmpeg could not grab a frame of {} at {:.1}s", f.media.display(), f.media_ms / 1000.0);
    }
    Ok(())
}

/// The prompt for one window (frames are attached separately, in order).
pub fn prompt(c: &Ctx, w: &Window, glossary: Option<&str>) -> String {
    let mut s = String::from(super::prompts::SCREEN_PROMPT);
    s.push_str(&super::prompts::names_rule(glossary));
    s.push('\n');
    for (i, f) in w.frames.iter().enumerate() {
        s.push_str(&format!("Image {}: the screen at {} (he is saying w{} {:?}).\n", i + 1, cutroom_core::resolve::clock(f.out_ms), f.word, c.src.words[f.word].text));
    }
    s.push('\n');
    s.push_str(super::prompts::TRANSCRIPT_KEY);
    s.push_str("\n\nWhat he says:\n");
    s.push_str(&brief(c, w.lo, w.hi, 6, None, 30));
    s
}

fn word_of(a: &str) -> Option<usize> {
    a.trim().strip_prefix('w').and_then(|x| x.parse::<usize>().ok())
}

/// A kept word of the window for an anchor the model gave: itself when kept and inside, else the
/// nearest kept word inside the window (forward for a start, backward for an end), when it is
/// at most 8 words away.
fn snap_word(c: &Ctx, w: &Window, id: usize, forward: bool) -> Option<usize> {
    let (lo, hi) = (c.kept[w.lo], c.kept[w.hi - 1]);
    if id + 8 < lo || id > hi + 8 {
        return None;
    }
    let id = id.clamp(lo, hi);
    let p = match c.pos[id] {
        Some(p) => p,
        None if forward => c.pos_at_or_after(id).min(w.hi - 1),
        None => c.pos_at_or_after(id).saturating_sub(1).max(w.lo),
    };
    Some(c.kept[p])
}

fn rect_ok(r: &[f64; 4]) -> bool {
    r.iter().all(|v| v.is_finite()) && r[0] >= 0.0 && r[1] >= 0.0 && r[2] >= 0.01 && r[3] >= 0.01 && r[0] + r[2] <= 1.001 && r[1] + r[3] <= 1.001
}

/// Keep the marks of one answer that are well-formed for this window; the rest with a reason.
pub fn check_marks(c: &Ctx, w: &Window, beats: Vec<(BeatSpec, String)>) -> (Vec<BeatSpec>, Vec<(String, String)>) {
    let mut keep = vec![];
    let mut bad = vec![];
    for (mut b, raw) in beats {
        // anchors on a word that was cut, or just outside the stretch, move to the nearest kept word
        let at = word_of(&b.at.0).and_then(|id| snap_word(c, w, id, true));
        let until = b.until.as_ref().map(|u| word_of(&u.0).and_then(|id| snap_word(c, w, id, false)));
        if let Some(a) = at {
            b.at = cutroom_core::spec::Anchor(format!("w{a}"));
        }
        match until {
            Some(Some(u)) if Some(u) > at => b.until = Some(cutroom_core::spec::Anchor(format!("w{u}"))),
            Some(_) => b.until = None,
            None => {}
        }
        let why = match &b.kind {
            BeatKind::Ring { rect } | BeatKind::Highlight { rect, .. } if !rect_ok(rect) => Some("rect outside the frame or too small".to_string()),
            BeatKind::Ring { rect } if rect[2] > 0.6 || rect[3] > 0.6 => Some("ring larger than 0.6 of the frame: circle one thing".into()),
            BeatKind::Highlight { rect, .. } if rect[3] > 0.15 => Some("highlight taller than 0.15: mark a line or two of text".into()),
            BeatKind::Zoom { rect } if !rect_ok(rect) || rect[2] < 0.25 || rect[3] < 0.2 => Some("zoom rect must be inside the frame and at least 0.25 x 0.2".into()),
            BeatKind::Arrow { from, to } if from.iter().chain(to).any(|v| !(0.0..=1.0).contains(v)) => Some("arrow points outside the frame".into()),
            BeatKind::Callout { text, .. } if text.split_whitespace().count() > 6 => Some("callout longer than 6 words".into()),
            BeatKind::Ring { .. } | BeatKind::Highlight { .. } | BeatKind::Zoom { .. } | BeatKind::Arrow { .. } | BeatKind::Callout { .. } => None,
            k => Some(format!("{} is not a screen mark", k.name())),
        };
        let why = why.or_else(|| at.is_none().then(|| format!("at {} is not near a kept word of this stretch", b.at.0)));
        match why {
            Some(y) => bad.push((raw, y)),
            None if keep.len() >= MAX_PER_WINDOW => bad.push((raw, format!("more than {MAX_PER_WINDOW} marks in one stretch"))),
            None => keep.push(b),
        }
    }
    (keep, bad)
}
