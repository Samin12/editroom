//! The transcript as the passes read it: kept words only, one sentence per line, the line's first
//! word id and [talk]/[screen], a `wN` marker every few words, and optional marks for review
//! candidates (`[[r3 ...]]`) and removed silences (`⏸2.4s`).

use cutroom_core::compose::segments;
use cutroom_core::model::Source;
use cutroom_core::resolve::Plan;
use std::collections::HashMap;

/// Kept words of a plan with what the viewer sees at each.
pub struct Ctx<'a> {
    pub src: &'a Source,
    pub plan: &'a Plan,
    /// Kept word ids, in order.
    pub kept: Vec<usize>,
    /// Output start (ms) per kept position.
    pub out_ms: Vec<f64>,
    /// The screen recording is visible at this kept position.
    pub screen: Vec<bool>,
    /// Position in `kept` per source word id (None when cut).
    pub pos: Vec<Option<usize>>,
}

impl<'a> Ctx<'a> {
    pub fn new(src: &'a Source, plan: &'a Plan) -> Ctx<'a> {
        let segs = segments(plan, src);
        let kept: Vec<usize> = plan.words.iter().map(|w| w.id).collect();
        let out_ms: Vec<f64> = plan.words.iter().map(|w| w.start_ms).collect();
        let mut screen = Vec::with_capacity(kept.len());
        let mut j = 0;
        for &o in &out_ms {
            while j + 1 < segs.len() && segs[j].out1 <= o {
                j += 1;
            }
            screen.push(segs.get(j).map(|s| s.out0 <= o + 1e-6 && s.screen.is_some()).unwrap_or(false));
        }
        let mut pos = vec![None; src.words.len()];
        for (p, &id) in kept.iter().enumerate() {
            pos[id] = Some(p);
        }
        Ctx { src, plan, kept, out_ms, screen, pos }
    }

    /// Kept position at or after source word `id`.
    pub fn pos_at_or_after(&self, id: usize) -> usize {
        self.kept.partition_point(|&k| k < id)
    }
}

/// Inline marks for the editorial pass.
#[derive(Default)]
pub struct Marks {
    /// Review item number (1-based) opening before this word id.
    pub open: HashMap<usize, usize>,
    /// Number of review items closing after this word id.
    pub close: HashMap<usize, usize>,
    /// Seconds of removed silence after this word id.
    pub pause: HashMap<usize, f64>,
}

/// Kept positions `[lo, hi)` as text. `every`: a `wN` marker every N words (0: line starts only).
pub fn brief(c: &Ctx, lo: usize, hi: usize, every: usize, marks: Option<&Marks>, width: usize) -> String {
    let mut out = String::new();
    let mut line = String::new();
    let mut n = 0usize;
    let mut since = 0usize;
    let hi = hi.min(c.kept.len());
    for p in lo..hi {
        let id = c.kept[p];
        let tag = if c.screen[p] { "screen" } else { "talk" };
        // a new line when the picture changes under him
        if n > 0 && p > lo && c.screen[p] != c.screen[p - 1] {
            out.push_str(&line);
            out.push('\n');
            line.clear();
            n = 0;
        }
        if n == 0 {
            line.push_str(&format!("w{id} [{tag}]"));
            since = 0;
        } else if every > 0 && since >= every {
            line.push_str(&format!(" w{id}"));
            since = 0;
        }
        if let Some(m) = marks {
            if let Some(k) = m.open.get(&id) {
                line.push_str(&format!(" [[r{k}"));
            }
        }
        let text = &c.src.words[id].text;
        line.push(' ');
        line.push_str(text);
        n += 1;
        since += 1;
        if let Some(m) = marks {
            if let Some(k) = m.close.get(&id) {
                for _ in 0..*k {
                    line.push_str("]]");
                }
            }
            if let Some(s) = m.pause.get(&id) {
                line.push_str(&format!(" ⏸{s:.1}s"));
            }
        }
        if text.ends_with(['.', '?', '!']) || n >= width {
            out.push_str(&line);
            out.push('\n');
            line.clear();
            n = 0;
        }
    }
    if !line.is_empty() {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Removed silences worth showing: a gap of at least `min_ms` between two consecutive source words
/// that are both kept, most of which the plan removed (seconds of the source gap).
pub fn pause_marks(c: &Ctx, min_ms: f64, max_ms: f64) -> HashMap<usize, f64> {
    let mut m = HashMap::new();
    let w = &c.src.words;
    for p in 0..c.kept.len().saturating_sub(1) {
        let (a, b) = (c.kept[p], c.kept[p + 1]);
        if b != a + 1 {
            continue;
        }
        let gap = w[b].start_ms - w[a].end_ms;
        if gap < min_ms || gap > max_ms {
            continue;
        }
        let out_gap = c.plan.map.to_out(w[b].start_ms) - c.plan.map.to_out(w[a].end_ms);
        if out_gap < gap - 500.0 {
            m.insert(a, gap / 1000.0);
        }
    }
    m
}
