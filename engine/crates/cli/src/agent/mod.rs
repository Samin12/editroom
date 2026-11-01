//! `cutroom agent`: the editorial passes, headless, through Claude Code (`claude -p`).
//!
//! The engine does everything deterministic; the model only makes editorial calls, in parallel:
//! 1. cuts: the detector's confident cuts, then an editorial pass over overlapping chunks of the
//!    kept transcript (retakes, restarts, repeats the detector missed, its unsure candidates, and
//!    silent screen actions worth keeping). Every cut quotes its words and is checked against them.
//! 2. chapters: the whole kept transcript -> chapters YAML.
//! 3. visuals, one call per chapter (stage visuals, proof requests, custom scenes), and at the same
//!    time the screen pass: frames of the screen recording -> rings, highlights, zooms, arrows.
//!
//! Every answer is validated in memory by the same checks as `cutroom check`. A file with errors
//! gets one repair round; whatever still fails is dropped and logged to `agent/dropped.yaml`, so
//! agent output never blocks a render.

mod beats;
mod brief;
mod editorial;
pub mod llm;
mod prompts;
mod screen;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use beats::Dropped;
use brief::Ctx;
use cutroom_core::anchor::{self, norm, Index};
use cutroom_core::compose::segments;
use cutroom_core::lint;
use cutroom_core::model::Source;
use cutroom_core::resolve::{self, clock, Plan};
use cutroom_core::spec::{self, flow, Anchor, CutSpec, Lane, Loaded, Spec};
use cutroom_cut::{CutReport, Span};
pub use llm::Usage;
use llm::{par_map, Llm};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct AgentOpts {
    /// Chapters and visuals.
    pub model: String,
    /// The editorial cut pass (and the review of unsure cuts).
    pub cut_model: String,
    /// The screen pass (looks at frames).
    pub screen_model: String,
    pub jobs: usize,
    /// Judge the detector's unsure candidates.
    pub review: bool,
    pub editorial: bool,
    pub screen: bool,
    pub scenes: bool,
    /// Chapters and visuals (false: cuts only).
    pub beats: bool,
    /// Regenerate the cuts (false: keep cuts.yaml and review.md as they are).
    pub cuts: bool,
    pub chunk_words: usize,
}

impl Default for AgentOpts {
    fn default() -> Self {
        AgentOpts { model: "sonnet".into(), cut_model: "opus".into(), screen_model: "sonnet".into(), jobs: 8, review: true, editorial: true, screen: true, scenes: true, beats: true, cuts: true, chunk_words: 1500 }
    }
}

#[derive(Serialize, Default)]
struct CutStats {
    detector: usize,
    review_candidates: usize,
    review_accepted: usize,
    editorial: usize,
    pauses_kept: usize,
    human: usize,
    /// Source seconds removed by the detector alone, and by everything.
    detector_s: f64,
    removed_s: f64,
    source_s: f64,
}

#[derive(Serialize, Default)]
struct RunReport {
    #[serde(flatten)]
    total: Usage,
    passes: BTreeMap<&'static str, Usage>,
    cuts: CutStats,
    beats: BTreeMap<String, usize>,
    screen_marks: usize,
    dropped: usize,
    pending_proofs: usize,
}

fn load(dir: &Path) -> Result<(Loaded, Source, Plan)> {
    let loaded = spec::load(&dir.join("edit.yaml"))?;
    let src = Source::load(&loaded.source_path)?;
    let plan = resolve::build(&loaded, &src);
    Ok((loaded, src, plan))
}

/// Cuts in `cuts.yaml` that a regenerating pass keeps: `by: human` always, `by: agent` when asked.
pub fn kept_cuts(dir: &Path, keep_agent: bool) -> Vec<CutSpec> {
    spec::read_spec(&dir.join("cuts.yaml"))
        .map(|s| s.cuts.into_iter().filter(|c| matches!(c.by.as_deref(), Some("human")) || (keep_agent && c.by.as_deref() == Some("agent"))).collect())
        .unwrap_or_default()
}

/// Append cuts to a `cuts:` file under a comment.
pub fn append_cuts(s: &mut String, title: &str, cuts: &[CutSpec]) {
    if cuts.is_empty() {
        return;
    }
    s.push_str(&format!("  # {title}\n"));
    for c in cuts {
        s.push_str(&format!("  - {}\n", flow(c)));
    }
}

fn ed_spec(e: &editorial::EdCut) -> CutSpec {
    CutSpec {
        from: Some(Anchor(format!("w{}", e.from))),
        to: (e.to != e.from).then(|| Anchor(format!("w{}", e.to))),
        ms: None,
        why: Some(e.why.clone()),
        by: Some("agent".into()),
        note: e.note.clone(),
    }
}

fn cuts_file(report: &CutReport, ed: &[editorial::EdCut], human: &[CutSpec]) -> String {
    let mut s = cutroom_cut::to_yaml(report);
    append_cuts(&mut s, "editorial cuts (cutroom agent; delete a line to undo it)", &ed.iter().map(ed_spec).collect::<Vec<_>>());
    append_cuts(&mut s, "human cuts (kept when cuts are regenerated)", human);
    s
}

/// Move the previous run's generated files out of the way (never deleted). `screen.yaml` only moves
/// when the screen pass will write a new one.
fn stash_previous(dir: &Path, log: &Path, screen: bool) -> Result<()> {
    let beats = dir.join("beats");
    let Ok(rd) = std::fs::read_dir(&beats) else { return Ok(()) };
    let old: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n == "00-chapters.yaml" || (screen && n == "screen.yaml") || (n.starts_with("ch") && n.ends_with(".yaml") && n[2..n.len() - 5].chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    if old.is_empty() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let to = log.join(format!("prev-{stamp}"));
    std::fs::create_dir_all(&to)?;
    for p in old {
        std::fs::rename(&p, to.join(p.file_name().unwrap()))?;
    }
    eprintln!("moved the previous run's beat files to {}", to.display());
    Ok(())
}

/// Image and video files in the usual asset folders, relative to the edit folder.
fn list_assets(dir: &Path) -> (Vec<String>, Vec<String>) {
    let (mut img, mut vid) = (vec![], vec![]);
    for sub in ["proof", "assets", "images", "broll", "clips"] {
        let Ok(rd) = std::fs::read_dir(dir.join(sub)) else { continue };
        let mut v: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        v.sort();
        for p in v {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            let rel = format!("{sub}/{}", p.file_name().and_then(|n| n.to_str()).unwrap_or(""));
            match ext.as_str() {
                "png" | "jpg" | "jpeg" | "webp" => img.push(rel),
                "mp4" | "mov" | "m4v" => vid.push(rel),
                _ => {}
            }
        }
    }
    (img, vid)
}

struct FileOut {
    usage: Usage,
    items: usize,
    dropped: Vec<Dropped>,
}

/// Ask for one spec file, validate it in memory, repair once, salvage what is valid, write it.
#[allow(clippy::too_many_arguments)]
fn write_validated(llm: &Llm, model: &str, prompt: &str, dir: &Path, file: &str, scope: Option<[Anchor; 2]>, base: &Loaded, src: &Source, c: Option<&Ctx>, log: &Path, name: &str) -> Result<FileOut> {
    let check = |text: &str| -> std::result::Result<Spec, Vec<String>> {
        let mut sp: Spec = serde_yaml::from_str(text).map_err(|e| vec![format!("not valid for this format: {e}")])?;
        if sp.scope.is_none() {
            sp.scope = scope.clone();
        }
        let mut errs = beats::issues_for(base, src, file, &sp);
        errs.extend(beats::chapter_gap_errors(base, src, file, &sp));
        if let Some(c) = c {
            errs.extend(beats::screen_line_errors(c, &sp));
        }
        if errs.is_empty() {
            Ok(sp)
        } else {
            Err(errs)
        }
    };
    let (ans, mut u) = llm.ask(model, prompt, &[], &log.join(format!("{name}.json")))?;
    let mut last = ans.clone();
    let (sp, dropped) = match check(&ans) {
        Ok(sp) => (sp, vec![]),
        Err(errs) => {
            let (ans2, u2) = llm.ask(model, &prompts::fix(prompt, &ans, &errs), &[], &log.join(format!("{name}-fix.json")))?;
            u.add(&u2);
            last = ans2.clone();
            match check(&ans2) {
                Ok(sp) => (sp, vec![]),
                Err(_) => {
                    let mut l = beats::parse_lenient(&ans2).or_else(|_| beats::parse_lenient(&ans)).map_err(|e| anyhow!("{file}: unreadable answer: {e}"))?;
                    if l.scope.is_none() {
                        l.scope = scope.clone();
                    }
                    beats::salvage(base, src, c, file, l)
                }
            }
        }
    };
    // a chapters answer may set how chapters are named on screen (output.kicker / output.intro)
    let out = if sp.chapters.is_empty() { None } else { beats::agent_output(&last).or_else(|| beats::agent_output(&ans)) };
    std::fs::write(dir.join(file), format!("{}{}", out.unwrap_or_default(), beats::render_file(&sp)))?;
    Ok(FileOut { usage: u, items: sp.beats.len() + sp.chapters.len(), dropped })
}

/// The editorial pass over one chunk: ask, validate, one repair round, keep what is valid.
#[allow(clippy::too_many_arguments)]
fn run_chunk(llm: &Llm, o: &AgentOpts, c: &Ctx, ch: &editorial::Chunk, report: &CutReport, marks: &brief::Marks, normw: &[String], log: &Path) -> Result<(editorial::EdResult, Usage)> {
    let text = brief::brief(c, ch.lo, ch.hi, 6, Some(marks), 40);
    let prompt = format!("{}{}\n\nTranscript:\n{text}", prompts::EDITORIAL_PROMPT, prompts::TRANSCRIPT_KEY);
    let name = log.join(format!("cuts-{:02}.json", ch.k + 1));
    let (ans, mut u) = llm.ask(&o.cut_model, &prompt, &[], &name)?;
    let mut r = editorial::validate(&ans, ch, c, report, normw);
    if !r.errors.is_empty() {
        let (ans2, u2) = llm.ask(&o.cut_model, &prompts::fix(&prompt, &ans, &r.errors), &[], &name.with_file_name(format!("cuts-{:02}-fix.json", ch.k + 1)))?;
        u.add(&u2);
        let r2 = editorial::validate(&ans2, ch, c, report, normw);
        if !r2.errors.is_empty() {
            eprintln!("  cuts {:02}: {} problems left after repair; keeping the valid cuts ({})", ch.k + 1, r2.errors.len(), r2.errors.join(" | ").chars().take(240).collect::<String>());
        }
        r = r2;
    }
    if r.over_cap {
        eprintln!("  cuts {:02}: still removes more than half of its stretch; dropping its editorial cuts", ch.k + 1);
        r.cuts.clear();
    }
    Ok((editorial::owned(r, ch, report), u))
}

fn excerpt(src: &Source, a: usize, b: usize) -> String {
    let w = &src.words;
    let join = |x: usize, y: usize| w[x..y].iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join(" ");
    if b + 1 - a > 14 {
        format!("{} … {}", join(a, a + 6), join(b + 1 - 6, b + 1))
    } else {
        join(a, b + 1)
    }
}

#[allow(clippy::too_many_arguments)]
fn summary_md(src: &Source, plan: &Plan, review: &[cutroom_cut::Proposal], accepted: &[usize], ed: &[editorial::EdCut], pauses: &[editorial::EdPause], st: &CutStats, reviewed: bool) -> String {
    let mut s = String::from("# Cuts (cutroom agent)\n\n");
    s.push_str(&format!(
        "Removed {} of {} ({:.0}%). Detector: {} cuts ({}). Unsure candidates: {} of {} cut. Editorial: {} cuts. Screen pauses kept: {}.\n\n",
        clock(st.removed_s * 1000.0),
        clock(st.source_s * 1000.0),
        100.0 * st.removed_s / st.source_s.max(1e-9),
        st.detector,
        clock(st.detector_s * 1000.0),
        st.review_accepted,
        st.review_candidates,
        st.editorial,
        st.pauses_kept
    ));
    s.push_str("Times are in the edited video (out) and the raw recording (src). To undo an editorial cut, delete its line (`by: agent`) from cuts.yaml; to force one, add `- {from: wA, to: wB, by: human}`.\n");
    if !ed.is_empty() {
        s.push_str("\n## Editorial cuts\n\n");
        for e in ed {
            let a = src.words[e.from].start_ms;
            let b = src.words[e.to].end_ms;
            s.push_str(&format!("- out {} · src {} · {:.1}s · {} · w{}-w{}: \"{}\"{}\n", clock(plan.map.to_out(a)), clock(a), (b - a) / 1000.0, e.why, e.from, e.to, excerpt(src, e.from, e.to), e.note.as_ref().map(|n| format!(" ({n})")).unwrap_or_default()));
        }
    }
    if !review.is_empty() {
        s.push_str(if reviewed { "\n## Unsure detector candidates\n\n" } else { "\n## Unsure detector candidates (not reviewed; accept with `cutroom cut --accept`)\n\n" });
        for (i, p) in review.iter().enumerate() {
            let verdict = if accepted.contains(&i) { "cut " } else { "kept" };
            match p.span {
                Span::Words(a, b) => s.push_str(&format!("- r{} {verdict} · src {} · {}: \"{}\"\n", i + 1, clock(src.words[a].start_ms), p.why.as_str(), excerpt(src, a, b))),
                Span::Ms(a, b) => s.push_str(&format!("- r{} {verdict} · src {}-{} · {}\n", i + 1, clock(a), clock(b), p.reason)),
            }
        }
    }
    if !pauses.is_empty() {
        s.push_str("\n## Screen pauses kept\n\n");
        for p in pauses {
            let t = src.words[p.after].end_ms;
            s.push_str(&format!("- after w{} {:?} · out {} · kept {:.1}s{}\n", p.after, src.words[p.after].text, clock(plan.map.to_out(t)), p.keep_ms / 1000.0, p.note.as_ref().map(|n| format!(" ({n})")).unwrap_or_default()));
        }
    }
    s
}

pub fn run(dir: &Path, o: &AgentOpts) -> Result<Usage> {
    let t0 = Instant::now();
    let log = dir.join("agent");
    std::fs::create_dir_all(&log)?;
    std::fs::create_dir_all(dir.join("beats"))?;
    let style = std::fs::read_to_string(dir.join("style.md")).ok();
    let llm = Llm::new(prompts::system(style.as_deref()));
    let mut rep = RunReport::default();
    if o.beats {
        stash_previous(dir, &log, o.screen)?;
    }

    // ---- 1. cuts ----
    if o.cuts {
        run_cuts(dir, o, &llm, &log, &mut rep)?;
    }

    // ---- 2. chapters + visuals, and the screen pass alongside ----
    visuals(dir, o, &llm, &log, &mut rep)?;

    for u in rep.passes.values() {
        rep.total.add(u);
    }
    rep.total.wall_s = t0.elapsed().as_secs_f64();
    for (k, u) in &rep.passes {
        eprintln!("  {k:<9} {:>3} calls  {:>8} in  {:>6} out  ${:.3}  {:.0}s", u.calls, u.total_in(), u.output, u.cost_usd, u.wall_s);
    }
    if o.beats {
        eprintln!("beats {:?}; {} screen marks; {} dropped (agent/dropped.yaml); {} proofs pending (`cutroom proofs`)", rep.beats, rep.screen_marks, rep.dropped, rep.pending_proofs);
    }
    std::fs::write(log.join("usage.json"), serde_json::to_string_pretty(&rep)?)?;
    Ok(rep.total)
}

fn run_cuts(dir: &Path, o: &AgentOpts, llm: &Llm, log: &Path, rep: &mut RunReport) -> Result<()> {
    let tc = Instant::now();
    let (_, src, _) = load(dir)?;
    let human = kept_cuts(dir, false);
    let mut report = cutroom_cut::detect(&src, &cutroom_cut::CutOpts::default());
    let review_before = report.review.clone();
    rep.cuts.detector = report.auto.len();
    rep.cuts.review_candidates = report.review.len();
    rep.cuts.human = human.len();
    rep.cuts.detector_s = report.removed_ms(&src, false) / 1000.0;
    rep.cuts.source_s = src.duration_ms / 1000.0;
    let mut ed_cuts = vec![];
    let mut pauses = vec![];
    let mut accepted: Vec<usize> = vec![];
    if o.editorial {
        std::fs::write(dir.join("cuts.yaml"), cuts_file(&report, &[], &human))?;
        let (_, src1, plan1) = load(dir)?;
        let c = Ctx::new(&src1, &plan1);
        let mut marks = brief::Marks { pause: brief::pause_marks(&c, 1500.0, 20_000.0), ..Default::default() };
        if o.review {
            for (i, p) in report.review.iter().enumerate() {
                if let Span::Words(a, b) = p.span {
                    marks.open.insert(a, i + 1);
                    *marks.close.entry(b).or_insert(0) += 1;
                }
            }
        }
        let normw: Vec<String> = src1.words.iter().map(|w| norm(&w.text)).collect();
        let chunks = editorial::chunks(&c.kept, o.chunk_words, o.chunk_words * 15 / 100);
        eprintln!("cuts: detector {} cuts ({}), {} unsure; editorial pass over {} chunks with {}", report.auto.len(), clock(rep.cuts.detector_s * 1000.0), report.review.len(), chunks.len(), o.cut_model);
        let results = par_map(chunks, o.jobs, |ch| run_chunk(llm, o, &c, &ch, &report, &marks, &normw, log));
        let mut u = Usage::default();
        let mut all = vec![];
        for r in results {
            match r {
                Ok((r, uu)) => {
                    u.add(&uu);
                    accepted.extend(r.accept);
                    all.extend(r.cuts);
                    pauses.extend(r.pauses);
                }
                Err(e) => eprintln!("  cuts: {e}"),
            }
        }
        accepted.sort_unstable();
        accepted.dedup();
        if !o.review {
            accepted.clear();
        }
        report = cutroom_cut::accept(&report, &accepted);
        rep.cuts.pauses_kept = editorial::apply_pauses(&mut report, &pauses);
        ed_cuts = editorial::merge_cuts(all);
        u.wall_s = tc.elapsed().as_secs_f64();
        rep.passes.insert("cuts", u);
    } else if o.review && !report.review.is_empty() {
        let review = cutroom_cut::review_md(&report, &src);
        let (ans, mut u) = llm.ask(&o.cut_model, &format!("{}{review}", prompts::REVIEW_PROMPT), &[], &log.join("review.json"))?;
        accepted = cutroom_cut::parse_review_answer(&ans, report.review.len())?;
        report = cutroom_cut::accept(&report, &accepted);
        u.wall_s = tc.elapsed().as_secs_f64();
        rep.passes.insert("review", u);
    }
    rep.cuts.review_accepted = accepted.len();
    rep.cuts.editorial = ed_cuts.len();
    std::fs::write(dir.join("cuts.yaml"), cuts_file(&report, &ed_cuts, &human))?;
    let (_, src, plan) = load(dir)?;
    rep.cuts.removed_s = (src.duration_ms - plan.map.kept_ms()) / 1000.0;
    std::fs::write(dir.join("review.md"), summary_md(&src, &plan, &review_before, &accepted, &ed_cuts, &pauses, &rep.cuts, o.review))?;
    eprintln!(
        "cuts: {} removed of {} (detector alone {}): {} unsure cut, {} editorial cuts, {} screen pauses kept ({:.0}s); summary in review.md",
        clock(rep.cuts.removed_s * 1000.0),
        clock(src.duration_ms),
        clock(rep.cuts.detector_s * 1000.0),
        accepted.len(),
        ed_cuts.len(),
        rep.cuts.pauses_kept,
        tc.elapsed().as_secs_f64()
    );
    Ok(())
}

fn visuals(dir: &Path, o: &AgentOpts, llm: &Llm, log: &Path, rep: &mut RunReport) -> Result<()> {
    if o.beats {
        let (base, src, plan) = load(dir)?;
        let c = Ctx::new(&src, &plan);
        let segs = segments(&plan, &src);
        let (vis, scr) = std::thread::scope(|s| {
            let scr = s.spawn(|| if o.screen { screen_pass(llm, o, dir, log, &base, &src, &c, &segs) } else { Ok(ScreenOut::default()) });
            let vis = chapters_and_visuals(llm, o, dir, log, &base, &src, &c);
            (vis, scr.join().map_err(|_| anyhow!("screen pass panicked")).and_then(|r| r))
        });
        let mut dropped = vec![];
        match vis {
            Ok(v) => {
                rep.passes.insert("chapters", v.chapters);
                rep.passes.insert("visuals", v.visuals);
                dropped.extend(v.dropped);
            }
            Err(e) => eprintln!("visuals: {e}"),
        }
        match scr {
            Ok(sc) => {
                rep.screen_marks = sc.marks;
                if o.screen {
                    rep.passes.insert("screen", sc.usage);
                }
                dropped.extend(sc.dropped);
            }
            Err(e) => eprintln!("screen: {e}"),
        }
        rep.dropped = dropped.len();
        beats::write_dropped(&log.join("dropped.yaml"), &dropped)?;
        let (loaded, _, plan) = load(dir)?;
        for b in &plan.beats {
            *rep.beats.entry(b.kind.name().to_string()).or_default() += 1;
        }
        rep.pending_proofs = lint::pending_proofs(&plan, &loaded.dir).len();
    }
    Ok(())
}

struct VisOut {
    chapters: Usage,
    visuals: Usage,
    dropped: Vec<Dropped>,
}

fn chapters_and_visuals(llm: &Llm, o: &AgentOpts, dir: &Path, log: &Path, base: &Loaded, src: &Source, c: &Ctx) -> Result<VisOut> {
    let t = Instant::now();
    let n = c.kept.len();
    let whole = brief::brief(c, 0, n, 0, None, 45);
    let chapters_file = "beats/00-chapters.yaml";
    let glossary = prompts::names_glossary(std::fs::read_to_string(dir.join("style.md")).ok().as_deref());
    let mut ch = write_validated(llm, &o.model, &format!("{}{whole}", prompts::chapter_prompt(glossary.as_deref())), dir, chapters_file, None, base, src, None, log, "chapters")?;
    ch.usage.wall_s = t.elapsed().as_secs_f64();
    let mut dropped = ch.dropped;
    eprintln!("chapters: {} ({:.0}s)", ch.items, ch.usage.wall_s);

    // chapter ranges, in kept positions
    let s: Spec = spec::read_spec(&dir.join(chapters_file))?;
    let idx = Index::new(src);
    let kept = |i: usize| c.pos.get(i).map(|p| p.is_some()).unwrap_or(false);
    let mut starts: Vec<usize> = s.chapters.iter().filter_map(|ch| anchor::resolve(&ch.at, src, &idx, (0, src.words.len()), &kept).ok().and_then(|r| r.word)).map(|w| c.pos_at_or_after(w)).filter(|&p| p < n).collect();
    starts.push(0);
    starts.sort_unstable();
    starts.dedup();
    let ranges: Vec<(usize, usize, usize)> = starts.iter().enumerate().map(|(i, &a)| (i, a, starts.get(i + 1).copied().unwrap_or(n))).filter(|r| r.2 > r.1).collect();

    // chapter number of each range (0: the intro before chapter 1), for on-screen numbering
    let mut chs: Vec<(usize, String)> = s.chapters.iter().filter_map(|ch| anchor::resolve(&ch.at, src, &idx, (0, src.words.len()), &kept).ok().and_then(|r| r.word).map(|w| (c.pos_at_or_after(w), ch.title.clone()))).collect();
    chs.sort_by_key(|x| x.0);
    chs.dedup_by_key(|x| x.0);
    let total = chs.len();

    // visuals per chapter, validated against the edit with chapters in it
    let (base, _, _) = load(dir)?;
    let kicker = base.spec.output.kicker.clone();
    let (images, videos) = list_assets(dir);
    let bp = prompts::beats_prompt(&prompts::BeatsOpts { scenes: o.scenes, screen_pass: o.screen, images, videos, glossary });
    let tv = Instant::now();
    let res = par_map(ranges, o.jobs, |(i, a, b)| {
        let num = chs.iter().filter(|x| x.0 <= a).count();
        let title = num.checked_sub(1).map(|k| chs[k].1.as_str()).unwrap_or("");
        let head = bp.strip_suffix("Chapter transcript:\n").unwrap_or(&bp);
        let text = format!("{}Chapter transcript:\n{}", prompts::chapter_context(num, total, title, &kicker), brief::brief(c, a, b, 6, None, 40));
        let file = format!("beats/ch{:02}.yaml", i + 1);
        let scope = Some([Anchor(format!("w{}", c.kept[a])), Anchor(format!("w{}", c.kept[b - 1]))]);
        write_validated(llm, &o.model, &format!("{head}{text}"), dir, &file, scope, &base, src, Some(c), log, &format!("ch{:02}", i + 1))
    });
    let mut vis = Usage::default();
    for r in res {
        match r {
            Ok(f) => {
                vis.add(&f.usage);
                dropped.extend(f.dropped);
            }
            Err(e) => eprintln!("  visuals: {e}"),
        }
    }
    vis.wall_s = tv.elapsed().as_secs_f64();
    Ok(VisOut { chapters: ch.usage, visuals: vis, dropped })
}

#[derive(Default)]
struct ScreenOut {
    usage: Usage,
    marks: usize,
    dropped: Vec<Dropped>,
}

/// Most screen windows one run looks at (about 2.5 hours of screen time).
const MAX_WINDOWS: usize = 200;

#[allow(clippy::too_many_arguments)]
fn screen_pass(llm: &Llm, o: &AgentOpts, dir: &Path, log: &Path, base: &Loaded, src: &Source, c: &Ctx, segs: &[cutroom_core::compose::Seg]) -> Result<ScreenOut> {
    let t = Instant::now();
    let file = "beats/screen.yaml";
    let mut wins = screen::windows(c, segs);
    if wins.len() > MAX_WINDOWS {
        eprintln!("screen: {} stretches; looking at the first {MAX_WINDOWS}", wins.len());
        wins.truncate(MAX_WINDOWS);
    }
    let glossary = prompts::names_glossary(std::fs::read_to_string(dir.join("style.md")).ok().as_deref());
    let frames_dir = log.join("frames");
    std::fs::create_dir_all(&frames_dir)?;
    eprintln!("screen: {} stretches with {}", wins.len(), o.screen_model);
    let res = par_map(wins, o.jobs, |w| -> Result<(Vec<spec::BeatSpec>, Vec<Dropped>, Usage)> {
        let mut imgs = vec![];
        let mut kept_frames = vec![];
        for (i, f) in w.frames.iter().enumerate() {
            let p = frames_dir.join(format!("s{:03}-{}.jpg", w.k + 1, i + 1));
            match screen::extract(f, &p) {
                Ok(()) => {
                    imgs.push(p);
                    kept_frames.push(f.clone());
                }
                Err(e) => eprintln!("  screen {:03}: {e}", w.k + 1),
            }
        }
        if imgs.is_empty() {
            return Ok((vec![], vec![], Usage::default()));
        }
        let w = screen::Window { frames: kept_frames, ..w };
        let (ans, u) = llm.ask(&o.screen_model, &screen::prompt(c, &w, glossary.as_deref()), &imgs, &log.join(format!("screen-{:03}.json", w.k + 1)))?;
        let l = beats::parse_lenient(&ans).map_err(|e| anyhow!("screen {:03}: unreadable answer: {e}", w.k + 1))?;
        let mut dropped: Vec<Dropped> = l.bad.into_iter().map(|(item, why)| Dropped { file: file.into(), item, why }).collect();
        let (ok, bad) = screen::check_marks(c, &w, l.beats);
        dropped.extend(bad.into_iter().map(|(item, why)| Dropped { file: file.into(), item, why }));
        let mut keep = vec![];
        for b in ok {
            let one = Spec { beats: vec![b.clone()], ..Default::default() };
            let errs = beats::issues_for(base, src, file, &one);
            if errs.is_empty() {
                keep.push(b);
            } else {
                dropped.push(Dropped { file: file.into(), item: flow(&b), why: errs.join("; ") });
            }
        }
        Ok((keep, dropped, u))
    });
    let mut out = ScreenOut::default();
    let mut all = vec![];
    for r in res {
        match r {
            Ok((k, d, u)) => {
                all.extend(k);
                out.dropped.extend(d);
                out.usage.add(&u);
            }
            Err(e) => eprintln!("  screen: {e}"),
        }
    }
    // one mark at a time per lane: resolve them together and drop later ones that overlap
    let sp = Spec { beats: all, ..Default::default() };
    let windows = beats::resolved(base, src, file, &sp);
    let mut order: Vec<usize> = (0..sp.beats.len()).filter(|&i| windows[i].is_some()).collect();
    order.sort_by(|&a, &b| windows[a].unwrap().0.total_cmp(&windows[b].unwrap().0));
    let mut last_end: BTreeMap<Lane, f64> = BTreeMap::new();
    let mut keep = vec![];
    for i in order {
        let (s0, s1) = windows[i].unwrap();
        let lane = sp.beats[i].kind.lane();
        let e = last_end.entry(lane).or_insert(f64::MIN);
        if s0 < *e - 1.0 {
            out.dropped.push(Dropped { file: file.into(), item: flow(&sp.beats[i]), why: "overlaps an earlier mark".into() });
            continue;
        }
        *e = s1;
        keep.push(sp.beats[i].clone());
    }
    out.marks = keep.len();
    std::fs::write(dir.join(file), beats::render_file(&Spec { beats: keep, ..Default::default() }))?;
    out.usage.wall_s = t.elapsed().as_secs_f64();
    eprintln!("screen: {} marks ({:.0}s)", out.marks, out.usage.wall_s);
    Ok(out)
}
