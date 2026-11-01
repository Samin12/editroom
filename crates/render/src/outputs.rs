//! Side outputs: YouTube chapters and captions from the same plan as the picture.

use cutroom_core::resolve::OutWord;
use cutroom_core::Plan;

fn yt_clock(ms: f64) -> String {
    let s = (ms / 1000.0).floor() as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// YouTube description chapters ("0:00 Intro" first; times land on the cards).
pub fn chapters(plan: &Plan) -> String {
    let mut out = String::new();
    if plan.chapters.first().map(|c| c.at_ms > 1000.0).unwrap_or(true) {
        out.push_str("0:00 Intro\n");
    }
    for (i, c) in plan.chapters.iter().enumerate() {
        let t = if i == 0 && c.at_ms <= 1000.0 { 0.0 } else { c.at_ms };
        out.push_str(&format!("{} {}\n", yt_clock(t), c.title));
    }
    out
}

fn srt_clock(ms: f64) -> String {
    let ms = ms.max(0.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, (ms / 60_000) % 60, (ms / 1000) % 60, ms % 1000)
}

/// Caption cues: up to ~42 characters, broken at sentence ends and pauses.
pub fn cues(words: &[OutWord]) -> Vec<(f64, f64, String)> {
    let mut out: Vec<(f64, f64, String)> = vec![];
    let mut cur: Vec<&OutWord> = vec![];
    let flush = |cur: &mut Vec<&OutWord>, out: &mut Vec<(f64, f64, String)>| {
        if let (Some(a), Some(b)) = (cur.first(), cur.last()) {
            let text = cur.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
            out.push((a.start_ms, b.end_ms.max(a.start_ms + 700.0), text));
        }
        cur.clear();
    };
    for w in words {
        if let Some(last) = cur.last() {
            let len: usize = cur.iter().map(|x| x.text.len() + 1).sum::<usize>() + w.text.len();
            let pause = w.start_ms - last.end_ms > 600.0;
            let sentence = last.text.ends_with(['.', '?', '!']);
            if len > 42 || pause || sentence {
                flush(&mut cur, &mut out);
            }
        }
        cur.push(w);
    }
    flush(&mut cur, &mut out);
    // no overlaps
    for i in 1..out.len() {
        if out[i - 1].1 > out[i].0 {
            out[i - 1].1 = out[i].0;
        }
    }
    out
}

pub fn srt(plan: &Plan) -> String {
    let mut s = String::new();
    for (i, (a, b, t)) in cues(&plan.words).iter().enumerate() {
        s.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, srt_clock(*a), srt_clock(*b), t));
    }
    s
}
