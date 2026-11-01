//! Dump the hand edit's per-word ground truth for a work folder as JSON (for benchmarks):
//! `cargo run --release -p cutroom-cut --example dump_truth -- --db yc.bmproj <work dir>`
//! prints [{"i": word index, "text": ..., "cut": true|false|null}, ...].

use cutroom_core::spec;
use cutroom_core::{Source, Track};
use cutroom_ingest::Project;
use std::collections::BTreeMap;
use std::path::Path;

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


fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (db, dir) = if a.len() >= 3 && a[0] == "--db" { (a[1].clone(), a[2].clone()) } else { ("/private/tmp/cutroom-probe/yc.bmproj".into(), a[0].clone()) };
    let project = Project::open_db(Path::new(&db), Path::new(""))?;
    let kept = project.mic_kept_ranges();
    let loaded = spec::load(&Path::new(&dir).join("edit.yaml"))?;
    let src = Source::load(&loaded.source_path)?;
    let t = truth(&src, &kept);
    let rows: Vec<serde_json::Value> = src.words.iter().enumerate().map(|(i, w)| serde_json::json!({"i": i, "text": w.text, "cut": t[i]})).collect();
    println!("{}", serde_json::to_string(&rows)?);
    Ok(())
}
