//! `cutroom agent`: the editorial passes, headless, through Claude Code (`claude -p`).
//!
//! The engine does everything deterministic; the model only answers three small questions:
//! 1. which ambiguous cuts to take (review.md -> "accept: 1,3,7-9")
//! 2. where the chapters are (whole kept transcript -> chapters YAML)
//! 3. which visuals each chapter gets (one call per chapter, in parallel -> beats YAML)
//!
//! Each answer is validated by `check`; a file with errors gets one repair round.

use anyhow::{anyhow, bail, Context, Result};
use cutroom_core::compose::segments;
use cutroom_core::lint;
use cutroom_core::model::Source;
use cutroom_core::resolve::{self, Plan, Severity};
use cutroom_core::spec;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

pub struct AgentOpts {
    pub model: String,
    pub jobs: usize,
    pub review: bool,
    pub beats: bool,
}

#[derive(Default, Debug, Clone, serde::Serialize)]
pub struct Usage {
    pub calls: u64,
    pub input: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub output: u64,
    pub cost_usd: f64,
    pub wall_s: f64,
}

impl Usage {
    fn add(&mut self, o: &Usage) {
        self.calls += o.calls;
        self.input += o.input;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.output += o.output;
        self.cost_usd += o.cost_usd;
    }
    pub fn total_in(&self) -> u64 {
        self.input + self.cache_read + self.cache_write
    }
}

const SYSTEM: &str = "You are the editor of Samin Yasar's YouTube videos (AI + YouTube growth tutorials, a talking head with screen recordings). You answer with exactly the requested format and nothing else: no prose, no code fences.";

/// One `claude -p` call; returns (text, usage). Prompts go in on stdin.
fn ask(model: &str, prompt: &str, log: &Path) -> Result<(String, Usage)> {
    let mut child = Command::new("claude")
        .args(["-p", "--model", model, "--output-format", "json", "--tools", "", "--strict-mcp-config", "--no-session-persistence", "--system-prompt", SYSTEM])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running `claude` (Claude Code CLI) - is it installed and signed in?")?;
    child.stdin.take().unwrap().write_all(prompt.as_bytes())?;
    let out = child.wait_with_output()?;
    std::fs::write(log.with_extension("prompt.txt"), prompt)?;
    std::fs::write(log, &out.stdout)?;
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|_| anyhow!("claude failed: {}", String::from_utf8_lossy(&out.stderr).chars().take(400).collect::<String>()))?;
    if v["is_error"].as_bool().unwrap_or(false) {
        bail!("claude error: {}", v["result"]);
    }
    let u = &v["usage"];
    let usage = Usage {
        calls: 1,
        input: u["input_tokens"].as_u64().unwrap_or(0),
        cache_read: u["cache_read_input_tokens"].as_u64().unwrap_or(0),
        cache_write: u["cache_creation_input_tokens"].as_u64().unwrap_or(0),
        output: u["output_tokens"].as_u64().unwrap_or(0),
        cost_usd: v["total_cost_usd"].as_f64().unwrap_or(0.0),
        wall_s: 0.0,
    };
    Ok((strip_fences(v["result"].as_str().unwrap_or("")), usage))
}

fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("```yaml").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string() + "\n"
}

fn load(dir: &Path) -> Result<(spec::Loaded, Source, Plan)> {
    let loaded = spec::load(&dir.join("edit.yaml"))?;
    let src = Source::load(&loaded.source_path)?;
    let plan = resolve::build(&loaded, &src);
    Ok((loaded, src, plan))
}

/// Kept transcript lines for words [lo, hi): "w123 [talk] sentence" ([screen] when the screen
/// recording is visible, [talk] when he is on camera only).
fn annotated_brief(src: &Source, plan: &Plan, lo: usize, hi: usize) -> String {
    let segs = segments(plan, src);
    let txt = resolve::brief(src, Some(plan), lo, hi, 45);
    let mut out = String::new();
    for line in txt.lines() {
        let (id, rest) = line.split_once(' ').unwrap_or((line, ""));
        let w: usize = id.trim_start_matches('w').parse().unwrap_or(0);
        let o = plan.map.to_out(src.words[w].start_ms);
        let screen = segs.iter().find(|s| s.out0 <= o && o < s.out1).map(|s| s.screen.is_some()).unwrap_or(false);
        out.push_str(&format!("{id} [{}] {rest}\n", if screen { "screen" } else { "talk" }));
    }
    out
}

const REVIEW_PROMPT: &str = "These are ambiguous cut candidates from a raw recording. [[...]] marks the proposed cut: an earlier attempt, a stutter or a restart; he always keeps the LAST attempt. Cut it when the bracketed words are said again right after (a retake), are a false start, or are a broken half sentence. Keep it when cutting would drop meaning that is not repeated, or the repetition is intentional emphasis.\n\nAnswer with ONE line: `accept: <numbers>` listing the items to cut, as comma-separated numbers and ranges (e.g. `accept: 1,3,7-9`), or `accept: none`.\n\n";

const CHAPTER_PROMPT: &str = "Split this video into YouTube chapters (one per real topic change; roughly every 3 to 12 minutes; the first chapter starts at the first line). Titles: 2 to 5 words, his own wording, no em dashes, no numbering. `short`: 1 to 2 words for an on-screen badge.\n\nAnswer with YAML only, in exactly this shape (`at` = the word id that starts the chapter's first line):\nchapters:\n  - {at: w0, title: Why YouTube matters, short: Why}\n\nTranscript (word id, [talk] on camera or [screen] demo, sentence):\n";

const BEATS_PROMPT: &str = r#"Add motion graphics to this chapter. Each visual is one YAML line anchored to a word id:
- {at: wN, kind: title, text: "...", kicker: "..."}                  a headline for a key idea
- {at: wN, kind: bullets, title: "...", items: [{text: "...", at: wN}, ...]}  a list he walks through (each item appears at its word)
- {at: wN, kind: number, value: "...", label: "..."}               a number he says ("100K", "$2,000", "3x")
- {at: wN, kind: code, title: "...", text: "..."}                   a prompt or command he reads out
- {at: wN, kind: quote, text: "...", by: "..."}                     a quote he cites
- {at: wN, kind: callout, text: "..."}                              a 2 to 4 word pill (the only kind allowed on [screen] lines)
Optional on any line: `until: wM` (stays until word M ends) or `hold: 3s`.
Rules: title, bullets, number, code and quote only on [talk] lines (they fill the panel next to his camera). One idea per beat, 3 to 10 s on screen, never overlapping, at most one beat per ~20 s, none while he is just chatting. Use his words; never invent numbers or facts he did not say. Headlines under 7 words, no em dashes. Fewer, stronger beats beat many.

Answer with YAML only, exactly:
beats:
  - {at: w123, kind: title, text: "..."}

Chapter transcript (word id, [talk] on camera or [screen] demo, sentence):
"#;

fn parse_accept(s: &str) -> Option<String> {
    let l = s.lines().find(|l| l.trim_start().starts_with("accept:"))?;
    let v = l.trim_start().trim_start_matches("accept:").trim();
    if v.eq_ignore_ascii_case("none") || v.is_empty() {
        Some(String::new())
    } else {
        Some(v.to_string())
    }
}

/// Issues of one spec file (beats/chapters it defines), checked in the context of the whole edit.
fn file_errors(dir: &Path, file: &str) -> Result<Vec<String>> {
    let (loaded, src, plan) = match load(dir) {
        Ok(x) => x,
        Err(e) => return Ok(if e.to_string().contains(file) { vec![e.to_string()] } else { vec![] }),
    };
    let segs = segments(&plan, &src);
    let issues = lint::check(&plan, &src, &segs, &loaded.dir, &[]);
    Ok(issues.into_iter().filter(|i| i.sev == Severity::Error && i.msg.contains(file)).map(|i| i.msg).collect())
}

pub fn run(dir: &Path, o: &AgentOpts) -> Result<Usage> {
    let t0 = Instant::now();
    let log = dir.join("agent");
    std::fs::create_dir_all(&log)?;
    std::fs::create_dir_all(dir.join("beats"))?;
    let mut total = Usage::default();

    // ---- 1. cut review ----
    let (_, src, _) = load(dir)?;
    let mut report = cutroom_cut::detect(&src, &cutroom_cut::CutOpts::default());
    if o.review && !report.review.is_empty() {
        let review = cutroom_cut::review_md(&report, &src);
        let (ans, u) = ask(&o.model, &format!("{REVIEW_PROMPT}{review}"), &log.join("review.json"))?;
        total.add(&u);
        let acc = parse_accept(&ans).ok_or_else(|| anyhow!("review answer has no `accept:` line: {ans}"))?;
        let idx = if acc.is_empty() { vec![] } else { cutroom_cut::parse_accept(&acc, report.review.len())? };
        eprintln!("review: accepted {} of {} candidates", idx.len(), report.review.len());
        report = cutroom_cut::accept(&report, &idx);
    }
    std::fs::write(dir.join("cuts.yaml"), cutroom_cut::to_yaml(&report))?;
    if !o.beats {
        total.wall_s = t0.elapsed().as_secs_f64();
        return Ok(total);
    }

    // ---- 2. chapters ----
    let (_, src, plan) = load(dir)?;
    let brief = annotated_brief(&src, &plan, 0, src.words.len());
    let chapters_file = "beats/00-chapters.yaml";
    let (ans, u) = ask(&o.model, &format!("{CHAPTER_PROMPT}{brief}"), &log.join("chapters.json"))?;
    total.add(&u);
    std::fs::write(dir.join(chapters_file), &ans)?;
    let errs = file_errors(dir, chapters_file)?;
    if !errs.is_empty() {
        let (ans2, u) = ask(&o.model, &format!("{CHAPTER_PROMPT}{brief}\nYour previous answer:\n{ans}\nIt has these errors; answer again, fixed:\n{}\n", errs.join("\n")), &log.join("chapters-fix.json"))?;
        total.add(&u);
        std::fs::write(dir.join(chapters_file), &ans2)?;
    }
    let (_, src, plan) = load(dir)?;
    eprintln!("chapters: {}", plan.chapters.len());

    // ---- 3. beats per chapter, in parallel ----
    let s: spec::Spec = spec::read_spec(&dir.join(chapters_file))?;
    let idx = cutroom_core::anchor::Index::new(&src);
    let all = |_: usize| true;
    let mut starts: Vec<usize> = s
        .chapters
        .iter()
        .filter_map(|c| cutroom_core::anchor::resolve(&c.at, &src, &idx, (0, src.words.len()), &all).ok().and_then(|r| r.word))
        .collect();
    starts.sort_unstable();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }
    let ranges: Vec<(usize, usize)> = starts.iter().enumerate().map(|(i, &a)| (a, starts.get(i + 1).copied().unwrap_or(src.words.len()))).collect();
    let jobs: Vec<(usize, usize, usize, String)> = ranges.iter().enumerate().map(|(i, &(a, b))| (i, a, b, annotated_brief(&src, &plan, a, b))).collect();
    let queue = std::sync::Mutex::new(jobs.into_iter());
    let total_res: std::sync::Mutex<Vec<Result<Usage>>> = std::sync::Mutex::new(vec![]);
    std::thread::scope(|sc| {
        let mut handles = vec![];
        for _ in 0..o.jobs.max(1) {
            handles.push(sc.spawn(|| loop {
                let job = queue.lock().unwrap().next();
                let Some((i, a, b, text)) = job else { break };
                let r = (|| -> Result<Usage> {
                    let file = format!("beats/ch{:02}.yaml", i + 1);
                    let name = log.join(format!("ch{:02}.json", i + 1));
                    let (ans, mut u) = ask(&o.model, &format!("{BEATS_PROMPT}{text}"), &name)?;
                    let body = format!("scope: [w{a}, w{}]\n{ans}", b.saturating_sub(1));
                    put(&dir.join(&file), &body)?;
                    // validate this file alone against the plan
                    let errs = file_errors(dir, &file)?;
                    if !errs.is_empty() {
                        let (ans2, u2) = ask(&o.model, &format!("{BEATS_PROMPT}{text}\nYour previous answer:\n{ans}\nIt has these errors; answer again, fixed:\n{}\n", errs.join("\n")), &name.with_file_name(format!("ch{:02}-fix.json", i + 1)))?;
                        u.add(&u2);
                        put(&dir.join(&file), &format!("scope: [w{a}, w{}]\n{ans2}", b.saturating_sub(1)))?;
                        let left = file_errors(dir, &file)?;
                        if !left.is_empty() {
                            // drop the beats that still fail rather than block the render
                            eprintln!("{file}: {} errors left after repair; see `cutroom check`", left.len());
                        }
                    }
                    Ok(u)
                })();
                total_res.lock().unwrap().push(r);
            }));
        }
        for h in handles {
            let _ = h.join();
        }
    });
    let results = total_res.into_inner().unwrap();
    for r in results {
        match r {
            Ok(u) => total.add(&u),
            Err(e) => eprintln!("beats: {e}"),
        }
    }
    total.wall_s = t0.elapsed().as_secs_f64();
    std::fs::write(log.join("usage.json"), serde_json::to_string_pretty(&total)?)?;
    Ok(total)
}

/// Write atomically so parallel workers never read a half-written file.
fn put(path: &Path, body: &str) -> Result<()> {
    let tmp: PathBuf = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
