//! Word lists: tidying, the simple transcript JSON, whisper.cpp output and running whisper-cli.

use anyhow::{anyhow, bail, Context, Result};
use cutroom_core::Word;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Sort words by start and make them non-overlapping: an overlap is clamped by ending the earlier
/// word where the later one starts. Empty texts are dropped, `end < start` is fixed.
pub fn tidy(words: &mut Vec<Word>) {
    words.retain(|w| !w.text.trim().is_empty() && w.start_ms.is_finite() && w.end_ms.is_finite());
    for w in words.iter_mut() {
        w.text = w.text.trim().to_string();
        if w.end_ms < w.start_ms {
            w.end_ms = w.start_ms;
        }
    }
    words.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms).then(a.end_ms.total_cmp(&b.end_ms)));
    for i in 1..words.len() {
        let next_start = words[i].start_ms;
        let prev = &mut words[i - 1];
        if prev.end_ms > next_start {
            prev.end_ms = next_start.max(prev.start_ms);
        }
    }
}

/// Parse a transcript file. Accepted shapes (times in seconds unless noted):
/// * `[{"text","start","end"}, ...]` (the simple format)
/// * `{"words": [...]}` with the same items
/// * Borumi/Parakeet `{"segments": [{"tokens": [{"text","start","end"}]}]}` (also nested under
///   `"transcription"`)
/// * whisper.cpp `-ojf` output (`{"transcription": [{"tokens": [{"text","offsets":{from,to}}]}]}`,
///   milliseconds), sub-word tokens merged into words
pub fn parse_transcript(v: &Value) -> Result<Vec<Word>> {
    if let Some(arr) = v.as_array() {
        return simple_items(arr);
    }
    if let Some(arr) = v.get("words").and_then(Value::as_array) {
        return simple_items(arr);
    }
    if let Some(segs) = v.get("segments").and_then(Value::as_array) {
        return parakeet_segments(segs);
    }
    if let Some(t) = v.get("transcription") {
        if let Some(segs) = t.get("segments").and_then(Value::as_array) {
            return parakeet_segments(segs);
        }
        if let Some(segs) = t.as_array() {
            return Ok(whisper_words(segs));
        }
    }
    bail!("unrecognised transcript JSON (expected [{{\"text\",\"start\",\"end\"}}] in seconds)")
}

pub fn load_transcript(path: &Path) -> Result<Vec<Word>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let v: Value = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let mut w = parse_transcript(&v).with_context(|| path.display().to_string())?;
    tidy(&mut w);
    Ok(w)
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}

fn simple_items(arr: &[Value]) -> Result<Vec<Word>> {
    let mut out = Vec::with_capacity(arr.len());
    for (i, it) in arr.iter().enumerate() {
        let text = it.get("text").or_else(|| it.get("word")).and_then(Value::as_str).ok_or_else(|| anyhow!("item {i}: missing text"))?;
        let s = num(it, "start").ok_or_else(|| anyhow!("item {i}: missing start"))?;
        let e = num(it, "end").ok_or_else(|| anyhow!("item {i}: missing end"))?;
        out.push(Word { text: text.to_string(), start_ms: s * 1000.0, end_ms: e * 1000.0 });
    }
    Ok(out)
}

/// Borumi's `transcription.segments[].tokens[]` (Parakeet: one token per word, seconds).
pub fn parakeet_segments(segs: &[Value]) -> Result<Vec<Word>> {
    let mut out = vec![];
    for s in segs {
        if let Some(toks) = s.get("tokens").and_then(Value::as_array) {
            out.extend(simple_items(toks)?);
        }
    }
    Ok(out)
}

/// whisper.cpp `-ojf` JSON: merge sub-word tokens (a token starting with a space starts a word),
/// drop special tokens (`[_BEG_]`, `[_TT_123]`), glue punctuation to the previous word.
pub fn whisper_words(segs: &[Value]) -> Vec<Word> {
    let mut out: Vec<Word> = vec![];
    for s in segs {
        let Some(toks) = s.get("tokens").and_then(Value::as_array) else { continue };
        let mut first = true;
        for t in toks {
            let text = t.get("text").and_then(Value::as_str).unwrap_or("");
            if text.is_empty() || (text.starts_with("[_") && text.ends_with(']')) {
                continue;
            }
            let off = t.get("offsets");
            let from = off.and_then(|o| num(o, "from")).unwrap_or(0.0);
            let to = off.and_then(|o| num(o, "to")).unwrap_or(from);
            let punct = text.trim().chars().all(|c| !c.is_alphanumeric());
            let starts_word = text.starts_with(' ') || first;
            first = false;
            match out.last_mut() {
                Some(w) if punct || !starts_word => {
                    w.text.push_str(text.trim_end());
                    if !punct {
                        w.end_ms = w.end_ms.max(to);
                    }
                }
                _ if punct => {}
                _ => out.push(Word { text: text.trim().to_string(), start_ms: from, end_ms: to }),
            }
        }
    }
    out
}

/// whisper-cli binary: `CUTROOM_WHISPER_BIN`, else `whisper-cli` on PATH, else Homebrew's.
pub fn whisper_bin() -> PathBuf {
    if let Ok(p) = std::env::var("CUTROOM_WHISPER_BIN") {
        return PathBuf::from(p);
    }
    let brew = PathBuf::from("/opt/homebrew/bin/whisper-cli");
    if brew.exists() {
        return brew;
    }
    PathBuf::from("whisper-cli")
}

/// Model: `CUTROOM_WHISPER_MODEL`, else the largest English ggml model found in the usual places.
pub fn whisper_model() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("CUTROOM_WHISPER_MODEL") {
        let p = PathBuf::from(p);
        if !p.exists() {
            bail!("CUTROOM_WHISPER_MODEL={} does not exist", p.display());
        }
        return Ok(p);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        format!("{home}/Library/Application Support/SceneForge/studio/models"),
        format!("{home}/.cache/whisper"),
        "/opt/homebrew/share/whisper-cpp/models".to_string(),
        "/opt/homebrew/share/whisper-cpp".to_string(),
    ];
    for name in ["ggml-large-v3-turbo.bin", "ggml-medium.en.bin", "ggml-small.en.bin", "ggml-base.en.bin", "ggml-tiny.en.bin"] {
        for d in &dirs {
            let p = Path::new(d).join(name);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    bail!("no whisper model found; set CUTROOM_WHISPER_MODEL to a ggml-*.bin file")
}

/// Transcribe any audio/video file with whisper-cli: ffmpeg makes a 16 kHz mono wav, whisper
/// writes full JSON with per-token offsets, tokens are merged into words (media ms).
pub fn whisper(input: &Path) -> Result<Vec<Word>> {
    let model = whisper_model()?;
    let dir = crate::temp_dir("whisper")?;
    let wav = dir.join("audio.wav");
    let st = Command::new("ffmpeg")
        .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(&wav)
        .status()
        .context("running ffmpeg")?;
    if !st.success() {
        let _ = std::fs::remove_dir_all(&dir);
        bail!("ffmpeg could not extract audio from {}", input.display());
    }
    let base = dir.join("out");
    let threads = std::thread::available_parallelism().map(|n| n.get().min(8)).unwrap_or(4);
    let out = Command::new(whisper_bin())
        .arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(&wav)
        .args(["-ojf", "-np", "-t", &threads.to_string(), "-of"])
        .arg(&base)
        .output()
        .context("running whisper-cli")?;
    let json = base.with_extension("json");
    let res = if out.status.success() && json.exists() {
        load_transcript(&json)
    } else {
        Err(anyhow!("whisper-cli failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")))
    };
    let _ = std::fs::remove_dir_all(&dir);
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(t: &str, s: f64, e: f64) -> Word {
        Word { text: t.into(), start_ms: s, end_ms: e }
    }

    #[test]
    fn tidy_sorts_and_clamps() {
        let mut v = vec![w("b", 500.0, 900.0), w("a", 0.0, 600.0), w(" ", 1.0, 2.0), w("c", 800.0, 700.0)];
        tidy(&mut v);
        let t: Vec<_> = v.iter().map(|x| (x.text.as_str(), x.start_ms, x.end_ms)).collect();
        assert_eq!(t, vec![("a", 0.0, 500.0), ("b", 500.0, 800.0), ("c", 800.0, 800.0)]);
    }

    #[test]
    fn whisper_tokens_merge() {
        let v: Value = serde_json::from_str(
            r#"{"transcription":[{"tokens":[
              {"text":"[_BEG_]","offsets":{"from":0,"to":0}},
              {"text":" I","offsets":{"from":400,"to":450}},
              {"text":"'m","offsets":{"from":450,"to":560}},
              {"text":" walking","offsets":{"from":560,"to":900}},
              {"text":".","offsets":{"from":900,"to":950}},
              {"text":"[_TT_50]","offsets":{"from":1000,"to":1000}}]},
              {"tokens":[{"text":" Um","offsets":{"from":1100,"to":1200}},{"text":",","offsets":{"from":1200,"to":1300}}]}]}"#,
        )
        .unwrap();
        let words = parse_transcript(&v).unwrap();
        let t: Vec<_> = words.iter().map(|x| (x.text.as_str(), x.start_ms, x.end_ms)).collect();
        assert_eq!(t, vec![("I'm", 400.0, 560.0), ("walking.", 560.0, 900.0), ("Um,", 1100.0, 1200.0)]);
    }

    #[test]
    fn simple_and_parakeet_shapes() {
        let a: Value = serde_json::from_str(r#"[{"text":"hi","start":0.5,"end":0.75}]"#).unwrap();
        assert_eq!(parse_transcript(&a).unwrap(), vec![w("hi", 500.0, 750.0)]);
        let b: Value = serde_json::from_str(r#"{"kind":"Success","transcription":{"segments":[{"tokens":[{"text":"yo","start":1,"end":1.5}]}]}}"#).unwrap();
        assert_eq!(parse_transcript(&b).unwrap(), vec![w("yo", 1000.0, 1500.0)]);
    }
}
