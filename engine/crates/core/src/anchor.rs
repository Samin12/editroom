//! Anchor parsing and resolution against the transcript.

use crate::model::Source;
use crate::spec::Anchor;
use anyhow::{anyhow, bail, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum AnchorKind {
    Word(usize),
    Phrase { words: Vec<String>, nth: Option<usize> },
    SourceSec(f64),
    OutputSec(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub kind: AnchorKind,
    pub offset_ms: f64,
}

/// Lowercase, keep letters/digits/apostrophes only. "Claude's," -> "claude's".
pub fn norm(w: &str) -> String {
    w.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '’')
        .map(|c| if c == '’' { '\'' } else { c })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn parse(a: &Anchor) -> Result<Parsed> {
    let s = a.0.trim();
    // offset suffix: +0.5s / -200ms (only for non-phrase anchors or after a closing quote)
    let (body, offset_ms) = split_offset(s)?;
    let body = body.trim();
    if let Some(rest) = body.strip_prefix('w') {
        if let Ok(n) = rest.parse::<usize>() {
            return Ok(Parsed { kind: AnchorKind::Word(n), offset_ms });
        }
    }
    if let Some(rest) = body.strip_prefix("s:") {
        return Ok(Parsed { kind: AnchorKind::SourceSec(parse_clock(rest)?), offset_ms });
    }
    if let Some(rest) = body.strip_prefix("o:") {
        return Ok(Parsed { kind: AnchorKind::OutputSec(parse_clock(rest)?), offset_ms });
    }
    // phrase, optionally quoted, optionally with #n
    let (phrase, nth) = match body.rfind('#') {
        Some(i) if body[i + 1..].trim().parse::<usize>().is_ok() => (&body[..i], Some(body[i + 1..].trim().parse::<usize>().unwrap())),
        _ => (body, None),
    };
    let phrase = phrase.trim().trim_matches('"').trim_matches('\'');
    let words: Vec<String> = phrase.split_whitespace().map(norm).filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        bail!("empty anchor {:?}", a.0);
    }
    Ok(Parsed { kind: AnchorKind::Phrase { words, nth }, offset_ms })
}

fn split_offset(s: &str) -> Result<(&str, f64)> {
    // find a trailing +x / -x that is a duration
    for (i, c) in s.char_indices().rev() {
        if c == '+' || c == '-' {
            let tail = &s[i + 1..];
            if tail.is_empty() || !tail.chars().next().unwrap().is_ascii_digit() {
                continue;
            }
            if let Ok(ms) = crate::spec::parse_dur_ms(tail) {
                let head = &s[..i];
                // only accept when the head is a complete anchor (w123, quoted phrase, s:, o:)
                let h = head.trim();
                let ok = (h.starts_with('w') && h[1..].parse::<usize>().is_ok()) || h.ends_with('"') || h.starts_with("s:") || h.starts_with("o:") || h.contains('#');
                if ok {
                    return Ok((head, if c == '+' { ms } else { -ms }));
                }
            }
            break;
        }
    }
    Ok((s, 0.0))
}

/// "754.2", "12:34.5", "1:02:03"
pub fn parse_clock(t: &str) -> Result<f64> {
    let parts: Vec<&str> = t.trim().split(':').collect();
    let mut secs = 0.0;
    for p in &parts {
        secs = secs * 60.0 + p.trim().parse::<f64>().map_err(|_| anyhow!("bad time {t:?}"))?;
    }
    Ok(secs)
}

/// Normalized transcript for fast phrase search.
pub struct Index {
    pub norm: Vec<String>,
}

impl Index {
    pub fn new(src: &Source) -> Index {
        Index { norm: src.words.iter().map(|w| norm(&w.text)).collect() }
    }

    /// All word indices where `phrase` starts (contiguous match, punctuation-insensitive;
    /// a phrase word also matches a transcript word that splits on '-').
    pub fn find(&self, phrase: &[String], lo: usize, hi: usize) -> Vec<usize> {
        let hi = hi.min(self.norm.len());
        let mut out = vec![];
        if phrase.is_empty() || lo >= hi {
            return out;
        }
        let mut i = lo;
        while i < hi {
            if self.norm[i] == phrase[0]
                && (0..phrase.len()).all(|k| i + k < self.norm.len() && self.norm[i + k] == phrase[k]) {
                    out.push(i);
                }
            i += 1;
        }
        out
    }

    /// Closest matches for an error message: positions sharing the most phrase words in a window.
    pub fn suggest(&self, phrase: &[String], lo: usize, hi: usize, src: &Source) -> String {
        let hi = hi.min(self.norm.len());
        let n = phrase.len().max(1);
        let mut best: Vec<(usize, usize)> = vec![];
        for i in lo..hi.saturating_sub(n.saturating_sub(1)) {
            let score = (0..n).filter(|&k| i + k < hi && phrase.contains(&self.norm[i + k])).count();
            if score * 2 >= n && score > 0 {
                best.push((score, i));
            }
        }
        best.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        best.dedup_by(|a, b| a.1.abs_diff(b.1) < n);
        best.iter()
            .take(3)
            .map(|&(_, i)| format!("w{i} \"{}\"", src.words[i..(i + n).min(src.words.len())].iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A resolved anchor: a word index (when the anchor named a word) and a source time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    pub word: Option<usize>,
    /// Last word of a phrase (for `until`).
    pub word_end: Option<usize>,
    pub src_ms: Option<f64>,
    pub out_ms: Option<f64>,
    pub offset_ms: f64,
}

/// Resolve `a` against the transcript. `kept` tells whether a word survives the cuts: phrase
/// matches prefer kept words. `scope` limits phrase search to word indices `[lo, hi)`.
pub fn resolve(a: &Anchor, src: &Source, idx: &Index, scope: (usize, usize), kept: &dyn Fn(usize) -> bool) -> Result<Resolved> {
    let p = parse(a)?;
    let r = Resolved { word: None, word_end: None, src_ms: None, out_ms: None, offset_ms: p.offset_ms };
    match p.kind {
        AnchorKind::Word(n) => {
            if n >= src.words.len() {
                bail!("w{n} does not exist (the transcript has {} words)", src.words.len());
            }
            Ok(Resolved { word: Some(n), word_end: Some(n), ..r })
        }
        AnchorKind::SourceSec(s) => Ok(Resolved { src_ms: Some(s * 1000.0), ..r }),
        AnchorKind::OutputSec(s) => Ok(Resolved { out_ms: Some(s * 1000.0), ..r }),
        AnchorKind::Phrase { words, nth } => {
            let all = idx.find(&words, scope.0, scope.1);
            let live: Vec<usize> = all.iter().copied().filter(|&i| (0..words.len()).all(|k| kept(i + k))).collect();
            let pool = if live.is_empty() { &all } else { &live };
            let pick = match (pool.len(), nth) {
                (0, _) => {
                    let sug = idx.suggest(&words, scope.0, scope.1, src);
                    bail!("phrase {:?} not found{}", a.0, if sug.is_empty() { String::new() } else { format!("; closest: {sug}") })
                }
                (_, Some(k)) if k >= 1 && k <= pool.len() => pool[k - 1],
                (_, Some(k)) => bail!("phrase {:?}: asked for match #{k} but there are {}", a.0, pool.len()),
                (1, None) => pool[0],
                (n, None) => bail!(
                    "phrase {:?} matches {n} places ({}); add #k or use a word id",
                    a.0,
                    pool.iter().take(5).map(|i| format!("w{i}")).collect::<Vec<_>>().join(", ")
                ),
            };
            Ok(Resolved { word: Some(pick), word_end: Some(pick + words.len() - 1), ..r })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Word;

    fn src(text: &str) -> Source {
        let words = text
            .split_whitespace()
            .enumerate()
            .map(|(i, t)| Word { text: t.into(), start_ms: i as f64 * 300.0, end_ms: i as f64 * 300.0 + 250.0 })
            .collect();
        Source { name: "t".into(), duration_ms: 100_000.0, words, ..Default::default() }
    }

    #[test]
    fn parses() {
        assert_eq!(parse(&Anchor("w12".into())).unwrap().kind, AnchorKind::Word(12));
        let p = parse(&Anchor("w12+0.5s".into())).unwrap();
        assert_eq!((p.kind, p.offset_ms), (AnchorKind::Word(12), 500.0));
        let p = parse(&Anchor("\"Connect Claude\"#2".into())).unwrap();
        assert_eq!(p.kind, AnchorKind::Phrase { words: vec!["connect".into(), "claude".into()], nth: Some(2) });
        let p = parse(&Anchor("s:1:02.5".into())).unwrap();
        assert_eq!(p.kind, AnchorKind::SourceSec(62.5));
        // a hyphenated phrase is not an offset
        let p = parse(&Anchor("pre-embedded knowledge".into())).unwrap();
        assert_eq!(p.kind, AnchorKind::Phrase { words: vec!["preembedded".into(), "knowledge".into()], nth: None });
    }

    #[test]
    fn phrases() {
        let s = src("So now we connect Claude to Poppy. Then we connect Claude to Notion.");
        let idx = Index::new(&s);
        let all = |_: usize| true;
        let r = resolve(&Anchor("connect claude to poppy".into()), &s, &idx, (0, 99), &all).unwrap();
        assert_eq!((r.word, r.word_end), (Some(3), Some(6)));
        assert!(resolve(&Anchor("connect claude".into()), &s, &idx, (0, 99), &all).unwrap_err().to_string().contains("matches 2"));
        let r = resolve(&Anchor("connect claude#2".into()), &s, &idx, (0, 99), &all).unwrap();
        assert_eq!(r.word, Some(9));
        // a cut word is skipped when another match is kept
        let not3 = |i: usize| i != 3;
        let r = resolve(&Anchor("connect claude".into()), &s, &idx, (0, 99), &not3).unwrap();
        assert_eq!(r.word, Some(9));
        let e = resolve(&Anchor("connect clod to poppy".into()), &s, &idx, (0, 99), &all).unwrap_err().to_string();
        assert!(e.contains("closest: w3"), "{e}");
    }
}
