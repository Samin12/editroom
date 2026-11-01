//! One `claude -p` call: text in, text out, with token and cost accounting. Calls that look at
//! images send them inline (stream-json input), so the model sees the frame in the same turn.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
    pub fn add(&mut self, o: &Usage) {
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

pub struct Llm {
    pub system: String,
    pub timeout: Duration,
    /// The Claude Code binary (`$CUTROOM_CLAUDE`, default `claude`).
    pub bin: String,
}

impl Llm {
    pub fn new(system: String) -> Llm {
        Llm { system, timeout: Duration::from_secs(900), bin: std::env::var("CUTROOM_CLAUDE").unwrap_or_else(|_| "claude".into()) }
    }

    /// Ask once, retrying one time after a failure (overload, a dropped connection).
    pub fn ask(&self, model: &str, prompt: &str, images: &[PathBuf], log: &Path) -> Result<(String, Usage)> {
        match self.once(model, prompt, images, log) {
            Ok(x) => Ok(x),
            Err(e) => {
                eprintln!("  retrying {} after: {}", log.file_name().and_then(|n| n.to_str()).unwrap_or("call"), first_line(&e.to_string()));
                std::thread::sleep(Duration::from_secs(4));
                self.once(model, prompt, images, log)
            }
        }
    }

    fn once(&self, model: &str, prompt: &str, images: &[PathBuf], log: &Path) -> Result<(String, Usage)> {
        let mut args: Vec<String> = ["-p", "--model", model, "--tools", "", "--strict-mcp-config", "--no-session-persistence", "--system-prompt", &self.system].iter().map(|s| s.to_string()).collect();
        let input: Vec<u8> = if images.is_empty() {
            args.extend(["--output-format", "json"].map(String::from));
            prompt.as_bytes().to_vec()
        } else {
            args.extend(["--input-format", "stream-json", "--output-format", "stream-json", "--verbose"].map(String::from));
            let mut content = vec![];
            for img in images {
                let bytes = std::fs::read(img).with_context(|| format!("reading {}", img.display()))?;
                let mt = match img.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
                    Some("png") => "image/png",
                    Some("webp") => "image/webp",
                    _ => "image/jpeg",
                };
                content.push(serde_json::json!({"type": "image", "source": {"type": "base64", "media_type": mt, "data": base64(&bytes)}}));
            }
            content.push(serde_json::json!({"type": "text", "text": prompt}));
            let msg = serde_json::json!({"type": "user", "message": {"role": "user", "content": content}});
            let mut line = serde_json::to_vec(&msg)?;
            line.push(b'\n');
            line
        };
        let mut log_prompt = prompt.to_string();
        for img in images {
            log_prompt.push_str(&format!("\n[image] {}", img.display()));
        }
        std::fs::write(log.with_extension("prompt.txt"), log_prompt)?;

        let mut child = Command::new(&self.bin)
            .args(&args)
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("running `claude` (Claude Code CLI) - is it installed and signed in?")?;
        let mut stdin = child.stdin.take().unwrap();
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let mut so = child.stdout.take().unwrap();
        let mut se = child.stderr.take().unwrap();
        let ro = std::thread::spawn(move || {
            let mut b = vec![];
            let _ = so.read_to_end(&mut b);
            b
        });
        let re = std::thread::spawn(move || {
            let mut b = vec![];
            let _ = se.read_to_end(&mut b);
            b
        });
        let t0 = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait()? {
                break Some(s);
            }
            if t0.elapsed() > self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let _ = writer.join();
        let stdout = ro.join().unwrap_or_default();
        let stderr = re.join().unwrap_or_default();
        std::fs::write(log, &stdout)?;
        if status.is_none() {
            bail!("claude timed out after {}s", self.timeout.as_secs());
        }
        let v: Value = if images.is_empty() {
            serde_json::from_slice(&stdout).map_err(|_| anyhow!("claude failed: {}", String::from_utf8_lossy(&stderr).chars().take(400).collect::<String>()))?
        } else {
            String::from_utf8_lossy(&stdout)
                .lines()
                .rev()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .find(|v| v["type"] == "result")
                .ok_or_else(|| anyhow!("claude failed: {}", String::from_utf8_lossy(&stderr).chars().take(400).collect::<String>()))?
        };
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
            wall_s: t0.elapsed().as_secs_f64(),
        };
        Ok((strip_fences(v["result"].as_str().unwrap_or("")), usage))
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(200).collect()
}

/// Drop a ```yaml fence around an answer (models add one now and then).
pub fn strip_fences(s: &str) -> String {
    let t = s.trim();
    // a fenced answer, or prose around a ```yaml block: take the block's body (a bare ``` inside
    // an unfenced answer is content, e.g. in a code beat)
    let i = if t.starts_with("```") { Some(0) } else { t.find("```yaml").or_else(|| t.find("```yml")) };
    if let Some(i) = i {
        let rest = &t[i + 3..];
        let rest = rest.strip_prefix("yaml").or_else(|| rest.strip_prefix("yml")).unwrap_or(rest);
        if let Some(j) = rest.find("```") {
            return rest[..j].trim().to_string() + "\n";
        }
        return rest.trim().to_string() + "\n";
    }
    t.to_string() + "\n"
}

/// Standard base64 (RFC 4648, padded).
pub fn base64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    s
}

/// Run `f` over `items` on up to `jobs` threads; results keep the input order.
pub fn par_map<T: Send, R: Send>(items: Vec<T>, jobs: usize, f: impl Fn(T) -> R + Sync) -> Vec<R> {
    let n = items.len();
    let queue = std::sync::Mutex::new(items.into_iter().enumerate());
    let out: std::sync::Mutex<Vec<Option<R>>> = std::sync::Mutex::new((0..n).map(|_| None).collect());
    std::thread::scope(|sc| {
        for _ in 0..jobs.clamp(1, n.max(1)) {
            sc.spawn(|| loop {
                let next = queue.lock().unwrap().next();
                let Some((i, t)) = next else { break };
                let r = f(t);
                out.lock().unwrap()[i] = Some(r);
            });
        }
    });
    out.into_inner().unwrap().into_iter().map(|r| r.expect("worker finished")).collect()
}
