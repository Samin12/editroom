//! Score the cuts of edit folders against the hand-edited Borumi timeline (same ground truth as
//! `eval_borumi`): word-level precision / recall of whatever `cuts.yaml` + includes remove, split
//! by who made each cut (detector, `by: agent`, `by: human`).
//!
//! ```sh
//! cargo run --release -p cutroom-cut --example eval_work -- --db yc.bmproj <work dir> [<work dir> ...]
//! ```
//! Each work folder must come from the same project as the database (`ingest borumi`, any scenes).

use cutroom_core::resolve::build;
use cutroom_core::spec::{self, Loaded};
use cutroom_core::{Source, Track};
use cutroom_ingest::Project;
use std::collections::BTreeMap;
use std::path::Path;

const DB: &str = "/private/tmp/cutroom-probe/yc.bmproj";

/// Ground truth per word: Some(true) cut, Some(false) kept, None not judged.
fn truth(src: &Source, kept: &BTreeMap<String, Vec<(f64, f64)>>) -> Vec<Option<bool>> {
    let n = src.words.len();
    let mut media_of: Vec<Option<(String, f64)>> = vec![None; n];
    let mut by_media: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, w) in src.words.iter().enumerate() {
        let mid = (w.start_ms + w.end_ms) / 2.0;
        if let Some(c) = src.clip_at(Track::Mic, mid) {
            media_of[i] = Some((c.media.clone(), c.media_at(mid)));
            by_media.entry(c.media.clone()).or_default().push(i);
        }
    }
    let mut t = vec![None; n];
    for (m, idx) in &by_media {
        let Some(k) = kept.get(m) else { continue };
        let flags: Vec<bool> = idx.iter().map(|&i| k.iter().any(|r| r.0 <= media_of[i].as_ref().unwrap().1 && media_of[i].as_ref().unwrap().1 < r.1)).collect();
        let (Some(f), Some(l)) = (flags.iter().position(|x| *x), flags.iter().rposition(|x| *x)) else { continue };
        for j in f..=l {
            t[idx[j]] = Some(!flags[j]);
        }
    }
    t
}

fn removed(loaded: &Loaded, src: &Source, keep: impl Fn(Option<&str>) -> bool) -> Vec<bool> {
    let mut l = loaded.clone();
    let (c, o): (Vec<_>, Vec<_>) = l.spec.cuts.into_iter().zip(l.cut_origin).filter(|(c, _)| keep(c.by.as_deref())).unzip();
    l.spec.cuts = c;
    l.cut_origin = o;
    let plan = build(&l, src);
    let kept: std::collections::HashSet<usize> = plan.words.iter().map(|w| w.id).collect();
    (0..src.words.len()).map(|i| !kept.contains(&i)).collect()
}

fn score(pred: &[bool], t: &[Option<bool>]) -> (usize, usize, usize) {
    let (mut tp, mut fp, mut fn_) = (0, 0, 0);
    for (p, g) in pred.iter().zip(t) {
        match (p, g) {
            (true, Some(true)) => tp += 1,
            (true, Some(false)) => fp += 1,
            (false, Some(true)) => fn_ += 1,
            _ => {}
        }
    }
    (tp, fp, fn_)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut db = DB.to_string();
    let mut dirs = vec![];
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--db" {
            db = args[i + 1].clone();
            i += 2;
        } else {
            dirs.push(args[i].clone());
            i += 1;
        }
    }
    let project = Project::open_db(Path::new(&db), Path::new(""))?;
    let kept = project.mic_kept_ranges();
    println!("| edit | set | words cut | precision | recall | F1 | TP | FP | FN |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|");
    for d in dirs {
        let loaded = spec::load(&Path::new(&d).join("edit.yaml"))?;
        let src = Source::load(&loaded.source_path)?;
        let t = truth(&src, &kept);
        let gt = t.iter().filter(|x| **x == Some(true)).count();
        let judged = t.iter().filter(|x| x.is_some()).count();
        for (name, detector_only) in [("detector", true), ("all", false)] {
            let p = removed(&loaded, &src, |b: Option<&str>| !detector_only || b.is_none());
            let (tp, fp, fn_) = score(&p, &t);
            let pr = tp as f64 / (tp + fp).max(1) as f64;
            let rc = tp as f64 / (tp + fn_).max(1) as f64;
            let f1 = if pr + rc > 0.0 { 2.0 * pr * rc / (pr + rc) } else { 0.0 };
            println!("| {d} | {name} | {} | {pr:.3} | {rc:.3} | {f1:.3} | {tp} | {fp} | {fn_} |", p.iter().filter(|x| **x).count());
        }
        println!("| {d} | ground truth | {gt} of {judged} judged | | | | | | |");
        if std::env::var("EVAL_CUTS").is_ok() {
            // each agent cut: how many of its words the hand edit also removed
            let idx = cutroom_core::anchor::Index::new(&src);
            let all = |_: usize| true;
            let res = |x: &cutroom_core::spec::Anchor| cutroom_core::anchor::resolve(x, &src, &idx, (0, src.words.len()), &all).ok();
            for c in loaded.spec.cuts.iter().filter(|c| c.by.as_deref() == Some("agent")) {
                let Some(from) = &c.from else { continue };
                let (Some(a), Some(b)) = (res(from).and_then(|r| r.word), res(c.to.as_ref().unwrap_or(from)).and_then(|r| r.word_end)) else { continue };
                let n = (a..=b).filter(|&i| t[i] == Some(true)).count();
                let m = (a..=b).filter(|&i| t[i].is_some()).count();
                println!("  agent w{a}-w{b} {:<15} hand edit also cut {n}/{m}", c.why.clone().unwrap_or_default());
            }
        }
    }
    Ok(())
}
