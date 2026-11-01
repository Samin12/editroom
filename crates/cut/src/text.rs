//! Transcript preparation: normalized and stemmed tokens, fillers, stop words, takes, boundaries.

use cutroom_core::anchor::norm;
use cutroom_core::{Source, Track};
use std::collections::HashMap;

/// Words that carry little meaning when matching restarts (weighted low).
const STOP: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "so", "to", "of", "in", "on", "at", "for", "with", "from", "by", "as", "is", "are", "was", "were", "be", "been", "being", "it", "it's", "its", "this", "that",
    "that's", "these", "those", "i", "i'm", "i'll", "i've", "i'd", "you", "you're", "you'll", "you've", "your", "we", "we're", "we'll", "our", "they", "they're", "their", "he", "she", "me", "my", "us",
    "them", "do", "does", "did", "have", "has", "had", "can", "will", "would", "should", "could", "just", "like", "okay", "ok", "alright", "right", "now", "then", "there", "there's", "here", "what",
    "which", "who", "how", "if", "not", "no", "yes", "all", "some", "any", "up", "out", "about", "into", "gonna", "wanna", "gotta", "going", "want", "let's", "let", "get", "go", "actually", "really",
    "very", "also", "well", "kind", "thing", "things", "something", "make", "see", "know", "think", "way", "because", "when", "where", "don't", "can't", "is", "am", "one",
];

/// Intensifiers said twice on purpose ("very, very", "really really").
pub const EMPHATIC: &[&str] = &["very", "really", "so", "super", "much", "many", "no", "yeah", "yes", "more", "again", "far", "long", "way", "bit", "never", "always", "too", "ever", "okay", "ok", "right", "now", "go", "come", "here"];

/// Words that open a clause and may stand in front of a restart ("And", "So", "Okay so").
const LEAD: &[&str] = &["and", "so", "but", "okay", "ok", "alright", "now", "well", "because", "then", "or", "also", "right", "yeah", "great", "cool", "oh", "plus"];

pub struct Tok {
    pub norm: String,
    pub stem: u32,
    pub filler: bool,
    /// Filler or no letters/digits at all: ignored by matching.
    pub skip: bool,
    pub stop: bool,
    pub lead: bool,
    pub ends_sentence: bool,
    /// Ends with any punctuation (sentence end or comma).
    pub ends_punct: bool,
    /// Written cut off ("wan-").
    pub cut_off: bool,
    pub take: u32,
    /// Silence before this word (ms); infinite at a take start.
    pub gap_before: f64,
}

pub struct Text {
    pub toks: Vec<Tok>,
    /// Content positions: indices of non-skip words.
    pub ci: Vec<usize>,
    /// Stem ids of the content positions.
    pub seq: Vec<u32>,
    /// Matching weight of each content position (stop words count less).
    pub weight: Vec<f32>,
    /// Source range of each take (by take number), from its clips.
    pub takes: Vec<(f64, f64)>,
}

/// A crude English stemmer: enough to match "agents"/"agent", "messes"/"mess", "editing"/"edit".
pub fn stem(w: &str) -> &str {
    // spoken forms of the same word
    match w {
        "gonna" => return "going",
        "wanna" => return "want",
        "gotta" => return "got",
        "ok" => return "okay",
        "cause" | "cuz" => return "because",
        _ => {}
    }
    let n = w.len();
    if !w.is_ascii() || n <= 3 {
        return w;
    }
    for (suf, min) in [("ing", 6), ("ies", 5), ("ed", 5), ("es", 5), ("s", 4)] {
        if n >= min && w.ends_with(suf) && !w.ends_with("ss") && !w.ends_with("'s") {
            return &w[..n - suf.len()];
        }
    }
    // possessives match their noun ("Claude's" ~ "Claude"); "it's", "that's", "here's" stay apart
    match w.strip_suffix("'s") {
        Some(base) if !STOP.contains(&base) && !STOP.contains(&w) => base,
        _ => w,
    }
}

impl Text {
    pub fn new(src: &Source, fillers: &[String]) -> Text {
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut takes: Vec<(f64, f64)> = vec![];
        for c in &src.clips {
            let t = c.take as usize;
            if takes.len() <= t {
                takes.resize(t + 1, (f64::INFINITY, f64::NEG_INFINITY));
            }
            takes[t].0 = takes[t].0.min(c.start_ms);
            takes[t].1 = takes[t].1.max(c.end_ms());
        }
        let mics: Vec<_> = {
            let mut m: Vec<_> = src.clips.iter().filter(|c| c.track == Track::Mic).collect();
            m.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
            m
        };
        let take_of = |t: f64| -> u32 {
            let j = mics.partition_point(|c| c.start_ms <= t);
            if j > 0 && mics[j - 1].contains(t) {
                return mics[j - 1].take;
            }
            // fall back to any clip
            src.clips.iter().find(|c| c.contains(t)).map(|c| c.take).unwrap_or(u32::MAX)
        };
        let mut toks = Vec::with_capacity(src.words.len());
        let mut prev_end = f64::NEG_INFINITY;
        let mut prev_take = u32::MAX - 1;
        for w in &src.words {
            let n = norm(&w.text);
            let filler = fillers.contains(&n);
            let skip = filler || n.is_empty();
            let s = stem(&n).to_string();
            let next = ids.len() as u32;
            let id = *ids.entry(s).or_insert(next);
            let take = take_of((w.start_ms + w.end_ms) / 2.0);
            let gap_before = if take != prev_take { f64::INFINITY } else { w.start_ms - prev_end };
            let t = w.text.trim_end_matches(['"', '\'', ')', '”', '’']);
            toks.push(Tok {
                stop: STOP.contains(&n.as_str()),
                lead: LEAD.contains(&n.as_str()),
                ends_sentence: t.ends_with(['.', '?', '!']),
                ends_punct: t.ends_with(['.', '?', '!', ',', ';', ':']),
                cut_off: w.text.trim().ends_with(['-', '—']),
                norm: n,
                stem: id,
                filler,
                skip,
                take,
                gap_before,
            });
            prev_end = w.end_ms;
            prev_take = take;
        }
        let ci: Vec<usize> = (0..toks.len()).filter(|&i| !toks[i].skip).collect();
        let seq = ci.iter().map(|&i| toks[i].stem).collect();
        let weight = ci.iter().map(|&i| if toks[i].stop { 0.25 } else { 1.0 }).collect();
        Text { toks, ci, seq, weight, takes }
    }

    /// Content position `x` starts a sentence: first of its take, after a sentence end, after a
    /// filler or after a pause.
    pub fn boundary(&self, x: usize) -> bool {
        let i = self.ci[x];
        if x == 0 || self.toks[i].gap_before >= 350.0 {
            return true;
        }
        let pi = self.ci[x - 1];
        if self.toks[pi].take != self.toks[i].take || self.toks[pi].ends_sentence {
            return true;
        }
        // a filler or a pause between the previous content word and this one
        (pi + 1..i).any(|k| self.toks[k].filler) || self.toks[pi + 1..=i].iter().any(|t| t.gap_before >= 350.0)
    }

    /// Clause start: a boundary, or one or two lead-in words ("and", "so", "okay so") after one.
    pub fn clause_start(&self, x: usize) -> bool {
        let lead = |y: usize| self.toks[self.ci[y]].lead;
        self.boundary(x) || (x >= 1 && lead(x - 1) && self.boundary(x - 1)) || (x >= 2 && lead(x - 1) && lead(x - 2) && self.boundary(x - 2))
    }

    pub fn same_take(&self, a: usize, b: usize) -> bool {
        self.toks[self.ci[a]].take == self.toks[self.ci[b]].take
    }
}

/// Weighted longest common subsequence of `a` and `b` (weights of `a`). Returns the matched
/// weight and, for each position of `a`, whether it was matched.
pub fn wlcs(a: &[u32], wa: &[f32], b: &[u32], dp: &mut Vec<f32>) -> f32 {
    let (m, n) = (a.len(), b.len());
    dp.clear();
    dp.resize((m + 1) * (n + 1), 0.0);
    let w = n + 1;
    for i in 1..=m {
        for j in 1..=n {
            let v = if a[i - 1] == b[j - 1] { dp[(i - 1) * w + j - 1] + wa[i - 1] } else { 0.0 };
            dp[i * w + j] = v.max(dp[(i - 1) * w + j]).max(dp[i * w + j - 1]);
        }
    }
    dp[m * w + n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems() {
        assert_eq!(stem("agents"), "agent");
        assert_eq!(stem("messes"), "mess");
        assert_eq!(stem("editing"), "edit");
        assert_eq!(stem("class"), "class");
        assert_eq!(stem("it's"), "it's");
        assert_eq!(stem("claude's"), "claude");
        assert_eq!(stem("that's"), "that's");
        assert_eq!(stem("there's"), "there's");
        assert_eq!(stem("here's"), "here's");
        assert_eq!(stem("is"), "is");
        assert_eq!(stem("gonna"), stem("going"));
    }

    #[test]
    fn weighted_lcs() {
        let mut dp = vec![];
        let v = wlcs(&[1, 2, 3, 4], &[1.0, 0.25, 1.0, 1.0], &[1, 9, 3, 4, 5], &mut dp);
        assert!((v - 3.0).abs() < 1e-6);
    }
}
