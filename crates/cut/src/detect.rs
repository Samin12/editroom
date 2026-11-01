//! The detectors. Each pushes raw proposals; `lib.rs` buckets and merges them.

use crate::text::{wlcs, Text};
use crate::{CutOpts, Proposal, Span, Why};
use cutroom_core::Source;
use std::collections::{HashMap, HashSet};

/// Restart features, kept for calibration (`detect_candidates`).
#[derive(Clone, Debug, Default)]
pub struct Feat {
    /// Content tokens in the abandoned attempt.
    pub len: usize,
    /// Exact matching tokens at the start of both attempts.
    pub prefix: usize,
    /// Weighted similarity of the two openings (0..1).
    pub sim: f32,
    /// Weighted share of the whole abandoned attempt re-said by the restart (0..1).
    pub cover: f32,
    /// Matched weight of the openings (content words count 1, stop words 0.25).
    pub info: f32,
    pub restart_at_clause: bool,
    pub attempt_at_clause: bool,
    /// The abandoned attempt stops without a sentence end.
    pub incomplete: bool,
    pub cross_take: bool,
    pub pause_ms: f64,
    /// Sentence ends inside the abandoned attempt (not counting its last word).
    pub inner_ends: usize,
}

/// um / uh. A filler token shorter than 60 ms is usually a recognition artefact, not a sound.
pub fn fillers(t: &Text, src: &Source, out: &mut Vec<Proposal>) {
    for (i, tk) in t.toks.iter().enumerate() {
        if tk.filler {
            let w = &src.words[i];
            let conf = if w.end_ms - w.start_ms < 60.0 { 0.3 } else { 0.9 };
            out.push(Proposal { span: Span::Words(i, i), why: Why::Filler, confidence: conf, reason: format!("filler {:?}", tk.norm) });
        }
    }
}

/// Immediate repeats of 1-4 words ("can be can be", "I I") and cut-off words ("wan-").
/// Not stutters: emphasis ("very, very"), a sentence end followed by the same word ("this. This
/// is"), and "bit by bit" / "over and over" / "picture in picture".
pub fn stutters(t: &Text, out: &mut Vec<Proposal>) {
    let n = t.seq.len();
    let word = |y: usize| &t.toks[t.ci[y]];
    let mut x = 0;
    while x < n {
        let mut hit = None;
        for len in (1..=4).rev() {
            if x + 2 * len > n || t.seq[x..x + len] != t.seq[x + len..x + 2 * len] || !t.same_take(x, x + 2 * len - 1) {
                continue;
            }
            // the second copy must follow promptly
            if word(x + len).gap_before > 1500.0 {
                continue;
            }
            hit = Some(len);
            break;
        }
        let Some(len) = hit else {
            let i = t.ci[x];
            if t.toks[i].norm.len() > 1 && t.toks[i].cut_off {
                out.push(Proposal { span: Span::Words(i, i), why: Why::FalseStart, confidence: 0.85, reason: "cut-off word".into() });
            }
            x += 1;
            continue;
        };
        let (a, b) = (x, x + 2 * len);
        x += len;
        // "X by X", "X and X", "X in X" inside the doubled stretch
        let joined = (a..b.saturating_sub(2)).any(|y| t.seq[y] == t.seq[y + 2] && matches!(word(y + 1).norm.as_str(), "by" | "and" | "in" | "to" | "after" | "or" | "on" | "for" | "upon"));
        if joined {
            continue;
        }
        let first_end = word(a + len - 1);
        let conf = if len == 1 {
            let w = word(a);
            if first_end.ends_punct || crate::text::EMPHATIC.contains(&w.norm.as_str()) || !w.stop {
                continue;
            }
            0.55
        } else if first_end.ends_sentence {
            // a whole short sentence said twice: often deliberate
            0.6
        } else if first_end.ends_punct {
            0.75
        } else {
            0.9
        };
        let words: Vec<&str> = (a..a + len).map(|y| word(y).norm.as_str()).collect();
        let (from, to) = (t.ci[a], t.ci[a + len] - 1);
        out.push(Proposal { span: Span::Words(from, to), why: Why::FalseStart, confidence: conf, reason: format!("repeat {:?}", words.join(" ")) });
    }
}

fn lcp(seq: &[u32], q: usize, p: usize, cap: usize) -> usize {
    let mut k = 0;
    while k < cap && p + k < seq.len() && seq[q + k] == seq[p + k] {
        k += 1;
    }
    k
}

/// A restart: the speaker abandons an attempt (`q..p`) and starts it again at `p`; the last
/// attempt is the keeper. Candidates come from an exact bigram (or a rare word) seen again
/// within `window_words`, scored on how much of the opening is re-said.
pub fn restarts(t: &Text, o: &CutOpts, mut sink: impl FnMut(Proposal, Feat)) {
    let seq = &t.seq;
    let n = seq.len();
    let win = o.window_words;
    let mut big: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    let mut uni: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut dp = vec![];
    let mut qs: Vec<usize> = vec![];
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for p in 0..n {
        qs.clear();
        if p + 1 < n {
            if let Some(v) = big.get(&(seq[p], seq[p + 1])) {
                qs.extend(v.iter().rev().take_while(|&&q| q + win >= p).copied());
            }
        }
        if t.weight[p] >= 1.0 {
            if let Some(v) = uni.get(&seq[p]) {
                qs.extend(v.iter().rev().take_while(|&&q| q + win >= p).copied());
            }
        }
        qs.sort_unstable_by(|a, b| b.cmp(a));
        qs.dedup();
        let mut best: Option<(f32, usize, Feat)> = None;
        for &q0 in &qs {
            if !t.same_take(q0, p) {
                continue;
            }
            // shift the pair back while the word before the attempt equals its last word: the
            // same edit, but it keeps the later copy and finds the true restart point
            let (mut q, mut pp) = (q0, p);
            while q > 0 && seq[q - 1] == seq[pp - 1] && t.same_take(q - 1, q) {
                q -= 1;
                pp -= 1;
            }
            let len = pp - q;
            let k = lcp(seq, q, pp, len);
            // exact immediate repeats are stutters
            if len <= 4 && k >= len {
                continue;
            }
            if seen.contains(&(q, pp)) {
                continue;
            }
            let Some((score, f)) = score_pair(t, q, pp, k, &mut dp) else { continue };
            if best.as_ref().is_none_or(|b| score > b.0 + 0.02) {
                best = Some((score, q, f));
            }
        }
        if p + 1 < n {
            big.entry((seq[p], seq[p + 1])).or_default().push(p);
        }
        if t.weight[p] >= 1.0 {
            uni.entry(seq[p]).or_default().push(p);
        }
        let Some((score, q, f)) = best else { continue };
        let p = q + f.len;
        seen.insert((q, p));
        let Some((from, to)) = cut_span(t, q, p, f.restart_at_clause) else { continue };
        let head: Vec<&str> = (p..(p + 4).min(n)).map(|y| t.toks[t.ci[y]].norm.as_str()).collect();
        sink(Proposal { span: Span::Words(from, to), why: Why::Retake, confidence: score, reason: format!("restarted at w{} {:?}", t.ci[p], head.join(" ")) }, f);
    }
}

fn score_pair(t: &Text, q: usize, p: usize, k: usize, dp: &mut Vec<f32>) -> Option<(f32, Feat)> {
    let seq = &t.seq;
    let n = seq.len();
    let len = p - q;
    let h = len.min(12);
    let a = &seq[q..q + h];
    let wa = &t.weight[q..q + h];
    let b = &seq[p..(p + h + 3).min(n)];
    let info = wlcs(a, wa, b, dp);
    let tot: f32 = wa.iter().sum();
    let sim = info / tot;
    if info < 1.0 || sim < 0.45 {
        return None;
    }
    let cover = if len > h {
        let l = len.min(80);
        let a = &seq[q..q + l];
        let wa = &t.weight[q..q + l];
        let b = &seq[p..(p + l + 8).min(n)];
        wlcs(a, wa, b, dp) / wa.iter().sum::<f32>()
    } else {
        sim
    };
    let (ip, ip1) = (t.ci[p], t.ci[p - 1]);
    let pause_ms = t.toks[ip1 + 1..=ip].iter().map(|x| x.gap_before).fold(0.0, f64::max);
    let f = Feat {
        len,
        prefix: k,
        sim,
        cover,
        info,
        restart_at_clause: t.clause_start(p),
        attempt_at_clause: t.clause_start(q),
        incomplete: !t.toks[ip1].ends_sentence,
        cross_take: !t.same_take(q, p),
        pause_ms: if pause_ms.is_finite() { pause_ms } else { 10_000.0 },
        inner_ends: (q..p - 1).filter(|&y| t.toks[t.ci[y]].ends_sentence).count(),
    };
    Some((confidence(&f), f))
}

/// Hand-tuned against Samin's course (see EVAL.md): mostly how much of the abandoned attempt
/// is said again, whether both attempts start at a clause, and how many sentences it spans.
pub fn confidence(f: &Feat) -> f32 {
    let c = f.cover;
    let mut s = 0.7 * c + 0.1 * f.sim;
    s += match (f.restart_at_clause, f.attempt_at_clause) {
        (true, true) => 0.25,
        (false, false) => -0.15,
        _ => 0.0,
    };
    s -= (0.08 * f.inner_ends as f32).min(0.3) * (1.2 - c);
    if f.prefix >= 4 {
        s += 0.05;
        // cut off mid-sentence, then said again word for word: the textbook restart
        if c >= 0.95 && f.incomplete {
            s += 0.05;
        }
    }
    if f.len < 3 {
        s -= 0.3;
    }
    if f.len >= 20 && c < 0.5 {
        s -= 0.15;
    }
    // a single re-said word ("hook, and the hook is...") is weak evidence
    if f.info < 1.5 {
        s -= 0.1;
    }
    if f.cross_take {
        s -= 0.3;
    }
    s.clamp(0.0, 1.0)
}

/// Word range removed for a restart pair: from the abandoned attempt's clause start to just
/// before the restart, keeping lead-in words ("And", "So") the restart opens with.
fn cut_span(t: &Text, q: usize, p: usize, restart_at_clause: bool) -> Option<(usize, usize)> {
    let lead = |y: usize| t.toks[t.ci[y]].lead;
    let mut s = q;
    if restart_at_clause && !t.boundary(q) {
        // take the attempt's own lead-in with it ("First, the rules..." -> "So the rules...")
        let mut x = q;
        while x > 0 && q - x < 2 && !t.boundary(x) && t.same_take(x - 1, q) {
            x -= 1;
        }
        if t.boundary(x) && x < q {
            s = x;
        }
    }
    let mut e = p;
    if e >= 2 && e - 1 > s && lead(e - 1) && t.boundary(e - 1) {
        e -= 1;
    } else if e >= 3 && e - 2 > s && lead(e - 1) && lead(e - 2) && t.boundary(e - 2) {
        e -= 2;
    }
    let from = t.ci[s];
    // include fillers right before the attempt
    let from = (0..from).rev().take_while(|&i| t.toks[i].skip).last().unwrap_or(from);
    let to = t.ci[e] - 1;
    (to >= from).then_some((from, to))
}

/// A take whose content is mostly re-said by one of the next takes (and was not recorded at the
/// same time, as multi-camera takes are) is abandoned.
pub fn abandoned_takes(t: &Text, src: &Source, out: &mut Vec<Proposal>) {
    let ntakes = t.takes.len();
    // content positions per take
    let mut by_take: Vec<Vec<usize>> = vec![vec![]; ntakes];
    for x in 0..t.seq.len() {
        let tk = t.toks[t.ci[x]].take as usize;
        if tk < ntakes {
            by_take[tk].push(x);
        }
    }
    let shingles = |xs: &[usize]| -> HashMap<(u32, u32, u32), usize> {
        let mut m = HashMap::new();
        for w in xs.windows(3) {
            m.entry((t.seq[w[0]], t.seq[w[1]], t.seq[w[2]])).or_insert(w[0]);
        }
        m
    };
    let sets: Vec<HashMap<(u32, u32, u32), usize>> = by_take.iter().map(|xs| shingles(xs)).collect();
    for a in 0..ntakes {
        if by_take[a].len() < 8 {
            continue;
        }
        for b in a + 1..(a + 4).min(ntakes) {
            let shared: Vec<(usize, usize)> = sets[a].iter().filter_map(|(k, &xa)| sets[b].get(k).map(|&xb| (xa, xb))).collect();
            let overlap = shared.len() as f32 / sets[a].len().max(1) as f32;
            if overlap < 0.5 {
                continue;
            }
            // same words at the same moment of both takes = recorded simultaneously
            let rel = |x: usize, tk: usize| src.words[t.ci[x]].start_ms - t.takes[tk].0;
            let same_time = shared.iter().filter(|(xa, xb)| (rel(*xa, a) - rel(*xb, b)).abs() < 400.0).count();
            if same_time * 2 > shared.len() {
                continue;
            }
            let xs = &by_take[a];
            let from = t.ci[xs[0]];
            let to = t.ci[*xs.last().unwrap()];
            out.push(Proposal { span: Span::Words(from, to), why: Why::Retake, confidence: (0.45 + 0.55 * overlap).min(0.98), reason: format!("take re-said by a later take ({:.0}%)", overlap * 100.0) });
            break;
        }
    }
}

/// Silences inside a take are shrunk to `silence_pad_ms` either side; dead air at take starts
/// and ends is cut down to `dead_air_pad_ms`.
pub fn silences(t: &Text, src: &Source, o: &CutOpts, out: &mut Vec<Proposal>) {
    let w = &src.words;
    for i in 1..w.len() {
        if t.toks[i].take != t.toks[i - 1].take {
            continue;
        }
        let gap = w[i].start_ms - w[i - 1].end_ms;
        if gap <= o.silence_min_ms {
            continue;
        }
        let a = w[i - 1].end_ms + o.silence_pad_ms;
        let b = w[i].start_ms - o.silence_pad_ms;
        if b - a >= 40.0 {
            out.push(Proposal { span: Span::Ms(a, b), why: Why::Silence, confidence: 0.9, reason: format!("{:.1}s pause", gap / 1000.0) });
        }
    }
    let mut seen: HashSet<u32> = HashSet::new();
    for (tk, &(t0, t1)) in t.takes.iter().enumerate() {
        if !t0.is_finite() || !seen.insert(tk as u32) {
            continue;
        }
        let first = t.toks.iter().position(|x| x.take == tk as u32);
        let last = t.toks.iter().rposition(|x| x.take == tk as u32);
        match (first, last) {
            (Some(f), Some(l)) => {
                let a = w[f].start_ms - o.dead_air_pad_ms;
                if a - t0 >= 40.0 {
                    out.push(Proposal { span: Span::Ms(t0, a), why: Why::DeadAir, confidence: 0.95, reason: "before the first word".into() });
                }
                let b = w[l].end_ms + o.dead_air_pad_ms;
                if t1 - b >= 40.0 {
                    out.push(Proposal { span: Span::Ms(b, t1), why: Why::DeadAir, confidence: 0.95, reason: "after the last word".into() });
                }
            }
            _ => out.push(Proposal { span: Span::Ms(t0, t1), why: Why::DeadAir, confidence: 0.6, reason: "take without words".into() }),
        }
    }
}
