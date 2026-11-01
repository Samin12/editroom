//! Ground-truth evaluation of `cutroom_cut::detect` on Samin's course project.
//!
//! The project was cut by a multi-agent LLM workflow plus manual edits. For every narration mic
//! recording on the CURRENT Borumi timeline, its mic segments say which media-time intervals
//! were kept; every word between the first and last kept word that is not kept was cut. This
//! example ingests the project in Raw mode (full recordings), runs `detect`, and scores it.
//!
//! ```sh
//! cargo run --release -p cutroom-cut --example eval_borumi -- [--db yc.bmproj] [--medias dir] [--write]
//! ```
//! `--write` saves the report to `crates/cut/EVAL.md`. `CUTROOM_EVAL_DUMP=x.tsv` also dumps every
//! restart candidate with its features and ground-truth label (for calibration).

use cutroom_core::{Source, Track};
use cutroom_cut::{detect, restart_candidates, review_md, to_yaml, CutOpts, CutReport, Proposal, Span, Why};
use cutroom_ingest::{BorumiMode, Project};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

const DB: &str = "/private/tmp/cutroom-probe/yc.bmproj";
/// Fallback media folder: `$CUTROOM_EVAL_MEDIAS` (the bundle's `medias/`).
const MEDIAS: &str = "";

/// Ground truth per word: `Some(true)` cut, `Some(false)` kept, `None` not judged.
struct Truth {
    word: Vec<Option<bool>>,
    off_timeline: usize,
    out_of_scope: usize,
    regions: Vec<Region>,
}

/// A judged recording, in media ms.
struct Region {
    /// Source time of media time 0.
    off: f64,
    /// First kept word start .. last kept word end.
    span: (f64, f64),
    kept: Vec<(f64, f64)>,
}

fn truth(src: &Source, kept: &BTreeMap<String, Vec<(f64, f64)>>) -> Truth {
    let n = src.words.len();
    let mut media_of: Vec<Option<(String, f64, f64)>> = vec![None; n]; // (media, media time, source offset)
    let mut by_media: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, w) in src.words.iter().enumerate() {
        let mid = (w.start_ms + w.end_ms) / 2.0;
        if let Some(c) = src.clip_at(Track::Mic, mid) {
            media_of[i] = Some((c.media.clone(), c.media_at(mid), c.start_ms - c.media_in_ms));
            by_media.entry(c.media.clone()).or_default().push(i);
        }
    }
    let mut t = Truth { word: vec![None; n], off_timeline: 0, out_of_scope: 0, regions: vec![] };
    for (m, idx) in &by_media {
        let Some(k) = kept.get(m) else {
            t.off_timeline += idx.len();
            continue;
        };
        let is_kept = |i: usize| {
            let mt = media_of[i].as_ref().unwrap().1;
            k.iter().any(|r| r.0 <= mt && mt < r.1)
        };
        let flags: Vec<bool> = idx.iter().map(|&i| is_kept(i)).collect();
        let (Some(f), Some(l)) = (flags.iter().position(|x| *x), flags.iter().rposition(|x| *x)) else {
            t.off_timeline += idx.len();
            continue;
        };
        t.out_of_scope += idx.len() - (l - f + 1);
        for j in f..=l {
            t.word[idx[j]] = Some(!flags[j]);
        }
        let off = media_of[idx[f]].as_ref().unwrap().2;
        let span = (src.words[idx[f]].start_ms - off, src.words[idx[l]].end_ms - off);
        t.regions.push(Region { off, span, kept: k.clone() });
    }
    t
}

fn covered(ranges: &[(f64, f64)], t: f64) -> bool {
    let j = ranges.partition_point(|r| r.0 <= t);
    j > 0 && t < ranges[j - 1].1
}

fn merged(mut r: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    r.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = vec![];
    for x in r.into_iter().filter(|x| x.1 > x.0) {
        match out.last_mut() {
            Some(l) if x.0 <= l.1 => l.1 = l.1.max(x.1),
            _ => out.push(x),
        }
    }
    out
}

fn ranges_of<'a>(src: &Source, ps: impl Iterator<Item = &'a Proposal>) -> Vec<(f64, f64)> {
    merged(ps.map(|p| p.range(src)).collect())
}

fn predicted(src: &Source, ranges: &[(f64, f64)]) -> Vec<bool> {
    src.words.iter().map(|w| covered(ranges, (w.start_ms + w.end_ms) / 2.0)).collect()
}

#[derive(Default, Clone, Copy)]
struct Prf {
    tp: usize,
    fp: usize,
    fn_: usize,
}

impl Prf {
    fn new(pred: &[bool], t: &Truth) -> Prf {
        let mut s = Prf::default();
        for (p, g) in pred.iter().zip(&t.word) {
            match (p, g) {
                (true, Some(true)) => s.tp += 1,
                (true, Some(false)) => s.fp += 1,
                (false, Some(true)) => s.fn_ += 1,
                _ => {}
            }
        }
        s
    }
    fn p(&self) -> f64 {
        self.tp as f64 / (self.tp + self.fp).max(1) as f64
    }
    fn r(&self) -> f64 {
        self.tp as f64 / (self.tp + self.fn_).max(1) as f64
    }
    fn f1(&self) -> f64 {
        let (p, r) = (self.p(), self.r());
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

fn len(r: &[(f64, f64)]) -> f64 {
    r.iter().map(|x| x.1 - x.0).sum()
}

fn intersect(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let (mut i, mut j, mut out) = (0, 0, vec![]);
    while i < a.len() && j < b.len() {
        let lo = a[i].0.max(b[j].0);
        let hi = a[i].1.min(b[j].1);
        if hi > lo {
            out.push((lo, hi));
        }
        if a[i].1 < b[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Time-level comparison inside the judged spans: (ours, ground truth, both) in ms.
fn time_cmp(t: &Truth, ours: &[(f64, f64)]) -> (f64, f64, f64) {
    let (mut o, mut g, mut b) = (0.0, 0.0, 0.0);
    for Region { off, span, kept } in &t.regions {
        let span = vec![*span];
        let ours_m: Vec<(f64, f64)> = merged(ours.iter().map(|r| (r.0 - off, r.1 - off)).collect());
        let ours_m = intersect(&ours_m, &span);
        // ground truth cut = span minus kept
        let mut gt = vec![];
        let mut cur = span[0].0;
        for k in intersect(kept, &span) {
            if k.0 > cur {
                gt.push((cur, k.0));
            }
            cur = cur.max(k.1);
        }
        if span[0].1 > cur {
            gt.push((cur, span[0].1));
        }
        o += len(&ours_m);
        g += len(&gt);
        b += len(&intersect(&ours_m, &gt));
    }
    (o, g, b)
}

fn mins(ms: f64) -> String {
    format!("{:.1} min", ms / 60_000.0)
}

fn snippet(src: &Source, a: usize, b: usize) -> String {
    let w = &src.words;
    let j = |x: usize, y: usize| w[x..y].iter().map(|x| x.text.as_str()).collect::<Vec<_>>().join(" ");
    let lo = a.saturating_sub(6);
    let hi = (b + 7).min(w.len());
    let mid = if b + 1 - a > 22 { format!("{} … {}", j(a, a + 14), j(b - 5, b + 1)) } else { j(a, b + 1) };
    format!("…{} [[{}]] {}…", j(lo, a), mid, j(b + 1, hi)).replace('|', "/")
}

fn spread<T: Clone>(v: &[T], n: usize) -> Vec<T> {
    if v.len() <= n {
        return v.to_vec();
    }
    (0..n).map(|i| v[i * v.len() / n].clone()).collect()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let db = arg("--db").unwrap_or_else(|| DB.into());
    let medias = arg("--medias").or_else(|| std::env::var("CUTROOM_EVAL_MEDIAS").ok()).unwrap_or_else(|| MEDIAS.into());
    let write = args.iter().any(|a| a == "--write");

    let t0 = Instant::now();
    let project = Project::open_db(Path::new(&db), Path::new(&medias))?;
    let src = project.to_source(BorumiMode::Raw, false)?;
    let ingest = t0.elapsed();
    let kept = project.mic_kept_ranges();
    let gt = truth(&src, &kept);
    let opts = CutOpts::default();

    let mut runs: Vec<Duration> = vec![];
    let mut report = CutReport::default();
    for _ in 0..7 {
        let t = Instant::now();
        report = detect(&src, &opts);
        runs.push(t.elapsed());
    }
    runs.sort();
    let t = Instant::now();
    let md = review_md(&report, &src);
    let yaml = to_yaml(&report);
    let render = t.elapsed();

    let judged = gt.word.iter().filter(|x| x.is_some()).count();
    let gt_cut = gt.word.iter().filter(|x| **x == Some(true)).count();
    let auto_r = ranges_of(&src, report.auto.iter());
    let all_r = ranges_of(&src, report.auto.iter().chain(&report.review));
    let auto = Prf::new(&predicted(&src, &auto_r), &gt);
    let all = Prf::new(&predicted(&src, &all_r), &gt);

    let mut s = String::new();
    let _ = writeln!(s, "# cutroom-cut evaluation (Samin's YouTube course)\n");
    let _ = writeln!(s, "Generated by `cargo run --release -p cutroom-cut --example eval_borumi -- --write`.\n");
    let _ = writeln!(s, "Ground truth: the current Borumi timeline of the course project (cut by a multi-agent LLM workflow plus manual edits). A raw word counts as *cut* when its narration recording is on the timeline but the word is not inside a kept mic segment. Some of those cuts were editorial (content dropped for the narrative), so recall cannot reach 100%.\n");
    let _ = writeln!(s, "## Data\n");
    let _ = writeln!(s, "- Raw source: {:.2} h, {} words, {} narration takes in {} scenes (ingest {:.0} ms)", src.duration_ms / 3.6e6, src.words.len(), src.clips.iter().map(|c| c.take).max().map(|x| x as usize + 1).unwrap_or(0), src.scenes.len(), ingest.as_secs_f64() * 1000.0);
    let _ = writeln!(s, "- Judged words: {judged} ({gt_cut} cut by the ground truth, {:.1}%)", 100.0 * gt_cut as f64 / judged.max(1) as f64);
    let _ = writeln!(s, "- Not judged: {} words in recordings that are not on the timeline at all, {} words before the first / after the last kept word of a recording\n", gt.off_timeline, gt.out_of_scope);

    let _ = writeln!(s, "## Word-level cut detection\n");
    let _ = writeln!(s, "| set | proposals | precision | recall | F1 | TP | FP | FN |");
    let _ = writeln!(s, "|---|---:|---:|---:|---:|---:|---:|---:|");
    for (name, n, m) in [("auto", report.auto.len(), auto), ("auto + review", report.auto.len() + report.review.len(), all)] {
        let _ = writeln!(s, "| {name} | {n} | {:.3} | {:.3} | {:.3} | {} | {} | {} |", m.p(), m.r(), m.f1(), m.tp, m.fp, m.fn_);
    }
    // split halves: the score was hand-tuned on this project, so check both halves agree
    let half = src.words.len() / 2;
    let pa = predicted(&src, &auto_r);
    let mask = |lo: usize, hi: usize| -> Truth { Truth { word: gt.word.iter().enumerate().map(|(i, g)| if (lo..hi).contains(&i) { *g } else { None }).collect(), off_timeline: 0, out_of_scope: 0, regions: vec![] } };
    let (h1, h2) = (Prf::new(&pa, &mask(0, half)), Prf::new(&pa, &mask(half, src.words.len())));
    let _ = writeln!(s, "\nSplit halves (first / second half of the recording): auto precision {:.3} / {:.3}, recall {:.3} / {:.3}.", h1.p(), h2.p(), h1.r(), h2.r());
    // identical-copy swaps: an auto retake the ground truth kept while it cut the restart right
    // after it instead (the same words, the other copy kept)
    let mut swapped = 0;
    for p in report.auto.iter().filter(|p| p.why == Why::Retake) {
        let Span::Words(a, b) = p.span else { continue };
        let ours: Vec<bool> = (a..=b).filter_map(|k| gt.word[k]).collect();
        let kept_here = ours.iter().filter(|x| !**x).count();
        if ours.is_empty() || kept_here * 2 < ours.len() {
            continue;
        }
        let next: Vec<bool> = (b + 1..(2 * b + 2 - a).min(src.words.len())).filter_map(|k| gt.word[k]).collect();
        if !next.is_empty() && next.iter().filter(|x| **x).count() * 10 >= next.len() * 6 {
            swapped += kept_here;
        }
    }
    let _ = writeln!(
        s,
        "\nSwap-aware: {swapped} of the auto false-positive words are identical-copy swaps (the ground truth kept the first attempt and cut the restart right after it; the same edit with the other copy kept). Counting those as correct, auto precision is {:.3}.",
        (auto.tp + swapped) as f64 / (auto.tp + auto.fp).max(1) as f64
    );
    let _ = writeln!(s, "\nIf every review item is accepted the result is the *auto + review* row; an LLM judging the review queue lands between the two rows.\n");
    let _ = writeln!(s, "### Per category\n");
    let _ = writeln!(s, "Precision of the words each category removes; *share of GT* = ground-truth cut words it catches.\n");
    let _ = writeln!(s, "| category | auto cuts | auto words | auto precision | share of GT | review cuts | review words | review precision |");
    let _ = writeln!(s, "|---|---:|---:|---:|---:|---:|---:|---:|");
    for why in Why::ALL {
        let a: Vec<&Proposal> = report.auto.iter().filter(|p| p.why == why).collect();
        let r: Vec<&Proposal> = report.review.iter().filter(|p| p.why == why).collect();
        let pa = Prf::new(&predicted(&src, &ranges_of(&src, a.iter().copied())), &gt);
        let pr = Prf::new(&predicted(&src, &ranges_of(&src, r.iter().copied())), &gt);
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {:.3} | {} | {} | {} |",
            why.as_str(),
            a.len(),
            pa.tp + pa.fp,
            if pa.tp + pa.fp > 0 { format!("{:.3}", pa.p()) } else { "–".into() },
            pa.tp as f64 / gt_cut.max(1) as f64,
            r.len(),
            pr.tp + pr.fp,
            if pr.tp + pr.fp > 0 { format!("{:.3}", pr.p()) } else { "–".into() }
        );
    }

    let _ = writeln!(s, "\n## Time removed (inside the judged spans)\n");
    let (ga, gg, gb) = time_cmp(&gt, &auto_r);
    let (aa, _, ab) = time_cmp(&gt, &all_r);
    let sil: Vec<&Proposal> = report.auto.iter().filter(|p| matches!(p.why, Why::Silence)).collect();
    let (sa, _, sb) = time_cmp(&gt, &ranges_of(&src, sil.iter().copied()));
    let _ = writeln!(s, "| | removed | of which the ground truth also removed |");
    let _ = writeln!(s, "|---|---:|---:|");
    let _ = writeln!(s, "| ground truth | {} | – |", mins(gg));
    let _ = writeln!(s, "| auto | {} | {} ({:.0}%) |", mins(ga), mins(gb), 100.0 * gb / ga.max(1.0));
    let _ = writeln!(s, "| auto + review | {} | {} ({:.0}%) |", mins(aa), mins(ab), 100.0 * ab / aa.max(1.0));
    let _ = writeln!(s, "| silence cuts alone | {} | {} ({:.0}%) |", mins(sa), mins(sb), 100.0 * sb / sa.max(1.0));
    let _ = writeln!(s, "\nWhole raw source: auto removes {} of {} ({:.0}%), auto + review {}.", mins(report.removed_ms(&src, false)), mins(src.duration_ms), 100.0 * report.removed_ms(&src, false) / src.duration_ms, mins(report.removed_ms(&src, true)));

    let abandoned: Vec<&Proposal> = report.auto.iter().chain(&report.review).filter(|p| p.reason.starts_with("take re-said")).collect();
    let _ = writeln!(s, "\nAbandoned whole takes proposed: {} ({} of their words are on the timeline).", abandoned.len(), abandoned.iter().map(|p| if let Span::Words(a, b) = p.span { (a..=b).filter(|&i| gt.word[i].is_some()).count() } else { 0 }).sum::<usize>());

    let _ = writeln!(s, "\n## Speed and review cost\n");
    let _ = writeln!(s, "- `detect` on {:.1} h / {} words: {:.1} ms (best of 7), {:.1} ms median", src.duration_ms / 3.6e6, src.words.len(), runs[0].as_secs_f64() * 1000.0, runs[3].as_secs_f64() * 1000.0);
    let _ = writeln!(s, "- `review_md` + `to_yaml`: {:.1} ms", render.as_secs_f64() * 1000.0);
    let _ = writeln!(s, "- Review queue: {} items, {} bytes (~{} tokens)", report.review.len(), md.len(), md.len() / 4);
    let _ = writeln!(s, "- cuts.yaml: {} auto cuts, {} bytes", report.auto.len(), yaml.len());

    // ---- misses: ground-truth cut runs nothing proposed touches ----
    let pred_all = predicted(&src, &all_r);
    let mut runs_gt: Vec<(usize, usize)> = vec![];
    let mut i = 0;
    while i < gt.word.len() {
        if gt.word[i] == Some(true) {
            let mut j = i;
            while j + 1 < gt.word.len() && gt.word[j + 1] == Some(true) {
                j += 1;
            }
            if (i..=j).all(|k| !pred_all[k]) && j - i >= 2 {
                runs_gt.push((i, j));
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    runs_gt.sort_by_key(|r| std::cmp::Reverse(r.1 - r.0));
    let _ = writeln!(s, "\n## 10 misses (ground-truth cuts nothing proposed; {} such runs of 3+ words)\n", runs_gt.len());
    for (a, b) in spread(&runs_gt, 10) {
        let _ = writeln!(s, "- w{a} ({} words): {}", b + 1 - a, snippet(&src, a, b));
    }
    // ---- false positives: auto cuts the ground truth kept ----
    let mut fps: Vec<(usize, usize, Why, f32)> = report
        .auto
        .iter()
        .filter_map(|p| match p.span {
            Span::Words(a, b) => {
                let judged: Vec<bool> = (a..=b).filter_map(|k| gt.word[k]).collect();
                let cut = judged.iter().filter(|x| **x).count();
                (!judged.is_empty() && cut * 2 < judged.len()).then_some((a, b, p.why, p.confidence))
            }
            _ => None,
        })
        .collect();
    fps.sort_by_key(|r| std::cmp::Reverse(r.1 - r.0));
    let _ = writeln!(s, "\n## 10 false positives (auto cuts the ground truth kept; {} such cuts)\n", fps.len());
    for (a, b, why, c) in spread(&fps, 10) {
        let _ = writeln!(s, "- w{a} {} {:.2}: {}", why.as_str(), c, snippet(&src, a, b));
    }

    s.push_str(
        "\n## How detect works\n\n\
- **Restarts / retakes**: for every word, earlier positions within 120 words that share its bigram (or a rare word) are restart candidates. A pair is shifted back while the word before the attempt equals its last word (the same edit, aligned on the true restart, keeping the later copy). Score = weighted share of the abandoned attempt that is said again (stemmed, stop words weigh 0.25) + whether both attempts start a clause + exact-prefix length − sentences inside the attempt. The cut runs from the attempt's clause start to just before the restart, keeping the restart's lead-in (\"And\", \"So\"). Chains of attempts merge, so only the last one stays.\n\
- **Stutters**: immediate repeats of 2-4 words (auto unless the first copy is a whole sentence); single function-word repeats go to review; emphasis (\"very, very\"), sentence joins (\"this. This is\") and \"bit by bit\" are left alone.\n\
- **Fillers**: um / uh / erm (not \"like\" / \"so\"); tokens under 60 ms are recognition artefacts and skipped.\n\
- **Silences**: pauses over 300 ms inside a take keep 120 ms next to each word; dead air before a take's first and after its last word keeps 250 ms. Never inside a word.\n\
- **Abandoned takes**: a take whose word trigrams are mostly re-said by one of the next three takes, unless the matching words sit at the same moments (multi-camera takes of one performance).\n\n\
## Known gaps\n\n\
- About 30% of the ground-truth cut words fall in no candidate at all: editorial cuts (a sentence dropped for the story) and abandoned clauses that are never re-said. No repetition signal exists for those; they need an LLM pass over the script, not this detector.\n\
- Paraphrased restarts that share no bigram with the attempt are missed; restarts whose first copy the ground truth preferred count as false positives above (see swap-aware precision).\n\
- Parakeet word timings run edge to edge, so many pauses hide inside word durations; silence detection only sees gaps between words.\n\
- The scoring was hand-tuned on this one project (the split-half numbers agree); expect to recalibrate on other speakers.\n",
    );
    print!("{s}");
    if write {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("EVAL.md");
        std::fs::write(&p, &s)?;
        eprintln!("wrote {}", p.display());
    }
    if let Ok(path) = std::env::var("CUTROOM_EVAL_DUMP") {
        let mut d = String::from("from\tto\tconf\tlen\tprefix\tsim\tcover\tinfo\trestart_clause\tattempt_clause\tincomplete\tcross\tpause\tinner_ends\tgt_frac\tr_frac\tjudged\tsnippet\n");
        for (p, f) in restart_candidates(&src, &opts) {
            let Span::Words(a, b) = p.span else { continue };
            let judged: Vec<bool> = (a..=b).filter_map(|k| gt.word[k]).collect();
            let frac = judged.iter().filter(|x| **x).count() as f64 / judged.len().max(1) as f64;
            // the restart: as many words after the cut as it removes
            let rj: Vec<bool> = (b + 1..(2 * b + 2 - a).min(src.words.len())).filter_map(|k| gt.word[k]).collect();
            let rfrac = rj.iter().filter(|x| **x).count() as f64 / rj.len().max(1) as f64;
            let _ = writeln!(
                d,
                "{a}\t{b}\t{:.3}\t{}\t{}\t{:.3}\t{:.3}\t{:.2}\t{}\t{}\t{}\t{}\t{:.0}\t{}\t{:.3}\t{:.3}\t{}\t{}",
                p.confidence, f.len, f.prefix, f.sim, f.cover, f.info, f.restart_at_clause as u8, f.attempt_at_clause as u8, f.incomplete as u8, f.cross_take as u8, f.pause_ms, f.inner_ends, frac, rfrac, judged.len(),
                snippet(&src, a, b).replace('\t', " ")
            );
        }
        std::fs::write(&path, d)?;
        let labels: String = gt.word.iter().map(|g| match g { Some(true) => '1', Some(false) => '0', None => '-' }).collect();
        std::fs::write(format!("{path}.gt"), labels)?;
        // other auto proposals (fillers, stutters) as word spans, for combined numbers
        let mut o = String::new();
        for p in report.auto.iter().chain(&report.review) {
            if let (Span::Words(a, b), false) = (p.span, p.why == Why::Retake) {
                let w = &src.words;
                let gb = if a > 0 { w[a].start_ms - w[a - 1].end_ms } else { 0.0 };
                let ga = if b + 1 < w.len() { w[b + 1].start_ms - w[b].end_ms } else { 0.0 };
                let _ = writeln!(o, "{a}\t{b}\t{}\t{:.3}\t{}\t{gb:.0}\t{ga:.0}\t{:.0}", p.why.as_str(), p.confidence, snippet(&src, a, b), w[b].end_ms - w[a].start_ms);
            }
        }
        std::fs::write(format!("{path}.other"), o)?;
        eprintln!("dumped candidates to {path}");
    }
    let _ = std::fs::write(std::env::temp_dir().join("cutroom-eval-review.md"), &md);
    let _ = std::fs::write(std::env::temp_dir().join("cutroom-eval-cuts.yaml"), &yaml);
    Ok(())
}
