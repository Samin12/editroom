//! The editorial cut pass: what the detector missed (retakes it could not pair, section restarts,
//! repeats, abandoned lines), judged by a model over overlapping chunks of the kept transcript.
//! Answers are validated against the words (each cut quotes what it removes), then merged with
//! the detector's cuts.

use super::brief::Ctx;
use cutroom_core::anchor::norm;
use cutroom_cut::{CutReport, Span};
use serde_yaml::Value;

pub const WHYS: [&str; 7] = ["retake", "false_start", "abandoned", "repeat", "self_talk", "filler", "section_restart"];
/// Longest single cut (source ms), and the longest for a retake / section restart.
pub const MAX_CUT_MS: f64 = 90_000.0;
pub const MAX_RETAKE_MS: f64 = 300_000.0;
/// Shorter cuts are left out: a jump cut on a visible face for a fraction of a second costs more
/// than the word it removes (Samin's policy: no removal under 300 ms).
pub const MIN_CUT_MS: f64 = 300.0;
/// Most of a chunk's kept words one answer may remove.
pub const MAX_SHARE: f64 = 0.5;

/// A stretch of kept positions `[lo, hi)`. Its answer counts for items that start in
/// `[own_lo, own_hi)` (source word ids): where chunks overlap, the later chunk wins.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub k: usize,
    pub lo: usize,
    pub hi: usize,
    pub own_lo: usize,
    pub own_hi: usize,
}

/// Split `kept` (word ids) into chunks of about `target` words overlapping by `overlap` words.
pub fn chunks(kept: &[usize], target: usize, overlap: usize) -> Vec<Chunk> {
    let n = kept.len();
    if n == 0 {
        return vec![];
    }
    let target = target.max(50);
    let overlap = overlap.min(target / 2);
    let step = target - overlap;
    let mut starts = vec![0usize];
    while starts.last().unwrap() + target < n {
        starts.push(starts.last().unwrap() + step);
    }
    // a last chunk that would be mostly overlap merges into the one before
    if starts.len() > 1 && n - starts[starts.len() - 1] < overlap + step / 4 {
        starts.pop();
    }
    let mut out = vec![];
    for (k, &s) in starts.iter().enumerate() {
        let next = starts.get(k + 1).copied();
        let hi = match next {
            Some(_) => (s + target).min(n),
            None => n,
        };
        out.push(Chunk { k, lo: s, hi, own_lo: if k == 0 { 0 } else { kept[s] }, own_hi: next.map(|x| kept[x]).unwrap_or(usize::MAX) });
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdCut {
    pub from: usize,
    pub to: usize,
    pub why: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdPause {
    /// Index into `CutReport::auto` of the silence cut that gives time back.
    pub cut: usize,
    pub after: usize,
    pub keep_ms: f64,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct EdResult {
    pub cuts: Vec<EdCut>,
    /// Review items to accept (0-based).
    pub accept: Vec<usize>,
    pub pauses: Vec<EdPause>,
    /// Problems, phrased for the model's repair round.
    pub errors: Vec<String>,
    /// The answer removes more than `MAX_SHARE` of the chunk.
    pub over_cap: bool,
}

fn word_id(v: &Value) -> Option<usize> {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    s.trim().trim_start_matches(['w', 'W']).parse().ok()
}

fn words_of(t: &str) -> Vec<String> {
    t.split_whitespace().map(norm).filter(|w| !w.is_empty()).collect()
}

/// Exact word range for a cut that quotes its words: `text` is all the words, or the first few,
/// ` ... ` and the last few. Looks near the given ids first, then anywhere in `[lo, hi]`.
pub fn snap(src_words: &[String], from: usize, to: usize, text: &str, lo: usize, hi: usize) -> Option<(usize, usize)> {
    let t = text.replace('…', " ... ");
    let (head, tail) = match t.split_once("...") {
        Some((a, b)) => (words_of(a), words_of(b)),
        None => (words_of(&t), vec![]),
    };
    if head.is_empty() {
        return None;
    }
    let hi = hi.min(src_words.len().saturating_sub(1));
    let matches_at = |i: usize, seq: &[String]| i + seq.len() <= src_words.len() && src_words[i..i + seq.len()].iter().zip(seq).all(|(a, b)| a == b);
    let near = |c: usize, r: usize| -> Vec<usize> {
        let mut v: Vec<usize> = (c.saturating_sub(r)..=(c + r)).filter(|&i| i >= lo && i <= hi).collect();
        v.sort_by_key(|&i| (i as i64 - c as i64).abs());
        v
    };
    let find = |seq: &[String], around: usize| -> Option<usize> {
        near(around, 10).into_iter().find(|&i| matches_at(i, seq)).or_else(|| {
            let all: Vec<usize> = (lo..=hi).filter(|&i| matches_at(i, seq)).collect();
            (all.len() == 1).then(|| all[0])
        })
    };
    if tail.is_empty() {
        let a = find(&head, from)?;
        return Some((a, a + head.len() - 1));
    }
    let a = find(&head, from)?;
    // the tail must end at or after the head
    let b = near(to, 12)
        .into_iter()
        .filter(|&j| j + 1 >= tail.len() && j + 1 - tail.len() >= a)
        .find(|&j| matches_at(j + 1 - tail.len(), &tail))
        .or_else(|| (a..=hi).filter(|&j| j + 1 >= tail.len() && j + 1 - tail.len() >= a).find(|&j| matches_at(j + 1 - tail.len(), &tail)))?;
    Some((a, b))
}

/// Parse and check one chunk's answer.
pub fn validate(ans: &str, ch: &Chunk, c: &Ctx, report: &CutReport, norm_words: &[String]) -> EdResult {
    let mut r = EdResult::default();
    let doc: Value = match serde_yaml::from_str(ans) {
        Ok(v) => v,
        Err(e) => {
            r.errors.push(format!("not valid YAML: {e}"));
            return r;
        }
    };
    let lo_id = c.kept[ch.lo];
    let hi_id = c.kept[ch.hi - 1];
    let w = &c.src.words;

    // ---- accept ----
    let acc: Vec<String> = match &doc["accept"] {
        Value::Sequence(s) => s.iter().filter_map(|v| v.as_str().map(String::from).or_else(|| v.as_u64().map(|n| n.to_string()))).collect(),
        Value::String(s) => s.split([',', ' ']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty() && !x.eq_ignore_ascii_case("none")).collect(),
        Value::Number(n) => vec![n.to_string()],
        _ => vec![],
    };
    for a in acc {
        let Ok(k) = a.trim_start_matches(['r', 'R']).parse::<usize>() else {
            r.errors.push(format!("accept: {a:?} is not an item like r3"));
            continue;
        };
        match report.review.get(k.wrapping_sub(1)) {
            Some(p) => {
                if let Span::Words(x, _) = p.span {
                    if x >= lo_id && x <= hi_id {
                        r.accept.push(k - 1);
                    }
                }
            }
            None => r.errors.push(format!("accept: r{k} does not exist")),
        }
    }

    // ---- cuts ----
    let mut removed: std::collections::BTreeSet<usize> = Default::default();
    if let Value::Sequence(cuts) = &doc["cuts"] {
        for (i, v) in cuts.iter().enumerate() {
            let (Some(from), to) = (word_id(&v["from"]), word_id(&v["to"])) else {
                r.errors.push(format!("cut #{}: `from` must be a word id like w120", i + 1));
                continue;
            };
            let to = to.unwrap_or(from);
            let why = v["why"].as_str().unwrap_or("edit").trim().to_lowercase();
            let why = if WHYS.contains(&why.as_str()) { why } else { "edit".into() };
            let note = v["note"].as_str().map(|s| s.chars().take(80).collect::<String>());
            let (a, b) = match v["text"].as_str() {
                Some(t) if !t.trim().is_empty() => match snap(norm_words, from, to, t, lo_id, hi_id) {
                    Some(x) => x,
                    None => {
                        let s0 = from.min(w.len() - 1);
                        let got: Vec<&str> = w[s0..=(s0 + 8).min(w.len() - 1)].iter().map(|x| x.text.as_str()).collect();
                        r.errors.push(format!("cut w{from}-w{to}: text {t:?} does not match the words there (w{from} starts {:?})", got.join(" ")));
                        continue;
                    }
                },
                _ => (from, to),
            };
            if a > b {
                r.errors.push(format!("cut w{from}-w{to}: `to` comes before `from`"));
                continue;
            }
            if a < lo_id || b > hi_id {
                r.errors.push(format!("cut w{a}-w{b} is outside this stretch (w{lo_id}-w{hi_id})"));
                continue;
            }
            let dur = w[b].end_ms - w[a].start_ms;
            let max = if why == "retake" || why == "section_restart" { MAX_RETAKE_MS } else { MAX_CUT_MS };
            if dur > max {
                r.errors.push(format!("cut w{a}-w{b} ({}) removes {:.0}s; a single cut may remove at most {:.0}s (split it, or it is not one retake)", why, dur / 1000.0, max / 1000.0));
                continue;
            }
            if dur < MIN_CUT_MS {
                continue;
            }
            let kept_inside: Vec<usize> = (a..=b).filter(|&i| c.pos[i].is_some()).collect();
            if kept_inside.is_empty() {
                continue; // already cut
            }
            removed.extend(kept_inside);
            r.cuts.push(EdCut { from: a, to: b, why, note });
        }
    }
    let share = removed.len() as f64 / (ch.hi - ch.lo).max(1) as f64;
    if share > MAX_SHARE {
        r.over_cap = true;
        r.errors.push(format!("these cuts remove {:.0}% of the stretch; that is far more than retakes and stumbles; cut only what is plainly redundant", share * 100.0));
    }

    // ---- pauses ----
    if let Value::Sequence(ps) = &doc["pauses"] {
        for v in ps {
            let Some(after) = word_id(&v["after"]) else {
                r.errors.push("pause: `after` must be a word id".into());
                continue;
            };
            let keep_ms = (v["keep"].as_f64().unwrap_or(1.5) * 1000.0).clamp(300.0, 2500.0);
            if after < lo_id || after > hi_id {
                continue;
            }
            match silence_after(report, &w.iter().map(|x| x.end_ms).collect::<Vec<_>>(), after) {
                Some(cut) => r.pauses.push(EdPause { cut, after, keep_ms, note: v["note"].as_str().map(String::from) }),
                None => r.errors.push(format!("pause after w{after}: no removed silence there")),
            }
        }
    }
    r
}

/// The detector silence cut (index into `auto`) that starts right after word `after` (±2 words).
fn silence_after(report: &CutReport, word_end: &[f64], after: usize) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for (w, &e) in word_end.iter().enumerate().skip(after.saturating_sub(2)).take(5) {
        for (i, p) in report.auto.iter().enumerate() {
            if let Span::Ms(a, b) = p.span {
                if a >= e - 50.0 && a <= e + 700.0 && b - a >= 900.0 {
                    let d = (w as i64 - after as i64).unsigned_abs() as usize;
                    if best.map(|x| d < x.1).unwrap_or(true) {
                        best = Some((i, d));
                    }
                }
            }
        }
    }
    best.map(|b| b.0)
}

/// Apply kept pauses to the detector's silence cuts: give back `keep_ms` at the start of each,
/// dropping cuts that become too short to matter.
pub fn apply_pauses(report: &mut CutReport, pauses: &[EdPause]) -> usize {
    let mut keep: std::collections::BTreeMap<usize, f64> = Default::default();
    for p in pauses {
        let e = keep.entry(p.cut).or_insert(0.0);
        *e = e.max(p.keep_ms);
    }
    let mut drop = vec![];
    for (&i, &k) in &keep {
        if let Some(p) = report.auto.get_mut(i) {
            if let Span::Ms(a, b) = p.span {
                if b - (a + k) < 300.0 {
                    drop.push(i);
                } else {
                    p.span = Span::Ms(a + k, b);
                }
            }
        }
    }
    for i in drop.into_iter().rev() {
        report.auto.remove(i);
    }
    keep.len()
}

/// Items of a chunk's result that the chunk owns (start inside its own range).
pub fn owned(r: EdResult, ch: &Chunk, report: &CutReport) -> EdResult {
    let mine = |id: usize| id >= ch.own_lo && id < ch.own_hi;
    EdResult {
        cuts: r.cuts.into_iter().filter(|x| mine(x.from)).collect(),
        accept: r.accept.into_iter().filter(|&k| matches!(report.review[k].span, Span::Words(a, _) if mine(a))).collect(),
        pauses: r.pauses.into_iter().filter(|p| mine(p.after)).collect(),
        errors: r.errors,
        over_cap: r.over_cap,
    }
}

/// Merge overlapping or touching editorial cuts (word ranges); the first reason wins.
pub fn merge_cuts(mut v: Vec<EdCut>) -> Vec<EdCut> {
    v.sort_by_key(|c| (c.from, c.to));
    let mut out: Vec<EdCut> = vec![];
    for c in v {
        if let Some(l) = out.last_mut() {
            if c.from <= l.to + 1 {
                l.to = l.to.max(c.to);
                continue;
            }
        }
        out.push(c);
    }
    out
}
