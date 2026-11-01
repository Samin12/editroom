//! cutroom-cut: find what to remove from a raw recording, without an LLM.
//!
//! [`detect`] proposes cuts on the source transcript: restarts and retakes (the LAST attempt is
//! the keeper), stutters and false starts, fillers, long silences and dead air at take edges.
//! Confident proposals go to [`CutReport::auto`]; ambiguous ones to [`CutReport::review`] for an
//! LLM to judge from the compact [`review_md`]. [`to_yaml`] writes the auto cuts as a `cuts:`
//! spec file for cutroom-core.

mod detect;
pub mod text;

pub use detect::{confidence, Feat};

use cutroom_core::resolve::{clock, word_cut_range, CUT_PAD_MS};
use cutroom_core::Source;
use serde::Serialize;
use std::fmt::Write as _;

/// Why a cut is proposed (the `why:` of the spec).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// An abandoned attempt that is said again (a restarted sentence, paragraph or whole take).
    Retake,
    /// A stutter or a cut-off start inside a line ("can be can be", "I I").
    FalseStart,
    /// um, uh, erm.
    Filler,
    /// A long pause inside a take, shrunk.
    Silence,
    /// Nothing said at the start or end of a take.
    DeadAir,
}

impl Why {
    pub fn as_str(self) -> &'static str {
        match self {
            Why::Retake => "retake",
            Why::FalseStart => "false_start",
            Why::Filler => "filler",
            Why::Silence => "silence",
            Why::DeadAir => "dead_air",
        }
    }
    pub const ALL: [Why; 5] = [Why::Retake, Why::FalseStart, Why::Filler, Why::Silence, Why::DeadAir];
}

/// What a proposal removes: words `from..=to` (ids into `Source::words`) or a source ms range.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Span {
    Words(usize, usize),
    Ms(f64, f64),
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Proposal {
    pub span: Span,
    pub why: Why,
    /// 0..1: how sure the detector is that the cut is right.
    pub confidence: f32,
    /// One line for humans.
    pub reason: String,
}

impl Proposal {
    /// Source range removed (word cuts keep `CUT_PAD_MS` next to kept neighbours).
    pub fn range(&self, src: &Source) -> (f64, f64) {
        match self.span {
            Span::Words(a, b) => word_cut_range(src, a, b),
            Span::Ms(a, b) => (a, b),
        }
    }
    fn start_ms(&self, src: &Source) -> f64 {
        match self.span {
            Span::Words(a, _) => src.words[a].start_ms,
            Span::Ms(a, _) => a,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct CutReport {
    /// Confident cuts, in source order.
    pub auto: Vec<Proposal>,
    /// Ambiguous cuts for an LLM (or a human) to decide, in source order.
    pub review: Vec<Proposal>,
}

impl CutReport {
    /// Merged source ranges removed by the auto cuts (plus the review ones if asked).
    pub fn ranges(&self, src: &Source, with_review: bool) -> Vec<(f64, f64)> {
        let mut r: Vec<(f64, f64)> = self.auto.iter().chain(self.review.iter().filter(|_| with_review)).map(|p| p.range(src)).filter(|r| r.1 > r.0).collect();
        r.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out: Vec<(f64, f64)> = vec![];
        for x in r {
            match out.last_mut() {
                Some(l) if x.0 <= l.1 => l.1 = l.1.max(x.1),
                _ => out.push(x),
            }
        }
        out
    }
    pub fn removed_ms(&self, src: &Source, with_review: bool) -> f64 {
        self.ranges(src, with_review).iter().map(|r| r.1 - r.0).sum()
    }
}

#[derive(Clone, Debug)]
pub struct CutOpts {
    /// Pauses longer than this inside a take are shrunk (ms).
    pub silence_min_ms: f64,
    /// Silence kept next to each word when a pause is shrunk (ms).
    pub silence_pad_ms: f64,
    /// Silence kept before a take's first word and after its last (ms).
    pub dead_air_pad_ms: f64,
    /// Normalized filler words (lowercase).
    pub fillers: Vec<String>,
    /// How far back a restart may reach, in words.
    pub window_words: usize,
    /// Confidence from which a proposal is applied without review.
    pub auto_min: f32,
    /// Confidence from which a proposal is worth a review (below: dropped).
    pub review_min: f32,
    /// Words of context on each side in `review_md`.
    pub context_words: usize,
}

impl Default for CutOpts {
    fn default() -> Self {
        CutOpts {
            silence_min_ms: 300.0,
            silence_pad_ms: CUT_PAD_MS,
            dead_air_pad_ms: 250.0,
            fillers: ["um", "uh", "erm", "er", "uhm", "umm", "uhh", "uh-huh"].iter().map(|s| s.to_string()).collect(),
            window_words: 120,
            auto_min: 0.9,
            review_min: 0.4,
            context_words: 15,
        }
    }
}

fn raw(src: &Source, o: &CutOpts) -> Vec<Proposal> {
    let t = text::Text::new(src, &o.fillers);
    let mut props = vec![];
    detect::fillers(&t, src, &mut props);
    detect::stutters(&t, &mut props);
    detect::restarts(&t, o, |p, _| props.push(p));
    detect::abandoned_takes(&t, src, &mut props);
    detect::silences(&t, src, o, &mut props);
    props
}

/// Propose cuts for `src`. Pure Rust, no network; a 3-hour transcript takes well under a second.
pub fn detect(src: &Source, o: &CutOpts) -> CutReport {
    let props = raw(src, o);
    let (auto, review): (Vec<Proposal>, Vec<Proposal>) = props.into_iter().filter(|p| p.confidence >= o.review_min).partition(|p| p.confidence >= o.auto_min);
    let auto = merge(auto);
    let auto_words: Vec<(usize, usize)> = auto.iter().filter_map(|p| if let Span::Words(a, b) = p.span { Some((a, b)) } else { None }).collect();
    let covered = |a: usize, b: usize| {
        let j = auto_words.partition_point(|w| w.0 <= a);
        j > 0 && auto_words[j - 1].1 >= b
    };
    let review: Vec<Proposal> = merge(review).into_iter().filter(|p| !matches!(p.span, Span::Words(a, b) if covered(a, b))).collect();
    let mut r = CutReport { auto, review };
    r.auto.sort_by(|a, b| a.start_ms(src).total_cmp(&b.start_ms(src)));
    r.review.sort_by(|a, b| a.start_ms(src).total_cmp(&b.start_ms(src)));
    r
}

/// Every restart candidate with its features, before bucketing (for calibration).
#[doc(hidden)]
pub fn restart_candidates(src: &Source, o: &CutOpts) -> Vec<(Proposal, Feat)> {
    let t = text::Text::new(src, &o.fillers);
    let mut v = vec![];
    detect::restarts(&t, o, |p, f| v.push((p, f)));
    v
}

/// Union overlapping or touching proposals: word spans with word spans, ms with ms. The merged
/// one keeps the strongest reason and the highest-priority `why` (retake first).
fn merge(mut v: Vec<Proposal>) -> Vec<Proposal> {
    let key = |p: &Proposal| match p.span {
        Span::Words(a, b) => (0, a as f64, b as f64),
        Span::Ms(a, b) => (1, a, b),
    };
    v.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
    });
    let mut out: Vec<Proposal> = vec![];
    for p in v {
        if let Some(l) = out.last_mut() {
            let touch = match (l.span, p.span) {
                (Span::Words(_, lb), Span::Words(a, _)) => a <= lb + 1,
                (Span::Ms(_, lb), Span::Ms(a, _)) => a <= lb + 1.0,
                _ => false,
            };
            if touch {
                l.span = match (l.span, p.span) {
                    (Span::Words(la, lb), Span::Words(_, b)) => Span::Words(la, lb.max(b)),
                    (Span::Ms(la, lb), Span::Ms(_, b)) => Span::Ms(la, lb.max(b)),
                    (s, _) => s,
                };
                if p.why < l.why || (p.why == l.why && p.confidence > l.confidence) {
                    l.why = p.why;
                    l.reason = p.reason;
                }
                l.confidence = l.confidence.max(p.confidence);
                continue;
            }
        }
        out.push(p);
    }
    out
}

fn ms(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r:.1}")
    }
}

/// The auto cuts as a cutroom spec file (`cuts:` only), loadable as an include.
pub fn to_yaml(r: &CutReport) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# cutroom cut: {} automatic cuts; {} more await review (review.md)", r.auto.len(), r.review.len());
    s.push_str("cuts:\n");
    for p in &r.auto {
        match p.span {
            Span::Words(a, b) if a == b => {
                let _ = write!(s, "  - {{from: w{a}, why: {}}}", p.why.as_str());
            }
            Span::Words(a, b) => {
                let _ = write!(s, "  - {{from: w{a}, to: w{b}, why: {}}}", p.why.as_str());
            }
            Span::Ms(a, b) => {
                let _ = write!(s, "  - {{ms: [{}, {}], why: {}}}", ms(a), ms(b), p.why.as_str());
            }
        }
        if matches!(p.why, Why::Retake | Why::FalseStart) {
            let _ = write!(s, "  # {:.2} {}", p.confidence, p.reason.replace('\n', " "));
        }
        s.push('\n');
    }
    s
}

/// Ambiguous candidates only, each as one line with `context_words` of context either side and
/// the proposed cut in `[[ ]]`. Made to be judged by an LLM with as few tokens as possible.
pub fn review_md(r: &CutReport, src: &Source) -> String {
    review_md_with(r, src, CutOpts::default().context_words)
}

pub fn review_md_with(r: &CutReport, src: &Source, ctx: usize) -> String {
    let w = &src.words;
    let mut s = String::new();
    let _ = writeln!(s, "# Cut review: {} candidates. Answer one line each: `r<n> cut`, `r<n> keep`, or `r<n> cut w<a>-w<b>`.", r.review.len());
    s.push_str("# [[...]] is the proposed cut (an earlier attempt / stutter); the last attempt is kept.\n");
    let join = |a: usize, b: usize| w[a..b].iter().map(|x| x.text.as_str()).collect::<Vec<_>>().join(" ");
    for (n, p) in r.review.iter().enumerate() {
        match p.span {
            Span::Words(a, b) => {
                let lo = a.saturating_sub(ctx);
                let hi = (b + 1 + ctx).min(w.len());
                let mid = if b + 1 - a > 36 { format!("{} … {}", join(a, a + 20), join(b + 1 - 12, b + 1)) } else { join(a, b + 1) };
                let _ = writeln!(s, "r{} {} w{a}-w{b}: {} [[{mid}]] {}", n + 1, p.why.as_str(), join(lo, a), join(b + 1, hi));
            }
            Span::Ms(a, b) => {
                let _ = writeln!(s, "r{} {} {}-{}: {}", n + 1, p.why.as_str(), clock(a), clock(b), p.reason);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests;

/// Review items to accept, from "1,3,7-9" or "all" (1-based, as numbered in review.md).
pub fn parse_accept(s: &str, n: usize) -> anyhow::Result<Vec<usize>> {
    if s.trim() == "all" {
        return Ok((0..n).collect());
    }
    let mut out = vec![];
    for part in s.split([',', ' ']).map(str::trim).filter(|p| !p.is_empty()) {
        let part = part.trim_start_matches('r');
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim_start_matches('r').parse::<usize>()?, b.trim_start_matches('r').parse::<usize>()?),
            None => (part.parse::<usize>()?, part.parse::<usize>()?),
        };
        if a == 0 || b > n || a > b {
            anyhow::bail!("review item {part} is out of range 1..{n}");
        }
        out.extend(a - 1..b);
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// `r` with the accepted review items (indices into `review`) moved into `auto`.
pub fn accept(r: &CutReport, idx: &[usize]) -> CutReport {
    let mut auto = r.auto.clone();
    let mut review = vec![];
    for (i, p) in r.review.iter().enumerate() {
        if idx.contains(&i) {
            auto.push(p.clone());
        } else {
            review.push(p.clone());
        }
    }
    auto.sort_by(|a, b| span_start(a).total_cmp(&span_start(b)));
    CutReport { auto, review }
}

fn span_start(p: &Proposal) -> f64 {
    match p.span {
        Span::Words(a, _) => a as f64,
        Span::Ms(a, _) => a,
    }
}

/// One line per category: count and time removed.
pub fn summary(r: &CutReport, src: &Source) -> String {
    let mut s = String::new();
    for why in Why::ALL {
        let ps: Vec<&Proposal> = r.auto.iter().filter(|p| p.why == why).collect();
        let rv = r.review.iter().filter(|p| p.why == why).count();
        if ps.is_empty() && rv == 0 {
            continue;
        }
        let ms: f64 = ps.iter().map(|p| {
            let (a, b) = p.range(src);
            b - a
        }).sum();
        let _ = writeln!(s, "{:<12} {:>5} cuts {:>8}  ({} to review)", why.as_str(), ps.len(), clock(ms), rv);
    }
    let _ = write!(s, "total removed {} of {} (with every review item: {})", clock(r.removed_ms(src, false)), clock(src.duration_ms), clock(r.removed_ms(src, true)));
    s
}
