//! `cutroom proofs`: image beats that ask for proof (`proof: {claim, find}`) and have no file yet.
//! A capture step (a person, a browser agent) saves each screenshot to its `src`; whatever is still
//! missing at the end becomes a plain text beat carrying the claim (`--drop-missing`), so the
//! render never waits on proof that could not be found.

use anyhow::{Context, Result};
use cutroom_core::compose::segments;
use cutroom_core::lint;
use cutroom_core::model::Source;
use cutroom_core::resolve::{self, clock};
use cutroom_core::spec::{self, BeatKind, Loaded};
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Pending {
    /// Spec file of the beat (relative to the edit folder) and its index in that file's `beats`.
    pub file: String,
    pub index: usize,
    pub at: String,
    pub out_ms: f64,
    pub out: String,
    pub claim: String,
    pub find: Option<String>,
    /// Where to save the screenshot (relative to the edit folder).
    pub src: String,
    /// `talk` (on camera) or `screen` (his screen recording shows there).
    pub line: &'static str,
}

fn open(dir: &Path) -> Result<(Loaded, Source, resolve::Plan)> {
    let sp = if dir.is_dir() { dir.join("edit.yaml") } else { dir.to_path_buf() };
    let loaded = spec::load(&sp)?;
    let src = Source::load(&loaded.source_path).with_context(|| format!("loading {}", loaded.source_path.display()))?;
    let plan = resolve::build(&loaded, &src);
    Ok((loaded, src, plan))
}

pub fn pending(dir: &Path) -> Result<Vec<Pending>> {
    let (loaded, src, plan) = open(dir)?;
    let segs = segments(&plan, &src);
    let mut out = vec![];
    for b in lint::pending_proofs(&plan, &loaded.dir) {
        let BeatKind::Image { src: path, proof: Some(req), .. } = &b.kind else { continue };
        let origin = &loaded.beat_origin[b.idx].file;
        let index = loaded.beat_origin[..b.idx].iter().filter(|o| &o.file == origin).count();
        let screen = segs.iter().find(|s| s.out0 <= b.start_ms && b.start_ms < s.out1).map(|s| s.screen.is_some()).unwrap_or(false);
        out.push(Pending {
            file: origin.clone(),
            index,
            at: loaded.spec.beats[b.idx].at.0.clone(),
            out_ms: b.start_ms,
            out: clock(b.start_ms),
            claim: req.claim.clone(),
            find: req.find.clone(),
            src: path.clone(),
            line: if screen { "screen" } else { "talk" },
        });
    }
    Ok(out)
}

/// What a missing proof turns into: its caption (what the viewer was meant to read; the claim when
/// there is none) as a title (short) or quote on camera, as a callout over the screen, or nothing
/// when it is too long for either.
pub fn fallback(p: &Pending, orig: &Value) -> Option<Value> {
    let text = orig.get("caption").and_then(|v| v.as_str()).map(str::trim).filter(|t| !t.is_empty()).unwrap_or(p.claim.trim()).trim_end_matches('.').to_string();
    let n = text.split_whitespace().count();
    let kind = match (p.line, n) {
        ("talk", 1..=7) => "title",
        ("talk", 8..=25) => "quote",
        ("screen", 1..=5) => "callout",
        _ => return None,
    };
    let mut m = Mapping::new();
    for k in ["at", "until", "hold", "lead"] {
        if let Some(v) = orig.get(k) {
            m.insert(Value::from(k), v.clone());
        }
    }
    m.insert(Value::from("kind"), Value::from(kind));
    m.insert(Value::from("text"), Value::from(text));
    Some(Value::Mapping(m))
}

pub fn drop_missing(dir: &Path) -> Result<Vec<String>> {
    let base = if dir.is_dir() { dir.to_path_buf() } else { dir.parent().unwrap_or(Path::new(".")).to_path_buf() };
    let list = pending(dir)?;
    let mut by_file: BTreeMap<String, Vec<Pending>> = BTreeMap::new();
    for p in list {
        by_file.entry(p.file.clone()).or_default().push(p);
    }
    let mut log = vec![];
    for (file, ps) in by_file {
        let path = base.join(&file);
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let mut doc: Value = serde_yaml::from_str(&text)?;
        let Some(Value::Sequence(beats)) = doc.get_mut("beats") else { continue };
        let mut remove = vec![];
        for p in &ps {
            let Some(orig) = beats.get(p.index).cloned() else { continue };
            match fallback(p, &orig) {
                Some(v) => {
                    let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("").to_string();
                    beats[p.index] = v;
                    log.push(format!("{file} #{} at {} ({}): no proof for {:?}; now a {kind}", p.index, p.at, p.out, p.claim));
                }
                None => {
                    remove.push(p.index);
                    log.push(format!("{file} #{} at {} ({}): no proof for {:?}; removed", p.index, p.at, p.out, p.claim));
                }
            }
        }
        remove.sort_unstable();
        for i in remove.into_iter().rev() {
            beats.remove(i);
        }
        std::fs::write(&path, serde_yaml::to_string(&doc)?)?;
    }
    if !log.is_empty() {
        let lp = base.join("agent").join("proofs.log");
        std::fs::create_dir_all(lp.parent().unwrap())?;
        let mut old = std::fs::read_to_string(&lp).unwrap_or_default();
        for l in &log {
            old.push_str(l);
            old.push('\n');
        }
        std::fs::write(&lp, old)?;
    } else {
        log.push("no pending proofs".into());
    }
    Ok(log)
}
