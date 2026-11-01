//! Validate model-written spec files in memory (no half-written files on disk for parallel
//! workers to trip over), salvage what is valid, and log what had to go.

use super::brief::Ctx;
use cutroom_core::compose::segments;
use cutroom_core::lint;
use cutroom_core::model::Source;
use cutroom_core::resolve::{self, Severity};
use cutroom_core::spec::{flow, Anchor, BeatSpec, ChapterSpec, Lane, Loaded, Origin, Spec};
use serde::Serialize;
use serde_yaml::Value;

/// A beat or chapter that was dropped, and why.
#[derive(Serialize, Clone, Debug)]
pub struct Dropped {
    pub file: String,
    pub item: String,
    pub why: String,
}

/// `base` with `file`'s beats and chapters replaced by `spec`'s; also returns where `spec`'s beats
/// start in the merged list.
fn with_file(base: &Loaded, file: &str, spec: &Spec) -> (Loaded, usize) {
    let mut l = base.clone();
    let keep = |o: &Origin| o.file != file;
    let (b, bo): (Vec<_>, Vec<_>) = l.spec.beats.into_iter().zip(l.beat_origin).filter(|(_, o)| keep(o)).unzip();
    let (c, co): (Vec<_>, Vec<_>) = l.spec.chapters.into_iter().zip(l.chapter_origin).filter(|(_, o)| keep(o)).unzip();
    l.spec.beats = b;
    l.beat_origin = bo;
    l.spec.chapters = c;
    l.chapter_origin = co;
    let offset = l.spec.beats.len();
    let o = Origin { file: file.to_string(), scope: spec.scope.clone() };
    for x in &spec.beats {
        l.spec.beats.push(x.clone());
        l.beat_origin.push(o.clone());
    }
    for x in &spec.chapters {
        l.spec.chapters.push(x.clone());
        l.chapter_origin.push(o.clone());
    }
    (l, offset)
}

/// Errors `check` reports for `file` if it contained `spec` (everything else as in `base`).
pub fn issues_for(base: &Loaded, src: &Source, file: &str, spec: &Spec) -> Vec<String> {
    let (l, _) = with_file(base, file, spec);
    let plan = resolve::build(&l, src);
    let segs = segments(&plan, src);
    lint::check(&plan, src, &segs, &l.dir, &[]).into_iter().filter(|i| i.sev == Severity::Error && i.msg.contains(file)).map(|i| i.msg).collect()
}

/// Output window of each of `spec`'s beats (None when it does not resolve).
pub fn resolved(base: &Loaded, src: &Source, file: &str, spec: &Spec) -> Vec<Option<(f64, f64)>> {
    let (l, offset) = with_file(base, file, spec);
    let plan = resolve::build(&l, src);
    let mut out = vec![None; spec.beats.len()];
    for b in &plan.beats {
        if b.idx >= offset && b.idx - offset < out.len() {
            out[b.idx - offset] = Some((b.start_ms, b.end_ms));
        }
    }
    out
}

/// Chapters closer together than this are one topic split in two.
pub const MIN_CHAPTER_MS: f64 = 45_000.0;

/// Videos longer than this have a hook, and the first chapter starts after it.
pub const HOOK_VIDEO_MS: f64 = 120_000.0;
/// A first chapter this close to 0:00 sits on the hook.
pub const HOOK_MS: f64 = 5000.0;

pub fn chapter_gap_errors(base: &Loaded, src: &Source, file: &str, spec: &Spec) -> Vec<String> {
    if spec.chapters.is_empty() {
        return vec![];
    }
    let (l, _) = with_file(base, file, spec);
    let plan = resolve::build(&l, src);
    let mut out: Vec<String> = plan
        .chapters
        .windows(2)
        .filter(|w| w[1].at_ms - w[0].at_ms < MIN_CHAPTER_MS)
        .map(|w| format!("chapters {:?} and {:?} are only {:.0}s apart: merge them (a chapter is a real topic, usually minutes long)", w[0].title, w[1].title, (w[1].at_ms - w[0].at_ms) / 1000.0))
        .collect();
    if let Some(c) = plan.chapters.first() {
        if c.at_ms < HOOK_MS && plan.duration_ms() > HOOK_VIDEO_MS {
            out.push(format!("chapter {:?} starts at {}: that is his hook, and a card there cuts into his first sentence; start chapter 1 where his first real section begins (the intro gets no chapter; name it in output.intro)", c.title, resolve::clock(c.at_ms)));
        }
    }
    out
}

/// The `output` keys an agent may set from a chapters answer (`kicker`, `intro`), as one flow line.
pub fn agent_output(text: &str) -> Option<String> {
    let doc: Value = serde_yaml::from_str(text).ok()?;
    let m = doc.get("output")?.as_mapping()?;
    let mut keep = serde_json::Map::new();
    for k in ["kicker", "intro"] {
        match m.get(k) {
            Some(Value::String(v)) if v.chars().count() <= 60 => {
                let v = v.trim();
                let v = if k == "kicker" { v.split_whitespace().next().unwrap_or("").chars().take(16).collect::<String>() } else { v.to_string() };
                keep.insert(k.into(), serde_json::Value::String(v));
            }
            Some(Value::Null) if k == "kicker" => {
                keep.insert(k.into(), serde_json::Value::String(String::new()));
            }
            _ => {}
        }
    }
    (!keep.is_empty()).then(|| format!("output: {}\n", serde_json::Value::Object(keep)))
}

/// Stage beats anchored on a [screen] line would cover his screen.
pub fn screen_line_errors(c: &Ctx, spec: &Spec) -> Vec<String> {
    let mut out = vec![];
    for b in &spec.beats {
        if b.kind.lane() != Lane::Stage {
            continue;
        }
        let Some(id) = b.at.0.trim().strip_prefix('w').and_then(|x| x.split(['+', '-']).next()).and_then(|x| x.parse::<usize>().ok()) else { continue };
        if let Some(Some(p)) = c.pos.get(id) {
            if c.screen[*p] {
                out.push(format!("{} at w{id} is on a [screen] line: a stage visual there would cover his screen; anchor it on a [talk] line or leave it out", b.kind.name()));
            }
        }
    }
    out
}

/// A spec file read leniently: every beat / chapter that parses on its own, plus the ones that do
/// not (with the parse error).
pub struct Lenient {
    pub scope: Option<[Anchor; 2]>,
    pub beats: Vec<(BeatSpec, String)>,
    pub chapters: Vec<(ChapterSpec, String)>,
    pub bad: Vec<(String, String)>,
}

pub fn parse_lenient(text: &str) -> Result<Lenient, String> {
    let doc: Value = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
    let mut out = Lenient { scope: None, beats: vec![], chapters: vec![], bad: vec![] };
    if let Some(s) = doc.get("scope") {
        out.scope = serde_yaml::from_value(s.clone()).ok();
    }
    let raw = |v: &Value| serde_json::to_string(v).unwrap_or_default();
    if let Some(Value::Sequence(s)) = doc.get("beats") {
        for v in s {
            match serde_yaml::from_value::<BeatSpec>(v.clone()) {
                Ok(b) => out.beats.push((b, raw(v))),
                Err(e) => out.bad.push((raw(v), e.to_string())),
            }
        }
    }
    if let Some(Value::Sequence(s)) = doc.get("chapters") {
        for v in s {
            match serde_yaml::from_value::<ChapterSpec>(v.clone()) {
                Ok(c) => out.chapters.push((c, raw(v))),
                Err(e) => out.bad.push((raw(v), e.to_string())),
            }
        }
    }
    Ok(out)
}

/// Keep every item that is valid on its own; drop the rest.
pub fn salvage(base: &Loaded, src: &Source, c: Option<&Ctx>, file: &str, l: Lenient) -> (Spec, Vec<Dropped>) {
    let mut dropped: Vec<Dropped> = l.bad.into_iter().map(|(item, why)| Dropped { file: file.into(), item, why }).collect();
    let mut keep = Spec { scope: l.scope.clone(), ..Default::default() };
    for (ch, raw) in l.chapters {
        let one = Spec { scope: l.scope.clone(), chapters: vec![ch.clone()], ..Default::default() };
        let errs = issues_for(base, src, file, &one);
        if errs.is_empty() {
            keep.chapters.push(ch);
        } else {
            dropped.push(Dropped { file: file.into(), item: raw, why: errs.join("; ") });
        }
    }
    for (b, raw) in l.beats {
        let one = Spec { scope: l.scope.clone(), beats: vec![b.clone()], ..Default::default() };
        let mut errs = issues_for(base, src, file, &one);
        if let Some(c) = c {
            errs.extend(screen_line_errors(c, &one));
        }
        if errs.is_empty() {
            keep.beats.push(b);
        } else {
            dropped.push(Dropped { file: file.into(), item: raw, why: errs.join("; ") });
        }
    }
    (keep, dropped)
}

/// A spec file in the compact one-line-per-item style.
pub fn render_file(spec: &Spec) -> String {
    let mut s = String::new();
    if let Some([a, b]) = &spec.scope {
        s.push_str(&format!("scope: [{}, {}]\n", a.0, b.0));
    }
    if !spec.chapters.is_empty() {
        s.push_str("chapters:\n");
        for c in &spec.chapters {
            s.push_str(&format!("  - {}\n", flow(c)));
        }
    }
    if spec.beats.is_empty() {
        if spec.chapters.is_empty() {
            s.push_str("beats: []\n");
        }
        return s;
    }
    s.push_str("beats:\n");
    for b in &spec.beats {
        s.push_str(&format!("  - {}\n", flow(b)));
    }
    s
}

pub fn write_dropped(path: &std::path::Path, d: &[Dropped]) -> anyhow::Result<()> {
    let mut s = String::from("# beats and chapters cutroom agent dropped because they still failed `check` after a repair round\n");
    if d.is_empty() {
        s.push_str("[]\n");
    }
    for x in d {
        s.push_str(&format!("- {}\n", flow(x)));
    }
    std::fs::write(path, s)?;
    Ok(())
}
