//! Spec + Source -> Plan: every cut, chapter and beat turned into output time.

use crate::anchor::{self, Index, Resolved};
use crate::model::Source;
use crate::spec::{Anchor, BeatKind, CardMode, Lane, LayoutSpec, Loaded, OutputSpec};
use crate::timemap::TimeMap;
use serde::Serialize;
use std::collections::BTreeMap;

/// Silence kept next to a word when a cut removes its neighbour (ms).
pub const CUT_PAD_MS: f64 = 120.0;
/// Default lead of a beat before its anchor word (ms, ~4 frames).
pub const LEAD_MS: f64 = 130.0;
/// Tail after an `until` word (ms).
pub const TAIL_MS: f64 = 250.0;

#[derive(Serialize, Clone, Debug)]
pub struct OutWord {
    pub id: usize,
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResolvedBeat {
    pub idx: usize,
    pub lane: Lane,
    #[serde(flatten)]
    pub kind: BeatKind,
    /// Output window.
    pub start_ms: f64,
    pub end_ms: f64,
    /// Output times at which bullet items appear (one per item).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items_ms: Vec<f64>,
    /// Kept words spoken inside the window (output times), for word-synced reveals.
    #[serde(skip)]
    pub words: Vec<OutWord>,
    pub origin: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResolvedChapter {
    pub n: usize,
    pub title: String,
    pub short: String,
    /// Output time the chapter starts (the card start when cards are inserted).
    pub at_ms: f64,
    /// Card window, if a card is shown.
    pub card: Option<(f64, f64)>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warn,
    Info,
}

#[derive(Serialize, Clone, Debug)]
pub struct Issue {
    pub sev: Severity,
    /// Output time the issue is at (ms), when it has one.
    pub at_ms: Option<f64>,
    pub msg: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Plan {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub output: OutputSpec,
    pub layout: LayoutSpec,
    pub style: BTreeMap<String, String>,
    pub map: TimeMap,
    /// Source ranges removed (merged).
    pub cuts: Vec<(f64, f64)>,
    pub chapters: Vec<ResolvedChapter>,
    pub beats: Vec<ResolvedBeat>,
    #[serde(skip)]
    pub words: Vec<OutWord>,
    pub issues: Vec<Issue>,
}

impl Plan {
    pub fn duration_ms(&self) -> f64 {
        self.map.duration_ms
    }
    pub fn errors(&self) -> usize {
        self.issues.iter().filter(|i| i.sev == Severity::Error).count()
    }
    /// Active chapter at output time `o`.
    pub fn chapter_at(&self, o: f64) -> Option<&ResolvedChapter> {
        self.chapters.iter().rev().find(|c| c.at_ms <= o + 1e-6)
    }
}

/// Turn a word-range cut into a source range, keeping `CUT_PAD_MS` of silence next to the kept
/// neighbours (never a sliver of a removed word).
pub fn word_cut_range(src: &Source, from: usize, to: usize) -> (f64, f64) {
    let w = &src.words;
    let a = &w[from];
    let b = &w[to];
    let start = if from > 0 {
        let prev = &w[from - 1];
        let gap = (a.start_ms - prev.end_ms).max(0.0);
        prev.end_ms + (gap / 2.0).min(CUT_PAD_MS)
    } else {
        (a.start_ms - CUT_PAD_MS).max(0.0)
    };
    let end = if to + 1 < w.len() {
        let next = &w[to + 1];
        let gap = (next.start_ms - b.end_ms).max(0.0);
        next.start_ms - (gap / 2.0).min(CUT_PAD_MS)
    } else {
        (b.end_ms + CUT_PAD_MS).min(src.duration_ms)
    };
    (start, end.max(start))
}

pub fn build(loaded: &Loaded, src: &Source) -> Plan {
    let spec = &loaded.spec;
    let fps = spec.output.fps;
    let height = spec.output.height;
    let width = ((height as f64 * 16.0 / 9.0) / 2.0).round() as u32 * 2;
    let idx = Index::new(src);
    let mut issues = vec![];
    let nw = src.words.len();
    let full = (0usize, nw);

    let scope_of = |o: &crate::spec::Origin, issues: &mut Vec<Issue>| -> (usize, usize) {
        match &o.scope {
            None => full,
            Some([a, b]) => {
                let all = |_: usize| true;
                let ra = anchor::resolve(a, src, &idx, full, &all);
                let rb = anchor::resolve(b, src, &idx, full, &all);
                match (ra, rb) {
                    (Ok(ra), Ok(rb)) => (ra.word.unwrap_or(0), rb.word_end.map(|e| e + 1).unwrap_or(nw)),
                    (Err(e), _) | (_, Err(e)) => {
                        issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: scope: {e}", o.file) });
                        full
                    }
                }
            }
        }
    };

    // ---- cuts (anchors resolve against the full transcript; cuts may remove anything) ----
    let mut cuts = vec![];
    for (i, c) in spec.cuts.iter().enumerate() {
        let o = &loaded.cut_origin[i];
        let all = |_: usize| true;
        let sc = scope_of(o, &mut issues);
        if let Some([a, b]) = c.ms {
            if b > a {
                cuts.push((a, b));
            }
            continue;
        }
        let Some(from) = &c.from else {
            issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: cut #{i} needs `from` or `ms`", o.file) });
            continue;
        };
        let ra = anchor::resolve(from, src, &idx, sc, &all);
        let rb = anchor::resolve(c.to.as_ref().unwrap_or(from), src, &idx, sc, &all);
        match (ra, rb) {
            (Ok(ra), Ok(rb)) => match (ra.word, rb.word_end) {
                (Some(a), Some(b)) if b >= a => cuts.push(word_cut_range(src, a, b)),
                _ => issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: cut {:?}..{:?} must name words in order", o.file, from.0, c.to.as_ref().map(|t| &t.0)) }),
            },
            (Err(e), _) | (_, Err(e)) => issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: cut: {e}", o.file) }),
        }
    }
    let cuts = crate::timemap::merge(cuts);
    let in_cut = |t: f64| {
        let j = cuts.partition_point(|c| c.0 <= t);
        j > 0 && t < cuts[j - 1].1
    };
    let word_kept: Vec<bool> = src.words.iter().map(|w| !in_cut((w.start_ms + w.end_ms) / 2.0)).collect();
    let kept = |i: usize| word_kept.get(i).copied().unwrap_or(false);

    // ---- chapters ----
    let mut ch_src: Vec<(usize, f64, String, String)> = vec![]; // (spec idx, src insert time, title, short)
    for (i, c) in spec.chapters.iter().enumerate() {
        let o = &loaded.chapter_origin[i];
        let sc = scope_of(o, &mut issues);
        match anchor::resolve(&c.at, src, &idx, sc, &kept) {
            Ok(r) => {
                let t = anchor_src(&r, src).unwrap_or(0.0);
                // insert just before the first word, inside the silence before it
                let t_ins = match r.word {
                    Some(w) if w > 0 => {
                        let prev_end = src.words[w - 1].end_ms;
                        (src.words[w].start_ms - 150.0).max((prev_end + src.words[w].start_ms) / 2.0)
                    }
                    Some(w) => (src.words[w].start_ms - 150.0).max(0.0),
                    None => t,
                };
                if let Some(w) = r.word {
                    if !kept(w) {
                        issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: chapter {:?} starts on a cut word w{w}", o.file, c.title) });
                    }
                }
                ch_src.push((i, t_ins, c.title.clone(), c.short.clone().unwrap_or_else(|| c.title.clone())));
            }
            Err(e) => issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{}: chapter {:?}: {e}", o.file, c.title) }),
        }
    }
    ch_src.sort_by(|a, b| a.1.total_cmp(&b.1));
    let inserts: Vec<(f64, f64, usize)> = if spec.output.chapter_cards == CardMode::Insert {
        ch_src.iter().enumerate().map(|(n, c)| (c.1, spec.output.card_ms, n)).collect()
    } else {
        vec![]
    };
    let map = TimeMap::build(src.duration_ms, fps, &cuts, &inserts);

    let chapters: Vec<ResolvedChapter> = ch_src
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let (at_ms, card) = match spec.output.chapter_cards {
                CardMode::Insert => {
                    let gap = map.pieces.iter().find_map(|p| match *p {
                        crate::timemap::Piece::Gap { out0, dur, tag } if tag == n => Some((out0, out0 + dur)),
                        _ => None,
                    });
                    match gap {
                        Some(g) => (g.0, Some(g)),
                        None => (map.to_out(c.1), None),
                    }
                }
                CardMode::Overlay => {
                    let o = map.to_out(c.1);
                    (o, Some((o, o + spec.output.card_ms)))
                }
                CardMode::Off => (map.to_out(c.1), None),
            };
            ResolvedChapter { n: n + 1, title: c.2.clone(), short: c.3.clone(), at_ms, card }
        })
        .collect();

    // ---- words in output time ----
    let words: Vec<OutWord> = src
        .words
        .iter()
        .enumerate()
        .filter(|(i, _)| kept(*i))
        .map(|(i, w)| OutWord { id: i, text: w.text.clone(), start_ms: map.to_out(w.start_ms), end_ms: map.to_out(w.end_ms.min(w.start_ms.max(w.end_ms - 0.001))) })
        .collect();

    // ---- beats ----
    let dur = map.duration_ms;
    let mut beats = vec![];
    for (i, b) in spec.beats.iter().enumerate() {
        let o = &loaded.beat_origin[i];
        let sc = scope_of(o, &mut issues);
        let label = format!("{}: {} at {:?}", o.file, b.kind.name(), b.at.0);
        let ra = match anchor::resolve(&b.at, src, &idx, sc, &kept) {
            Ok(r) => r,
            Err(e) => {
                issues.push(Issue { sev: Severity::Error, at_ms: None, msg: format!("{label}: {e}") });
                continue;
            }
        };
        if let Some(w) = ra.word {
            if !kept(w) {
                issues.push(Issue { sev: Severity::Error, at_ms: Some(map.to_out(src.words[w].start_ms)), msg: format!("{label}: anchor word w{w} {:?} is cut", src.words[w].text) });
            }
        }
        let lead = b.lead.unwrap_or(LEAD_MS);
        let start = (anchor_out(&ra, src, &map, false) - lead).max(0.0);
        let end = match &b.until {
            Some(u) => match anchor::resolve(u, src, &idx, sc, &kept) {
                Ok(ru) => anchor_out(&ru, src, &map, true) + TAIL_MS,
                Err(e) => {
                    issues.push(Issue { sev: Severity::Error, at_ms: Some(start), msg: format!("{label}: until: {e}") });
                    start + b.kind.default_hold_ms()
                }
            },
            None => {
                let hold = match &b.hold {
                    Some(h) => match h.ms() {
                        Ok(v) => v,
                        Err(e) => {
                            issues.push(Issue { sev: Severity::Error, at_ms: Some(start), msg: format!("{label}: {e}") });
                            b.kind.default_hold_ms()
                        }
                    },
                    None => b.kind.default_hold_ms(),
                };
                start + lead + hold
            }
        };
        let end = end.min(dur);
        if end <= start {
            issues.push(Issue { sev: Severity::Error, at_ms: Some(start), msg: format!("{label}: ends before it starts") });
            continue;
        }
        let mut items_ms = vec![];
        if let BeatKind::Bullets { items, .. } = &b.kind {
            let n = items.len().max(1) as f64;
            for (k, it) in items.iter().enumerate() {
                let t = match it {
                    crate::spec::Item::At { at, .. } => match anchor::resolve(at, src, &idx, sc, &kept) {
                        Ok(r) => anchor_out(&r, src, &map, false) - lead,
                        Err(e) => {
                            issues.push(Issue { sev: Severity::Error, at_ms: Some(start), msg: format!("{label}: item {k}: {e}") });
                            start
                        }
                    },
                    _ => start + 350.0 + (end - start - 1500.0).max(0.0).min(700.0 * n) * k as f64 / n,
                };
                items_ms.push(t.clamp(start, end));
            }
        }
        let lo = words.partition_point(|w| w.end_ms <= start);
        let hi = words.partition_point(|w| w.start_ms < end);
        beats.push(ResolvedBeat {
            idx: i,
            lane: b.kind.lane(),
            kind: b.kind.clone(),
            start_ms: start,
            end_ms: end,
            items_ms,
            words: words[lo..hi.max(lo)].to_vec(),
            origin: o.file.clone(),
        });
    }
    beats.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));

    Plan { width, height, fps, output: spec.output.clone(), layout: spec.layout.clone(), style: spec.style.clone(), map, cuts, chapters, beats, words, issues }
}

fn anchor_src(r: &Resolved, src: &Source) -> Option<f64> {
    if let Some(w) = r.word {
        return Some(src.words[w].start_ms);
    }
    r.src_ms
}

/// Output time of an anchor: a word's start (or its last word's end for `until`).
fn anchor_out(r: &Resolved, src: &Source, map: &TimeMap, end: bool) -> f64 {
    let base = if let Some(o) = r.out_ms {
        o
    } else if end {
        match r.word_end.or(r.word) {
            Some(w) => map.to_out(src.words[w].end_ms - 0.001),
            None => map.to_out(r.src_ms.unwrap_or(0.0)),
        }
    } else {
        match r.word {
            Some(w) => map.to_out(src.words[w].start_ms),
            None => map.to_out(r.src_ms.unwrap_or(0.0)),
        }
    };
    base + r.offset_ms
}

/// Words of the source as a compact text block: one line per sentence, prefixed by the id of its
/// first word. Cut words are dropped. `per_line` caps line length.
pub fn brief(src: &Source, plan: Option<&Plan>, lo: usize, hi: usize, per_line: usize) -> String {
    let kept_set: Option<std::collections::HashSet<usize>> = plan.map(|p| p.words.iter().map(|w| w.id).collect());
    let mut out = String::new();
    let mut line = String::new();
    let mut n = 0;
    let mut first = None;
    let hi = hi.min(src.words.len());
    for i in lo..hi {
        if let Some(k) = &kept_set {
            if !k.contains(&i) {
                continue;
            }
        }
        let w = &src.words[i];
        if first.is_none() {
            first = Some(i);
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&w.text);
        n += 1;
        let end = w.text.ends_with(['.', '?', '!']);
        if end || n >= per_line {
            let t = match plan {
                Some(p) => p.map.to_out(w.start_ms),
                None => w.start_ms,
            };
            let _ = t;
            out.push_str(&format!("w{} {}\n", first.unwrap(), line));
            line.clear();
            n = 0;
            first = None;
        }
    }
    if !line.is_empty() {
        out.push_str(&format!("w{} {}\n", first.unwrap(), line));
    }
    out
}

/// "1:02:03.4" / "2:03.4"
pub fn clock(ms: f64) -> String {
    let t = (ms / 100.0).round() / 10.0;
    let h = (t / 3600.0).floor();
    let m = ((t - h * 3600.0) / 60.0).floor();
    let s = t - h * 3600.0 - m * 60.0;
    if h > 0.0 {
        format!("{}:{:02}:{:04.1}", h as u64, m as u64, s)
    } else {
        format!("{}:{:04.1}", m as u64, s)
    }
}

/// Anchor helper for callers that build anchors in code.
pub fn word_anchor(i: usize) -> Anchor {
    Anchor(format!("w{i}"))
}
