//! cutroom: an agentic video editor engine.
//!
//!   cutroom ingest borumi <bundle> <dir> [--timeline]   raw Borumi recording -> <dir>/source.json + edit.yaml
//!   cutroom ingest files <dir> --camera c.mp4 --screen s.mov [--mic m.wav] [--transcript t.json]
//!   cutroom cut <dir>                                   retakes, false starts, fillers, silences -> cuts.yaml + review.md
//!   cutroom brief <dir> [--from w100 --to w900]         compact kept transcript for an agent ("w123 sentence...")
//!   cutroom privacy <dir>                               OCR the screen for secrets -> privacy.json + beats/privacy.yaml
//!   cutroom check <dir>                                 lint the whole edit without rendering
//!   cutroom still <dir> --at <anchor> -o f.png          one frame
//!   cutroom sheet <dir> [--beats|--every 60] -o f.png   contact sheet (one image to review many frames)
//!   cutroom render <dir> -o out.mp4                     parallel render + chapters.txt + captions.srt
//!   cutroom stats <dir>                                 sizes, durations, token footprint

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use cutroom_core::anchor::{self, Index};
use cutroom_core::compose::segments;
use cutroom_core::lint;
use cutroom_core::model::Source;
use cutroom_core::resolve::{self, clock, Plan};
use cutroom_core::spec::{self, Anchor, Loaded};
use std::path::{Path, PathBuf};
use std::time::Instant;

mod agent;

#[derive(Parser)]
#[command(name = "cutroom", version, about = "Agentic video editor engine: word-anchored edits, native motion graphics, parallel render")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Read a recording into <dir>/source.json and start <dir>/edit.yaml.
    Ingest {
        #[command(subcommand)]
        from: IngestFrom,
    },
    /// Detect retakes, false starts, fillers and long silences; write cuts.yaml and review.md.
    Cut {
        dir: PathBuf,
        /// Accept review items too: "1,3,7-9" (numbers from review.md) or "all".
        #[arg(long)]
        accept: Option<String>,
    },
    /// Print the kept transcript compactly, one sentence per line, prefixed by its first word id.
    Brief {
        dir: PathBuf,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        /// Show cut words too (the raw transcript).
        #[arg(long)]
        raw: bool,
        /// Max words per line.
        #[arg(long, default_value_t = 40)]
        width: usize,
    },
    /// Scan screen recordings for secrets; write privacy.json and blur beats in beats/privacy.yaml.
    Privacy {
        dir: PathBuf,
        /// Sample every N ms of source.
        #[arg(long, default_value_t = 1000.0)]
        every: f64,
        /// Values that may stay visible: an exact email/value, "@domain", or "/regex/" (repeatable).
        #[arg(long)]
        allow: Vec<String>,
    },
    /// Lint the edit: anchors, overlaps, empty panels, privacy, missing files.
    Check {
        dir: PathBuf,
        /// Print info lines too.
        #[arg(long, short)]
        verbose: bool,
    },
    /// Print the resolved plan (output times) as JSON.
    Plan { dir: PathBuf },
    /// Render one frame to PNG.
    Still {
        dir: PathBuf,
        /// Anchor: w123, "a phrase", o:61.5 (output s), s:754.2 (source s).
        #[arg(long)]
        at: String,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Contact sheet: many frames in one image.
    Sheet {
        dir: PathBuf,
        /// One frame per beat (its middle) and per chapter card.
        #[arg(long)]
        beats: bool,
        /// One frame every N seconds of output.
        #[arg(long)]
        every: Option<f64>,
        /// Explicit anchors, comma separated.
        #[arg(long)]
        at: Option<String>,
        #[arg(long, default_value_t = 5)]
        cols: u32,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Render the video (and chapters.txt, captions.srt next to it).
    Render {
        dir: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
        /// Parallel chunks (default: cores / 4, max 12).
        #[arg(long)]
        jobs: Option<usize>,
        /// h264 (VideoToolbox, default), hevc, x264.
        #[arg(long, default_value = "h264")]
        codec: String,
        /// Output height override (1080, 1440, 2160).
        #[arg(long)]
        height: Option<u32>,
        /// Render only from this anchor (e.g. o:120).
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        no_audio: bool,
        /// Normalize loudness to -14 LUFS (YouTube).
        #[arg(long)]
        loudnorm: bool,
        /// Gain of system audio mixed under the mic (0 = mute).
        #[arg(long, default_value_t = 0.6)]
        system_gain: f32,
        /// Render even when check finds errors.
        #[arg(long)]
        force: bool,
        /// Share of workers decoding 5K+ screen recordings on the VideoToolbox engines (0..1).
        #[arg(long, default_value_t = 0.0)]
        hw: f64,
    },
    /// Durations, counts and the token footprint of the edit.
    Stats { dir: PathBuf },
    /// Editorial passes through Claude Code (`claude -p`): cut review, chapters, beats per chapter in parallel.
    Agent {
        dir: PathBuf,
        /// Model alias for `claude --model` (sonnet, opus, haiku).
        #[arg(long, default_value = "sonnet")]
        model: String,
        /// Chapters written in parallel.
        #[arg(long, default_value_t = 8)]
        jobs: usize,
        /// Skip the cut review (keep only confident cuts).
        #[arg(long)]
        no_review: bool,
        /// Only the cut review.
        #[arg(long)]
        cuts_only: bool,
    },
}

#[derive(Subcommand)]
enum IngestFrom {
    /// A Borumi project bundle (read-only: the database is copied first).
    Borumi {
        bundle: PathBuf,
        dir: PathBuf,
        /// What is on the Borumi timeline now, instead of the raw takes.
        #[arg(long)]
        timeline: bool,
        /// Only these scenes, 1-based inclusive ("3-5" or "4").
        #[arg(long)]
        scenes: Option<String>,
    },
    /// Plain files: camera and/or screen video, optional mic and transcript.
    Files {
        dir: PathBuf,
        #[arg(long)]
        camera: Option<PathBuf>,
        #[arg(long)]
        screen: Option<PathBuf>,
        #[arg(long)]
        mic: Option<PathBuf>,
        /// Word JSON: [{"text","start","end"}] in seconds. Without it whisper-cli transcribes.
        #[arg(long)]
        transcript: Option<PathBuf>,
    },
}

/// `<dir>` or `<dir>/edit.yaml`.
fn spec_path(p: &Path) -> PathBuf {
    if p.is_dir() {
        p.join("edit.yaml")
    } else {
        p.to_path_buf()
    }
}

struct Project {
    loaded: Loaded,
    src: Source,
    plan: Plan,
}

fn open(dir: &Path, height: Option<u32>) -> Result<Project> {
    let sp = spec_path(dir);
    let mut loaded = spec::load(&sp)?;
    if let Some(h) = height {
        loaded.spec.output.height = h;
    }
    let src = Source::load(&loaded.source_path).with_context(|| format!("loading {}", loaded.source_path.display()))?;
    let plan = resolve::build(&loaded, &src);
    Ok(Project { loaded, src, plan })
}

fn findings(dir: &Path) -> Vec<lint::Finding> {
    let p = if dir.is_dir() { dir.join("privacy.json") } else { dir.parent().unwrap_or(Path::new(".")).join("privacy.json") };
    std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Output ms of an anchor.
fn out_time(p: &Project, a: &str) -> Result<f64> {
    let idx = Index::new(&p.src);
    let kept: std::collections::HashSet<usize> = p.plan.words.iter().map(|w| w.id).collect();
    let k = |i: usize| kept.contains(&i);
    let r = anchor::resolve(&Anchor(a.into()), &p.src, &idx, (0, p.src.words.len()), &k)?;
    let base = if let Some(o) = r.out_ms {
        o
    } else if let Some(w) = r.word {
        p.plan.map.to_out(p.src.words[w].start_ms)
    } else {
        p.plan.map.to_out(r.src_ms.unwrap_or(0.0))
    };
    Ok(base + r.offset_ms)
}

fn word_index(p: &Project, a: &str) -> Result<usize> {
    let idx = Index::new(&p.src);
    let all = |_: usize| true;
    let r = anchor::resolve(&Anchor(a.into()), &p.src, &idx, (0, p.src.words.len()), &all)?;
    r.word.or_else(|| r.src_ms.map(|s| p.src.word_at_or_after(s))).ok_or_else(|| anyhow!("{a} is not a word"))
}

const EDIT_TEMPLATE: &str = "# cutroom edit. Anchors: w123 (word id from `cutroom brief`), \"a phrase\" (#2 for the 2nd match),\n# s:754.2 (source seconds), o:61.5 (output seconds). Visuals land 130 ms before their word.\nsource: source.json\ninclude: [cuts.yaml, beats/*.yaml]\noutput: {height: 1080, fps: 30, chapter_cards: insert, card_ms: 1800}\nlayout: {default: panel}\nchapters: []\nbeats: []\n";

fn write_new_project(dir: &Path, src: &Source) -> Result<()> {
    std::fs::create_dir_all(dir.join("beats"))?;
    src.save(&dir.join("source.json"))?;
    let edit = dir.join("edit.yaml");
    if !edit.exists() {
        std::fs::write(&edit, EDIT_TEMPLATE)?;
    }
    if !dir.join("cuts.yaml").exists() {
        std::fs::write(dir.join("cuts.yaml"), "cuts: []\n")?;
    }
    println!(
        "{}: {} words, {} clips, {} scenes, {}",
        dir.display(),
        src.words.len(),
        src.clips.len(),
        src.scenes.len(),
        clock(src.duration_ms)
    );
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ingest { from } => {
            let t = Instant::now();
            let (dir, src) = match from {
                IngestFrom::Borumi { bundle, dir, timeline, scenes } => {
                    let mode = if timeline { cutroom_ingest::BorumiMode::Timeline } else { cutroom_ingest::BorumiMode::Raw };
                    let mut src = cutroom_ingest::borumi(&bundle, mode)?;
                    if let Some(r) = scenes {
                        let (a, b) = r.split_once('-').unwrap_or((&r, &r));
                        let (a, b): (usize, usize) = (a.trim().parse()?, b.trim().parse()?);
                        if a == 0 || b < a {
                            bail!("--scenes is 1-based: 3-5");
                        }
                        src = src.slice_scenes(a - 1, b - 1);
                    }
                    (dir, src)
                }
                IngestFrom::Files { dir, camera, screen, mic, transcript } => (dir, cutroom_ingest::files(camera.as_deref(), screen.as_deref(), mic.as_deref(), transcript.as_deref())?),
            };
            let bad = src.validate();
            if !bad.is_empty() {
                bail!("source is invalid: {}", bad.join("; "));
            }
            write_new_project(&dir, &src)?;
            eprintln!("ingest {:.2}s", t.elapsed().as_secs_f64());
        }
        Cmd::Cut { dir, accept } => {
            let t = Instant::now();
            let sp = spec_path(&dir);
            let loaded = spec::load(&sp)?;
            let src = Source::load(&loaded.source_path)?;
            let mut report = cutroom_cut::detect(&src, &cutroom_cut::CutOpts::default());
            let base = sp.parent().unwrap_or(Path::new("."));
            if let Some(a) = &accept {
                let idx = cutroom_cut::parse_accept(a, report.review.len())?;
                report = cutroom_cut::accept(&report, &idx);
                println!("accepted {} review items", idx.len());
            }
            let yaml = cutroom_cut::to_yaml(&report);
            std::fs::write(base.join("cuts.yaml"), yaml)?;
            let review = cutroom_cut::review_md(&report, &src);
            std::fs::write(base.join("review.md"), &review)?;
            println!("{}", cutroom_cut::summary(&report, &src));
            println!("wrote cuts.yaml, review.md ({} bytes) in {:.2}s", review.len(), t.elapsed().as_secs_f64());
        }
        Cmd::Brief { dir, from, to, raw, width } => {
            let p = open(&dir, None)?;
            let lo = match &from {
                Some(a) => word_index(&p, a)?,
                None => 0,
            };
            let hi = match &to {
                Some(a) => word_index(&p, a)? + 1,
                None => p.src.words.len(),
            };
            print!("{}", resolve::brief(&p.src, if raw { None } else { Some(&p.plan) }, lo, hi, width));
        }
        Cmd::Privacy { dir, every, allow } => {
            let t = Instant::now();
            let sp = spec_path(&dir);
            let loaded = spec::load(&sp)?;
            let src = Source::load(&loaded.source_path)?;
            let mut opts = cutroom_privacy::ScanOpts { every_ms: every, ..Default::default() };
            opts.allow.extend(allow);
            let f = cutroom_privacy::scan(&src, &opts)?;
            let base = sp.parent().unwrap_or(Path::new("."));
            cutroom_privacy::write_json(&base.join("privacy.json"), &f)?;
            std::fs::create_dir_all(base.join("beats"))?;
            std::fs::write(base.join("beats/privacy.yaml"), cutroom_privacy::propose_beats(&f))?;
            let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
            for x in &f {
                *kinds.entry(x.kind.as_str()).or_default() += 1;
            }
            println!("{} findings {:?}; blur beats in beats/privacy.yaml ({:.1}s)", f.len(), kinds, t.elapsed().as_secs_f64());
        }
        Cmd::Check { dir, verbose } => {
            let p = open(&dir, None)?;
            let segs = segments(&p.plan, &p.src);
            let issues = lint::check(&p.plan, &p.src, &segs, &p.loaded.dir, &findings(&dir));
            let shown: Vec<_> = issues.iter().filter(|i| verbose || i.sev != resolve::Severity::Info).cloned().collect();
            print!("{}", lint::format(&shown));
            let n = |s: resolve::Severity| issues.iter().filter(|i| i.sev == s).count();
            println!(
                "{} errors, {} warnings, {} info | output {} ({} cut), {} beats, {} chapters",
                n(resolve::Severity::Error),
                n(resolve::Severity::Warn),
                n(resolve::Severity::Info),
                clock(p.plan.duration_ms()),
                clock(p.src.duration_ms - p.plan.map.kept_ms()),
                p.plan.beats.len(),
                p.plan.chapters.len()
            );
            if n(resolve::Severity::Error) > 0 {
                std::process::exit(1);
            }
        }
        Cmd::Plan { dir } => {
            let p = open(&dir, None)?;
            println!("{}", serde_json::to_string_pretty(&p.plan)?);
        }
        Cmd::Still { dir, at, out } => {
            let p = open(&dir, None)?;
            let t = out_time(&p, &at)?;
            let segs = segments(&p.plan, &p.src);
            let pm = cutroom_render::still(&p.plan, &p.src, &segs, &p.loaded.dir, t)?;
            cutroom_render::save_png(&pm, &out)?;
            println!("{} at {}", out.display(), clock(t));
        }
        Cmd::Sheet { dir, beats, every, at, cols, out } => {
            let p = open(&dir, None)?;
            let mut times = vec![];
            if beats {
                for b in &p.plan.beats {
                    times.push((b.start_ms + b.end_ms) / 2.0);
                }
                for c in &p.plan.chapters {
                    if let Some((a, b)) = c.card {
                        times.push((a + b) / 2.0);
                    }
                }
            }
            if let Some(e) = every {
                let mut t = 0.0;
                while t < p.plan.duration_ms() {
                    times.push(t);
                    t += e * 1000.0;
                }
            }
            if let Some(a) = at {
                for x in a.split(',') {
                    times.push(out_time(&p, x.trim())?);
                }
            }
            if times.is_empty() {
                bail!("give --beats, --every or --at");
            }
            times.sort_by(|a, b| a.total_cmp(b));
            let segs = segments(&p.plan, &p.src);
            let pm = cutroom_render::sheet(&p.plan, &p.src, &segs, &p.loaded.dir, &times, cols, 384)?;
            cutroom_render::save_png(&pm, &out)?;
            println!("{} ({} frames: {})", out.display(), times.len(), times.iter().take(40).map(|t| clock(*t)).collect::<Vec<_>>().join(" "));
        }
        Cmd::Render { dir, out, jobs, codec, height, from, to, no_audio, loudnorm, system_gain, force, hw } => {
            let p = open(&dir, height)?;
            let segs = segments(&p.plan, &p.src);
            let issues = lint::check(&p.plan, &p.src, &segs, &p.loaded.dir, &findings(&dir));
            let errors: Vec<_> = issues.iter().filter(|i| i.sev == resolve::Severity::Error).cloned().collect();
            if !errors.is_empty() && !force {
                print!("{}", lint::format(&errors));
                bail!("{} errors: fix them (cutroom check) or pass --force", errors.len());
            }
            let range = match (&from, &to) {
                (None, None) => None,
                _ => Some((from.as_deref().map(|a| out_time(&p, a)).transpose()?.unwrap_or(0.0), to.as_deref().map(|a| out_time(&p, a)).transpose()?.unwrap_or(p.plan.duration_ms()))),
            };
            let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
            let opts = cutroom_render::RenderOpts {
                out: out.clone(),
                jobs: jobs.unwrap_or((cores / 4).clamp(1, 12)),
                codec: codec.parse()?,
                range,
                audio: !no_audio,
                loudnorm,
                system_gain,
                work: None,
                quiet: false,
                hw_share: hw,
            };
            let r = cutroom_render::render(&p.plan, &p.src, &segs, &p.loaded.dir, &opts)?;
            let stem = out.with_extension("");
            std::fs::write(format!("{}.chapters.txt", stem.display()), cutroom_render::outputs::chapters(&p.plan))?;
            std::fs::write(format!("{}.srt", stem.display()), cutroom_render::outputs::srt(&p.plan))?;
            println!(
                "{}: {} ({} frames) in {:.1}s = {:.0} fps, {:.1}x realtime, {} jobs | compose {:.2} ms/f, decode wait {:.2} ms/f, {} decoders",
                out.display(),
                clock(r.output_s * 1000.0),
                r.frames,
                r.wall_s,
                r.fps,
                r.realtime_x,
                r.jobs,
                r.compose_ms_per_frame,
                r.decode_wait_ms_per_frame,
                r.feeds_opened
            );
            std::fs::write(format!("{}.report.json", stem.display()), serde_json::to_string_pretty(&r)?)?;
        }
        Cmd::Agent { dir, model, jobs, no_review, cuts_only } => {
            let u = agent::run(&dir, &agent::AgentOpts { model, jobs, review: !no_review, beats: !cuts_only })?;
            println!(
                "agent: {} calls, {} input tokens ({} cached), {} output tokens, ${:.3}, {:.0}s",
                u.calls,
                u.total_in(),
                u.cache_read,
                u.output,
                u.cost_usd,
                u.wall_s
            );
        }
        Cmd::Stats { dir } => {
            let p = open(&dir, None)?;
            let mut bytes = 0u64;
            let mut files = vec![spec_path(&dir)];
            for inc in &p.loaded.spec.include {
                if let Some(folder) = inc.strip_suffix("/*.yaml") {
                    if let Ok(rd) = std::fs::read_dir(p.loaded.dir.join(folder)) {
                        files.extend(rd.filter_map(|e| e.ok().map(|e| e.path())));
                    }
                } else {
                    files.push(p.loaded.dir.join(inc));
                }
            }
            let mut agent_bytes = 0u64;
            for f in &files {
                let n = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
                bytes += n;
                if !f.ends_with("cuts.yaml") && !f.ends_with("privacy.yaml") {
                    agent_bytes += n;
                }
            }
            let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
            for b in &p.plan.beats {
                *kinds.entry(b.kind.name()).or_default() += 1;
            }
            println!("source   {} ({} words)", clock(p.src.duration_ms), p.src.words.len());
            println!("output   {} ({} removed, {} cut ranges)", clock(p.plan.duration_ms()), clock(p.src.duration_ms - p.plan.map.kept_ms()), p.plan.cuts.len());
            println!("beats    {} {:?}", p.plan.beats.len(), kinds);
            println!("chapters {}", p.plan.chapters.len());
            println!("spec     {} bytes in {} files (~{} tokens); agent-written {} bytes (~{} tokens, {:.0} per beat)", bytes, files.len(), bytes / 4, agent_bytes, agent_bytes / 4, agent_bytes as f64 / 4.0 / p.plan.beats.len().max(1) as f64);
        }
    }
    Ok(())
}
