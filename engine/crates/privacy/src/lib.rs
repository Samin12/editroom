//! cutroom-privacy: find secrets on the screen recording and propose blur beats.
//!
//! [`scan`] samples every screen clip (default one frame per second of source time), skips frames
//! that did not change since the last OCR'd one, OCRs only the full-width bands that changed (with
//! tesseract, one process per core), runs the detectors in [`detect`] on the frame's words, and
//! merges sightings of the same thing across consecutive samples into one [`Finding`] in SOURCE
//! time. [`propose_beats`] turns findings into `kind: blur` beats anchored to source time (`s:`),
//! so they survive any later cut; `cutroom check` refuses a plan that shows a finding unblurred.
//!
//! Values are never kept: findings carry a redacted preview (`sk-ant...`, `ja***@e***`), and frames
//! go to tesseract over a pipe, never to disk.

mod detect;
mod frames;
mod ocr;

pub use cutroom_core::lint::Finding;
pub use detect::{redact, Pattern};

use anyhow::{anyhow, Context, Result};
use cutroom_core::model::{Source, Track};
use detect::{iou, union, Detectors, Obs};
use frames::{Chunk, Frame};
use ocr::{Tesseract, Word};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct ScanOpts {
    /// Sample every N ms of source time.
    pub every_ms: f64,
    /// Padding before the first and after the last sighting, as a fraction of the gap to the
    /// neighbouring sample: 0.5 = half a sample each side; 1.0 covers the whole unseen gap.
    /// Used when `refine_edges` is off.
    pub pad: f64,
    /// Place each finding's start and end on the frame where its box changes (decodes the gap
    /// between the two samples around each edge, every frame). Off: pad by `pad`.
    pub refine_edges: bool,
    /// Values never reported: exact text (case-insensitive; emails tolerate one OCR error),
    /// `@domain` for a whole email domain, or `/regex/`.
    pub allow: Vec<String>,
    /// Extra detectors, run next to the built-in ones.
    pub extra_patterns: Vec<Pattern>,
    /// Kinds to leave out (`phone`, `inbox`, `finance`, ...).
    pub skip_kinds: Vec<String>,
    /// Only scan this source-time window (ms).
    pub window_ms: Option<(f64, f64)>,
    /// OCR resolution relative to the media. None: native for recordings >= 2560 px wide
    /// (Retina captures, where UI text is ~24 px tall), 2x below.
    pub ocr_scale: Option<f64>,
    /// Parallel OCR workers (0 = one per core).
    pub threads: usize,
    /// Sample keyframes only when they are at most 0.75 * `every_ms` apart (far cheaper to decode).
    pub keyframes: bool,
    /// Samples per ffmpeg process (0 = sized to the cores). Each chunk's first frame is OCR'd whole.
    pub chunk_samples: usize,
    /// Gray-level change (on a 4x downscaled frame) that marks a pixel as changed.
    pub diff_threshold: u8,
    /// Relative media paths resolve against this folder.
    pub media_root: Option<PathBuf>,
    pub ffmpeg: String,
    pub ffprobe: String,
    pub tesseract: String,
    pub lang: String,
    /// Tesseract page segmentation mode (11 = sparse text, best for UIs).
    pub psm: u32,
}

impl Default for ScanOpts {
    fn default() -> Self {
        ScanOpts {
            every_ms: 1000.0,
            pad: 0.5,
            refine_edges: true,
            allow: ["git@github.com", "noreply@anthropic.com", "@example.com", "@example.org", "@users.noreply.github.com"].map(String::from).to_vec(),
            extra_patterns: vec![],
            skip_kinds: vec![],
            window_ms: None,
            ocr_scale: None,
            threads: 0,
            keyframes: true,
            chunk_samples: 0,
            diff_threshold: 24,
            media_root: None,
            ffmpeg: "ffmpeg".into(),
            ffprobe: "ffprobe".into(),
            tesseract: "tesseract".into(),
            lang: "eng".into(),
            psm: 11,
        }
    }
}

/// What a scan did (counts only, never values).
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ScanStats {
    /// Screen media scanned (ms).
    pub media_ms: f64,
    pub frames_sampled: usize,
    /// Frames that needed any OCR (the rest were unchanged).
    pub frames_ocr: usize,
    /// Of those, frames OCR'd whole.
    pub frames_ocr_full: usize,
    /// OCR'd pixels / sampled pixels.
    pub ocr_area: f64,
    /// Summed over workers (ms).
    pub decode_ms: f64,
    pub ocr_ms: f64,
    pub detect_ms: f64,
    pub wall_ms: f64,
    pub workers: usize,
    /// Keyframe sampling used for this many clips.
    pub keyframe_clips: usize,
    /// Finding edges placed on the exact frame.
    pub edges_refined: usize,
    pub refine_ms: f64,
    pub per_kind: BTreeMap<String, usize>,
}

/// Scan every screen clip of `src` for secrets.
pub fn scan(src: &Source, opts: &ScanOpts) -> Result<Vec<Finding>> {
    scan_with_stats(src, opts).map(|(f, _)| f)
}

/// A clip as scanned: its media and where it sits.
struct Job {
    clip: usize,
    path: PathBuf,
    w: usize,
    h: usize,
    chunk: Chunk,
}

/// One sample: when, and what was seen.
struct Sample {
    t_ms: f64,
    obs: Vec<Obs>,
}

/// A scanned chunk: clip, samples, stats, OCR frame size.
type ChunkOut = (usize, Vec<Sample>, ChunkStats, usize, usize);

#[derive(Default)]
struct ChunkStats {
    sampled: usize,
    ocr: usize,
    full: usize,
    area: f64,
    decode_ms: f64,
    ocr_ms: f64,
    detect_ms: f64,
}

pub fn scan_with_stats(src: &Source, opts: &ScanOpts) -> Result<(Vec<Finding>, ScanStats)> {
    let t0 = Instant::now();
    let det = Detectors::new(&opts.extra_patterns, &opts.allow, &opts.skip_kinds)?;
    let tess = Tesseract { bin: opts.tesseract.clone(), lang: opts.lang.clone(), psm: opts.psm };
    let every = opts.every_ms.max(50.0);
    let mut stats = ScanStats::default();
    let workers = if opts.threads == 0 { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) } else { opts.threads };

    // plan: clips -> sample times -> chunks
    let mut jobs: Vec<Job> = vec![];
    let mut clip_range: BTreeMap<usize, (f64, f64)> = BTreeMap::new();
    let mut clip_media: BTreeMap<usize, (PathBuf, usize, usize)> = BTreeMap::new();
    let mut probes: BTreeMap<PathBuf, frames::Probe> = BTreeMap::new();
    for (ci, c) in src.clips.iter().enumerate() {
        if c.track != Track::Screen {
            continue;
        }
        let (mut s0, mut s1) = (c.start_ms, c.end_ms());
        if let Some((a, b)) = opts.window_ms {
            s0 = s0.max(a);
            s1 = s1.min(b);
        }
        if s1 - s0 < 1.0 {
            continue;
        }
        let media = src.media(&c.media).ok_or_else(|| anyhow!("clip references unknown media {}", c.media))?;
        let path = match (&opts.media_root, media.path.is_relative()) {
            (Some(root), true) => root.join(&media.path),
            _ => media.path.clone(),
        };
        if !probes.contains_key(&path) {
            probes.insert(path.clone(), frames::probe(&opts.ffprobe, &path)?);
        }
        let p = &probes[&path];
        let (m0, mut m1) = (c.media_at(s0), c.media_at(s1));
        if p.dur_ms > 0.0 {
            m1 = m1.min(p.dur_ms);
        }
        if m1 - m0 < 1.0 {
            continue;
        }
        let scale = opts.ocr_scale.unwrap_or(if p.w >= 2560 { 1.0 } else { 2.0 });
        let (w, h) = (((p.w as f64 * scale).round() as usize).max(16), ((p.h as f64 * scale).round() as usize).max(16));
        clip_range.insert(ci, (s0, s1));
        clip_media.insert(ci, (path.clone(), w, h));
        stats.media_ms += m1 - m0;
        let nominal: Vec<f64> = (0..).map(|k| m0 + k as f64 * every).take_while(|&t| t < m1).collect();
        // keyframe mode when keyframes are dense enough
        let mut keep: Option<Vec<f64>> = None;
        if opts.keyframes {
            let kf = frames::keyframes(&opts.ffprobe, &path, m0 - 2.0 * every, m1 + 2.0 * every)?;
            let inside: Vec<f64> = kf.iter().copied().filter(|&k| k >= m0 && k < m1).collect();
            let mut gaps = kf.windows(2).filter(|w| w[1] > m0 && w[0] < m1).map(|w| w[1] - w[0]);
            if !inside.is_empty() && gaps.all(|g| g <= 0.75 * every) {
                let mut pick: Vec<f64> = nominal
                    .iter()
                    .map(|&t| {
                        let i = inside.partition_point(|&k| k < t);
                        let a = inside.get(i.saturating_sub(1)).copied();
                        let b = inside.get(i).copied();
                        match (a, b) {
                            (Some(a), Some(b)) => if t - a <= b - t { a } else { b },
                            (Some(a), None) => a,
                            (None, Some(b)) => b,
                            (None, None) => t,
                        }
                    })
                    .collect();
                pick.dedup_by(|a, b| (*a - *b).abs() < 0.5);
                keep = Some(pick);
                stats.keyframe_clips += 1;
            }
        }
        // enough chunks to keep every core busy, but long enough that the forced whole-frame
        // OCR at each chunk start stays rare
        let n = if opts.chunk_samples > 0 { opts.chunk_samples.max(2) } else { (nominal.len() / workers).clamp(8, 40) };
        match keep {
            Some(pick) => {
                for part in pick.chunks(n) {
                    let a = part[0] - 1.0;
                    let len = part[part.len() - 1] + 2.0 - a;
                    jobs.push(Job { clip: ci, path: path.clone(), w, h, chunk: Chunk { a_ms: a, len_ms: len, keep: Some(part.to_vec()), every_ms: every, crop: None } });
                }
            }
            None => {
                for part in nominal.chunks(n) {
                    let a = part[0];
                    let len = (part[part.len() - 1] + every).min(m1) - a;
                    jobs.push(Job { clip: ci, path: path.clone(), w, h, chunk: Chunk { a_ms: a, len_ms: len.max(1.0), keep: None, every_ms: every, crop: None } });
                }
            }
        }
    }

    stats.workers = workers;
    let pool = rayon::ThreadPoolBuilder::new().num_threads(workers).build()?;
    let results: Vec<Result<ChunkOut>> = pool.install(|| {
        jobs.par_iter()
            .map(|j| {
                let (samples, st) = run_chunk(j, &det, &tess, &opts.ffmpeg, opts.diff_threshold).with_context(|| format!("scanning {} at {:.1}s", j.path.display(), j.chunk.a_ms / 1000.0))?;
                Ok((j.clip, samples, st, j.w, j.h))
            })
            .collect()
    });

    // per clip: samples in time order, in source time
    let mut per_clip: BTreeMap<usize, (Vec<Sample>, usize, usize)> = BTreeMap::new();
    for r in results {
        let (ci, samples, st, w, h) = r?;
        stats.frames_sampled += st.sampled;
        stats.frames_ocr += st.ocr;
        stats.frames_ocr_full += st.full;
        stats.ocr_area += st.area;
        stats.decode_ms += st.decode_ms;
        stats.ocr_ms += st.ocr_ms;
        stats.detect_ms += st.detect_ms;
        let c = &src.clips[ci];
        let e = per_clip.entry(ci).or_insert((vec![], w, h));
        e.0.extend(samples.into_iter().map(|s| Sample { t_ms: c.start_ms + (s.t_ms - c.media_in_ms), obs: s.obs }));
    }
    if stats.frames_sampled > 0 {
        stats.ocr_area /= stats.frames_sampled as f64;
    }
    let mut hits: Vec<Hit> = vec![];
    for (ci, (mut samples, w, h)) in per_clip {
        samples.sort_by(|a, b| a.t_ms.total_cmp(&b.t_ms));
        let (s0, s1) = clip_range[&ci];
        hits.extend(track(&samples, ci, s0, s1, opts.pad, w, h));
    }
    if opts.refine_edges {
        let tr = Instant::now();
        stats.edges_refined = refine_edges(src, &mut hits, &clip_media, &opts.ffmpeg, &pool)?;
        stats.refine_ms = tr.elapsed().as_secs_f64() * 1000.0;
    }
    let findings = merge_hits(hits);
    for f in &findings {
        *stats.per_kind.entry(f.kind.clone()).or_default() += 1;
    }
    stats.wall_ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok((findings, stats))
}

/// Tiles of the change map, in OCR pixels.
const TILE: usize = 64;
/// Downscale for the change map.
const DS: usize = 4;
/// A tile changed when this many of its downscaled pixels did (a typed character, not codec noise).
const TILE_PX: usize = 3;
/// Changed tiles this close (in tiles) form one region.
const JOIN: usize = 2;
/// Unchanged margin OCR'd around a region, in tiles: rows above and below, columns left and right.
const PAD_Y: usize = 1;
const PAD_X: usize = 2;
/// Above this share of the frame changed, OCR the whole frame.
const FULL_AT: f64 = 0.5;

fn run_chunk(j: &Job, det: &Detectors, tess: &Tesseract, ffmpeg: &str, thr: u8) -> Result<(Vec<Sample>, ChunkStats)> {
    let (w, h) = (j.w, j.h);
    let mut st = ChunkStats::default();
    let mut out = vec![];
    let mut dec = frames::decode(ffmpeg, &j.path, &j.chunk, w, h)?;
    let (sw, sh) = (w / DS, h / DS);
    let mut reference: Option<Vec<u8>> = None;
    let mut words: Vec<Word> = vec![];
    let mut obs: Vec<Obs> = vec![];
    let mut t_dec = Instant::now();
    for Frame { t_ms, px } in dec.by_ref() {
        st.decode_ms += t_dec.elapsed().as_secs_f64() * 1000.0;
        st.sampled += 1;
        let small = downscale(&px, w, h);
        let regions: Vec<Region> = match &reference {
            None => vec![Region::whole(w, h)],
            Some(r) => changed_regions(r, &small, sw, sh, thr, w, h),
        };
        if !regions.is_empty() {
            // grow regions over words they cut, so no carried word is half re-read
            let mut regions = grow_regions(regions, &words, w, h);
            let area: usize = regions.iter().map(Region::area).sum();
            if area as f64 > FULL_AT * (w * h) as f64 {
                regions = vec![Region::whole(w, h)];
            }
            let to = Instant::now();
            let mut fresh: Vec<Word> = vec![];
            for r in &regions {
                let got = ocr_region(tess, &px, w, r)?;
                // a word touching an inner edge may be cut: the carried reading stays instead
                fresh.extend(got.into_iter().filter(|wd| !r.cuts(wd, w, h)));
            }
            st.ocr_ms += to.elapsed().as_secs_f64() * 1000.0;
            st.ocr += 1;
            st.area += (area.min(w * h)) as f64 / (w * h) as f64;
            if regions == [Region::whole(w, h)] {
                st.full += 1;
            }
            // a carried word goes when its tiles changed, or when fresh readings cover it; in the
            // unchanged margin of a region a partial re-read loses to the carried word
            let mut drop_fresh = vec![false; fresh.len()];
            words.retain(|c| {
                let Some(r) = regions.iter().find(|r| r.holds(c)) else { return true };
                if r.core_hits(c) {
                    return false;
                }
                let over: Vec<usize> = (0..fresh.len()).filter(|&i| same_place(c, &fresh[i])).collect();
                let covered: f32 = over.iter().map(|&i| fresh[i].b[2].min(c.b[2]) - fresh[i].b[0].max(c.b[0])).sum();
                if covered >= 0.8 * (c.b[2] - c.b[0]) {
                    return false;
                }
                for i in over {
                    drop_fresh[i] = true;
                }
                true
            });
            let mut keep = drop_fresh.iter().map(|d| !d);
            fresh.retain(|_| keep.next().unwrap_or(true));
            words.extend(fresh);
            // the reference follows only what was re-read
            let rf = reference.get_or_insert_with(|| small.clone());
            for r in &regions {
                let (x0, x1) = (r.x0 / DS, r.x1.div_ceil(DS).min(sw));
                for y in r.y0 / DS..r.y1.div_ceil(DS).min(sh) {
                    rf[y * sw + x0..y * sw + x1].copy_from_slice(&small[y * sw + x0..y * sw + x1]);
                }
            }
            let td = Instant::now();
            obs = det.detect(&words);
            st.detect_ms += td.elapsed().as_secs_f64() * 1000.0;
            if std::env::var_os("CUTROOM_PRIVACY_DEBUG").is_some() {
                // geometry, counts and redacted previews only
                eprintln!("t={:.0} regions={:?} words={} obs={:?}", t_ms, regions.iter().map(|r| [r.x0, r.y0, r.x1, r.y1]).collect::<Vec<_>>(), words.len(), obs);
            }
        }
        out.push(Sample { t_ms, obs: obs.clone() });
        t_dec = Instant::now();
    }
    dec.finish()?;
    Ok((out, st))
}

/// Regions taller than this are OCR'd as overlapping strips in parallel (idle cores help a busy chunk).
const STRIP: usize = 640;
/// Strip overlap: a text line up to this tall is whole in the strip that owns its center.
const OVERLAP: usize = 112;

/// OCR a region, split into strips when tall; each word comes from the strip owning its center.
fn ocr_region(tess: &Tesseract, px: &[u8], w: usize, r: &Region) -> Result<Vec<Word>> {
    let (y0, y1) = (r.y0, r.y1);
    let n = (y1 - y0).div_ceil(STRIP);
    if n <= 1 {
        return tess.run(px, w, r.x0, r.x1, y0, y1);
    }
    let step = (y1 - y0).div_ceil(n);
    let strips: Vec<(usize, usize, f32, f32)> = (0..n)
        .map(|k| {
            let (a, b) = (y0 + k * step, (y0 + (k + 1) * step).min(y1));
            let own0 = if k == 0 { f32::MIN } else { a as f32 };
            let own1 = if k + 1 == n { f32::MAX } else { b as f32 };
            (a.saturating_sub(OVERLAP / 2).max(y0), (b + OVERLAP / 2).min(y1), own0, own1)
        })
        .collect();
    let parts: Vec<Result<Vec<Word>>> = strips
        .par_iter()
        .map(|&(a, b, own0, own1)| Ok(tess.run(px, w, r.x0, r.x1, a, b)?.into_iter().filter(|wd| wd.yc() >= own0 && wd.yc() < own1).collect()))
        .collect();
    let mut out = vec![];
    for p in parts {
        out.extend(p?);
    }
    Ok(out)
}

/// 4x box downscale.
fn downscale(px: &[u8], w: usize, h: usize) -> Vec<u8> {
    let (sw, sh) = (w / DS, h / DS);
    let mut out = vec![0u8; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            let mut s = 0u32;
            for dy in 0..DS {
                let row = (y * DS + dy) * w + x * DS;
                for &p in &px[row..row + DS] {
                    s += p as u32;
                }
            }
            out[y * sw + x] = (s / (DS * DS) as u32) as u8;
        }
    }
    out
}

/// A box to OCR (OCR pixels, `x0..x1` x `y0..y1`); `core` is where the frame changed, the rest
/// is unchanged margin.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Region {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    core: [usize; 4],
}

impl Region {
    fn whole(w: usize, h: usize) -> Region {
        Region { x0: 0, y0: 0, x1: w, y1: h, core: [0, 0, w, h] }
    }
    fn area(&self) -> usize {
        (self.x1 - self.x0) * (self.y1 - self.y0)
    }
    /// The word lies inside.
    fn holds(&self, c: &Word) -> bool {
        c.b[0] >= self.x0 as f32 && c.b[1] >= self.y0 as f32 && c.b[2] <= self.x1 as f32 && c.b[3] <= self.y1 as f32
    }
    fn core_hits(&self, c: &Word) -> bool {
        let k = self.core.map(|v| v as f32);
        c.b[2] > k[0] && c.b[0] < k[2] && c.b[3] > k[1] && c.b[1] < k[3]
    }
    /// A fresh word touching an edge that is not the frame's edge.
    fn cuts(&self, wd: &Word, w: usize, h: usize) -> bool {
        (self.x0 > 0 && wd.b[0] <= self.x0 as f32 + 2.0) || (self.y0 > 0 && wd.b[1] <= self.y0 as f32 + 2.0) || (self.x1 < w && wd.b[2] >= self.x1 as f32 - 2.0) || (self.y1 < h && wd.b[3] >= self.y1 as f32 - 2.0)
    }
    fn overlaps(&self, o: &Region) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    fn absorb(&mut self, o: &Region) {
        self.x0 = self.x0.min(o.x0);
        self.y0 = self.y0.min(o.y0);
        self.x1 = self.x1.max(o.x1);
        self.y1 = self.y1.max(o.y1);
        self.core = [self.core[0].min(o.core[0]), self.core[1].min(o.core[1]), self.core[2].max(o.core[2]), self.core[3].max(o.core[3])];
    }
}

/// Two readings of the same spot: their boxes overlap by a third of the smaller one.
fn same_place(a: &Word, b: &Word) -> bool {
    let ix = (a.b[2].min(b.b[2]) - a.b[0].max(b.b[0])).max(0.0);
    let iy = (a.b[3].min(b.b[3]) - a.b[1].max(b.b[1])).max(0.0);
    let small = ((a.b[2] - a.b[0]) * (a.b[3] - a.b[1])).min((b.b[2] - b.b[0]) * (b.b[3] - b.b[1]));
    ix * iy > 0.3 * small.max(1.0)
}

/// Boxes around clusters of changed tiles, padded and merged.
fn changed_regions(r: &[u8], s: &[u8], sw: usize, sh: usize, thr: u8, w: usize, h: usize) -> Vec<Region> {
    let t = TILE / DS;
    let (tw, th) = (sw.div_ceil(t), sh.div_ceil(t));
    let mut changed = vec![false; tw * th];
    for ty in 0..th {
        for tx in 0..tw {
            let mut n = 0;
            'tile: for y in ty * t..((ty + 1) * t).min(sh) {
                for x in tx * t..((tx + 1) * t).min(sw) {
                    if r[y * sw + x].abs_diff(s[y * sw + x]) > thr {
                        n += 1;
                        if n >= TILE_PX {
                            changed[ty * tw + tx] = true;
                            break 'tile;
                        }
                    }
                }
            }
        }
    }
    // clusters: changed tiles within JOIN tiles of each other
    let mut seen = vec![false; tw * th];
    let mut out: Vec<Region> = vec![];
    for start in 0..tw * th {
        if !changed[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start];
        let (mut bx0, mut by0, mut bx1, mut by1) = (tw, th, 0, 0);
        while let Some(i) = stack.pop() {
            let (tx, ty) = (i % tw, i / tw);
            (bx0, by0, bx1, by1) = (bx0.min(tx), by0.min(ty), bx1.max(tx + 1), by1.max(ty + 1));
            for ny in ty.saturating_sub(JOIN)..(ty + JOIN + 1).min(th) {
                for nx in tx.saturating_sub(JOIN)..(tx + JOIN + 1).min(tw) {
                    let j = ny * tw + nx;
                    if changed[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        let core = [bx0 * TILE, by0 * TILE, (bx1 * TILE).min(w), (by1 * TILE).min(h)];
        out.push(Region {
            x0: bx0.saturating_sub(PAD_X) * TILE,
            y0: by0.saturating_sub(PAD_Y) * TILE,
            x1: ((bx1 + PAD_X) * TILE).min(w),
            y1: ((by1 + PAD_Y) * TILE).min(h),
            core,
        });
    }
    merge_regions(out)
}

fn merge_regions(mut v: Vec<Region>) -> Vec<Region> {
    loop {
        let mut merged = false;
        let mut out: Vec<Region> = vec![];
        for r in v {
            if let Some(o) = out.iter_mut().find(|o| o.overlaps(&r)) {
                o.absorb(&r);
                merged = true;
            } else {
                out.push(r);
            }
        }
        v = out;
        if !merged {
            return v;
        }
    }
}

/// Extend regions over every carried word they cut or nearly touch from inside (an edge right
/// at a text line makes tesseract misread it); merge overlaps. Words just outside stay out, so
/// growth does not run along a line word by word.
fn grow_regions(mut regions: Vec<Region>, words: &[Word], w: usize, h: usize) -> Vec<Region> {
    const CLEAR_X: f32 = 3.0;
    const CLEAR_Y: f32 = 6.0;
    loop {
        let mut grew = false;
        for r in regions.iter_mut() {
            let (x0, y0, x1, y1) = (r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32);
            for wd in words {
                let hits = wd.b[2] > x0 + 1.0 && wd.b[0] < x1 - 1.0 && wd.b[3] > y0 + 1.0 && wd.b[1] < y1 - 1.0;
                let clear = (r.x0 == 0 || wd.b[0] >= x0 + CLEAR_X) && (r.y0 == 0 || wd.b[1] >= y0 + CLEAR_Y) && (r.x1 == w || wd.b[2] <= x1 - CLEAR_X) && (r.y1 == h || wd.b[3] <= y1 - CLEAR_Y);
                if hits && !clear {
                    r.x0 = r.x0.min((wd.b[0] - CLEAR_X).max(0.0) as usize);
                    r.y0 = r.y0.min((wd.b[1] - CLEAR_Y).max(0.0) as usize);
                    r.x1 = r.x1.max(((wd.b[2] + CLEAR_X).ceil() as usize).min(w));
                    r.y1 = r.y1.max(((wd.b[3] + CLEAR_Y).ceil() as usize).min(h));
                    grew = true;
                }
            }
        }
        regions = merge_regions(regions);
        if !grew {
            return regions;
        }
    }
}

/// A finding before conversion (keeps the value hash for merging).
struct Hit {
    kind: String,
    key: u64,
    preview: String,
    s0: f64,
    s1: f64,
    rect: [f64; 4],
    clip: usize,
    /// Clip range (source ms).
    c: (f64, f64),
    /// Unseen gap before the first sighting / after the last: (from, to) source ms and the box
    /// (OCR pixels) at that sighting.
    gap0: Option<(f64, f64, [f32; 4])>,
    gap1: Option<(f64, f64, [f32; 4])>,
}

/// Merge sightings across consecutive samples. A track survives one sample without a sighting
/// (OCR flicker); it matches by value (same thing, nearby or moved) or by place (same kind, same
/// box, value re-read differently).
fn track(samples: &[Sample], clip: usize, c0: f64, c1: f64, pad: f64, w: usize, h: usize) -> Vec<Hit> {
    struct Tr {
        kind: String,
        key: u64,
        preview: String,
        first_rect: [f32; 4],
        last: [f32; 4],
        rect: [f32; 4],
        first: usize,
        last_i: usize,
    }
    const MAX_GAP: usize = 1;
    let mut open: Vec<Tr> = vec![];
    let mut done: Vec<Tr> = vec![];
    for (i, s) in samples.iter().enumerate() {
        let mut used = vec![false; open.len()];
        for o in &s.obs {
            let mut best: Option<(usize, f32)> = None;
            for (ti, t) in open.iter().enumerate() {
                if used[ti] || t.kind != o.kind {
                    continue;
                }
                let ov = iou(&t.last, &o.rect);
                // the same value anywhere on the next sample is the same thing moving (a scroll):
                // the track's box grows over the path between, so frames in between stay covered
                let score = if t.key == o.key && (ov > 0.1 || near(&t.last, &o.rect)) {
                    3.0 + ov
                } else if t.key == o.key {
                    2.0
                } else if ov > 0.5 {
                    ov
                } else {
                    continue;
                };
                if best.is_none_or(|(_, b)| score > b) {
                    best = Some((ti, score));
                }
            }
            match best {
                Some((ti, _)) => {
                    used[ti] = true;
                    let t = &mut open[ti];
                    t.last = o.rect;
                    t.rect = union(Some(t.rect), o.rect);
                    t.last_i = i;
                }
                None => {
                    open.push(Tr { kind: o.kind.clone(), key: o.key, preview: o.preview.clone(), first_rect: o.rect, last: o.rect, rect: o.rect, first: i, last_i: i });
                    used.push(true);
                }
            }
        }
        let (keep, close): (Vec<Tr>, Vec<Tr>) = open.into_iter().partition(|t| t.last_i + MAX_GAP >= i);
        open = keep;
        done.extend(close);
    }
    done.extend(open);
    let n = samples.len();
    done.into_iter()
        .map(|t| {
            let (a, b) = (samples[t.first].t_ms, samples[t.last_i].t_ms);
            // seen from the clip's first sample: it may have been there from the clip start
            let gap0 = (t.first > 0).then(|| (samples[t.first - 1].t_ms, a, t.first_rect));
            let gap1 = (t.last_i + 1 < n).then(|| (b, samples[t.last_i + 1].t_ms, t.last));
            let s0 = gap0.map_or(c0, |(p, a, _)| a - pad * (a - p).max(0.0));
            let s1 = gap1.map_or(c1, |(b, q, _)| b + pad * (q - b).max(0.0));
            Hit { kind: t.kind, key: t.key, preview: t.preview, s0: s0.max(c0), s1: s1.min(c1).max(s0.max(c0) + 1.0), rect: norm_rect(t.rect, w, h), clip, c: (c0, c1), gap0, gap1 }
        })
        .collect()
}

/// How a box changed across an unseen gap.
enum Change {
    /// Media ms of the first frame in the new state, and one frame's duration.
    At(f64, f64),
    /// The box looks the same at both ends: OCR missed it on one side, so it was likely there.
    Flat,
    Failed,
}

/// Put every finding edge on the frame where its box changes. Gaps shared by several findings
/// are decoded once. Returns how many edges were placed on a frame.
fn refine_edges(src: &Source, hits: &mut [Hit], media: &BTreeMap<usize, (PathBuf, usize, usize)>, ffmpeg: &str, pool: &rayon::ThreadPool) -> Result<usize> {
    // (clip, from, to in µs) -> [(hit, is_start, box)]
    type Groups = BTreeMap<(usize, i64, i64), Vec<(usize, bool, [f32; 4])>>;
    let mut groups: Groups = BTreeMap::new();
    for (i, hit) in hits.iter().enumerate() {
        for (g, start) in [(hit.gap0, true), (hit.gap1, false)] {
            if let Some((a, b, r)) = g {
                groups.entry((hit.clip, (a * 1000.0) as i64, (b * 1000.0) as i64)).or_default().push((i, start, r));
            }
        }
    }
    let groups: Vec<_> = groups.into_iter().collect();
    let results: Vec<Vec<(usize, bool, Change)>> = pool.install(|| {
        groups
            .par_iter()
            .map(|((ci, a, b), items)| {
                let (path, w, h) = &media[ci];
                let clip = &src.clips[*ci];
                let (ma, mb) = (clip.media_at(*a as f64 / 1000.0), clip.media_at(*b as f64 / 1000.0));
                let edges: Vec<([f32; 4], bool)> = items.iter().map(|x| (x.2, x.1)).collect();
                let ch = changes(ffmpeg, path, *w, *h, ma, mb, &edges);
                items.iter().zip(ch).map(|(x, c)| (x.0, x.1, c)).collect()
            })
            .collect()
    });
    let mut placed = 0;
    for (i, start, c) in results.into_iter().flatten() {
        let hit = &mut hits[i];
        let clip = &src.clips[hit.clip];
        let to_src = |m: f64| clip.start_ms + (m - clip.media_in_ms);
        // a start edge blurs from one frame before the change; an end edge until one frame after
        // the first frame without it
        if start {
            let (a, b, _) = hit.gap0.unwrap();
            hit.s0 = match c {
                Change::At(m, dur) => {
                    placed += 1;
                    to_src(m) - dur
                }
                Change::Flat => a - (b - a),
                Change::Failed => a,
            }
            .max(hit.c.0);
        } else {
            let (a, b, _) = hit.gap1.unwrap();
            hit.s1 = match c {
                Change::At(m, dur) => {
                    placed += 1;
                    to_src(m) + dur
                }
                Change::Flat => b + (b - a),
                Change::Failed => b,
            }
            .min(hit.c.1);
        }
        hit.s1 = hit.s1.max(hit.s0 + 1.0);
    }
    Ok(placed)
}

/// Decode media `ma..=mb` (every frame) and, per box, find where it leaves its state at `ma`.
/// A start edge (`true`) is the first frame 10% of the way to the end state (blur early); an end
/// edge the first frame 90% of the way (blur until it is gone).
fn changes(ffmpeg: &str, path: &Path, w: usize, h: usize, ma: f64, mb: f64, items: &[([f32; 4], bool)]) -> Vec<Change> {
    let fail = || items.iter().map(|_| Change::Failed).collect();
    let chunk = Chunk { a_ms: ma - 1.0, len_ms: mb - ma + 3.0, keep: None, every_ms: 0.0, crop: None };
    let Ok(mut dec) = frames::decode(ffmpeg, path, &chunk, w, h) else { return fail() };
    let boxes: Vec<[usize; 4]> = items
        .iter()
        .map(|(r, _)| {
            let (x0, y0) = ((r[0] - 8.0).max(0.0) as usize, (r[1] - 8.0).max(0.0) as usize);
            let (x1, y1) = (((r[2] + 8.0).ceil() as usize).min(w), ((r[3] + 8.0).ceil() as usize).min(h));
            [x0, y0, x1.max(x0 + 1), y1.max(y0 + 1)]
        })
        .collect();
    let crop = |px: &[u8], b: &[usize; 4]| -> Vec<u8> { (b[1]..b[3]).flat_map(|y| px[y * w + b[0]..y * w + b[2]].iter().copied()).collect() };
    let mut first: Vec<Vec<u8>> = vec![];
    let mut times: Vec<f64> = vec![];
    // per frame, per box: mean abs difference from the box at `ma`
    let mut dist: Vec<Vec<f64>> = vec![];
    for f in dec.by_ref() {
        if first.is_empty() {
            first = boxes.iter().map(|b| crop(&f.px, b)).collect();
        }
        dist.push(boxes.iter().zip(&first).map(|(b, f0)| mad(&crop(&f.px, b), f0)).collect());
        times.push(f.t_ms);
    }
    if dec.finish().is_err() || times.len() < 2 {
        return fail();
    }
    let mut steps: Vec<f64> = times.windows(2).map(|t| t[1] - t[0]).collect();
    steps.sort_by(f64::total_cmp);
    let dur = steps[steps.len() / 2].max(1.0);
    let n = times.len();
    items
        .iter()
        .enumerate()
        .map(|(k, (_, start))| {
            let total = dist[n - 1][k];
            if total < 2.0 {
                return Change::Flat;
            }
            let frac = if *start { 0.1 } else { 0.9 };
            let i = (0..n).find(|&i| dist[i][k] > frac * total).unwrap_or(n - 1);
            Change::At(times[i], dur)
        })
        .collect()
}

fn mad(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as u64).sum::<u64>() as f64 / a.len() as f64
}

/// Boxes close together (a value that moved a little, or re-OCR'd with a different extent).
fn near(a: &[f32; 4], b: &[f32; 4]) -> bool {
    let h = (a[3] - a[1]).max(b[3] - b[1]);
    (a[0] - b[0]).abs() < 4.0 * h && (a[1] - b[1]).abs() < 1.5 * h
}

/// Pad a pixel box (a quarter of its height, at least 4 px) and normalize to 0..1.
fn norm_rect(r: [f32; 4], w: usize, h: usize) -> [f64; 4] {
    let p = ((r[3] - r[1]) * 0.25).max(4.0);
    let (w, h) = (w as f64, h as f64);
    let x0 = ((r[0] - p) as f64 / w).clamp(0.0, 1.0);
    let y0 = ((r[1] - p) as f64 / h).clamp(0.0, 1.0);
    let x1 = ((r[2] + p) as f64 / w).clamp(0.0, 1.0);
    let y1 = ((r[3] + p) as f64 / h).clamp(0.0, 1.0);
    [x0, y0, x1 - x0, y1 - y0]
}

fn rect_iou(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let ix = ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.0);
    let iy = ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.0);
    let i = ix * iy;
    let u = a[2] * a[3] + b[2] * b[3] - i;
    if u <= 0.0 {
        0.0
    } else {
        i / u
    }
}

/// Join hits of the same thing that touch in time (clip boundaries of one recording).
fn merge_hits(mut hits: Vec<Hit>) -> Vec<Finding> {
    hits.sort_by(|a, b| a.s0.total_cmp(&b.s0));
    let mut out: Vec<Hit> = vec![];
    for h in hits {
        if let Some(p) = out.iter_mut().rev().take(50).find(|p| p.kind == h.kind && p.key == h.key && h.s0 <= p.s1 + 1.0 && rect_iou(&p.rect, &h.rect) > 0.5) {
            p.s1 = p.s1.max(h.s1);
            let (x0, y0) = (p.rect[0].min(h.rect[0]), p.rect[1].min(h.rect[1]));
            let (x1, y1) = ((p.rect[0] + p.rect[2]).max(h.rect[0] + h.rect[2]), (p.rect[1] + p.rect[3]).max(h.rect[1] + h.rect[3]));
            p.rect = [x0, y0, x1 - x0, y1 - y0];
            continue;
        }
        out.push(h);
    }
    out.into_iter().map(|h| Finding { src0_ms: h.s0, src1_ms: h.s1, rect: h.rect, kind: h.kind, preview: h.preview }).collect()
}

/// Write findings as JSON (what `cutroom check` reads from `privacy.json`).
pub fn write_json(path: &Path, findings: &[Finding]) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(findings)?).with_context(|| format!("writing {}", path.display()))
}

/// Blur beats for `findings`, as a YAML `beats:` list anchored to source time. Time is split at
/// every finding edge; each piece blurs the findings on screen then, and neighbouring pieces with
/// the same findings merge, so overlapping findings share a beat with several rects.
pub fn propose_beats(findings: &[Finding]) -> String {
    // start rounded down, end up, rects grown outward: a beat always covers its findings
    let span = |f: &Finding| ((f.src0_ms / 10.0).floor() * 10.0, (f.src1_ms / 10.0).ceil() * 10.0);
    let mut edges: Vec<f64> = findings.iter().flat_map(|f| {
        let (a, b) = span(f);
        [a, b]
    }).collect();
    edges.sort_by(f64::total_cmp);
    edges.dedup();
    let mut pieces: Vec<(f64, f64, Vec<usize>)> = vec![];
    for e in edges.windows(2) {
        let (a, b) = (e[0], e[1]);
        let on: Vec<usize> = (0..findings.len()).filter(|&i| {
            let (x, y) = span(&findings[i]);
            x <= a && y >= b
        }).collect();
        if on.is_empty() {
            continue;
        }
        match pieces.last_mut() {
            Some(p) if p.1 == a && p.2 == on => p.1 = b,
            _ => pieces.push((a, b, on)),
        }
    }
    let mut s = format!("# cutroom privacy: {} blur beats for {} findings, anchored to source time (s:) so they survive re-cuts.\n", pieces.len(), findings.len());
    s.push_str("# Delete a beat only after checking the frame: check refuses any finding left unblurred.\n");
    if pieces.is_empty() {
        s.push_str("beats: []\n");
        return s;
    }
    s.push_str("beats:\n");
    for (a, b, on) in pieces {
        let mut rects: Vec<[f64; 4]> = vec![];
        for &i in &on {
            let r = grow4(&findings[i].rect);
            // a rect inside another one adds nothing
            if rects.iter().any(|q| inside(&r, q)) {
                continue;
            }
            rects.retain(|q| !inside(q, &r));
            rects.push(r);
        }
        let rects: Vec<String> = rects.iter().map(|r| format!("[{:.4}, {:.4}, {:.4}, {:.4}]", r[0], r[1], r[2], r[3])).collect();
        let mut what: Vec<String> = on.iter().map(|&i| format!("{} {}", findings[i].kind, findings[i].preview)).collect();
        what.dedup();
        s.push_str(&format!("  - {{at: \"s:{:.2}\", until: \"s:{:.2}\", kind: blur, rects: [{}], lead: 0}}  # {}\n", a / 1000.0, b / 1000.0, rects.join(", "), what.join("; ")));
    }
    s
}

/// Round a rect outward to 4 decimals, inside 0..1.
fn grow4(r: &[f64; 4]) -> [f64; 4] {
    let x0 = (r[0] * 1e4).floor().max(0.0) / 1e4;
    let y0 = (r[1] * 1e4).floor().max(0.0) / 1e4;
    let x1 = ((r[0] + r[2]) * 1e4).ceil().min(1e4) / 1e4;
    let y1 = ((r[1] + r[3]) * 1e4).ceil().min(1e4) / 1e4;
    [x0, y0, x1 - x0, y1 - y0]
}

fn inside(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[0] >= b[0] && a[1] >= b[1] && a[0] + a[2] <= b[0] + b[2] + 1e-9 && a[1] + a[3] <= b[1] + b[3] + 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(key: u64, y: f32) -> Obs {
        Obs { kind: "email".into(), key, preview: "ja***@e***".into(), rect: [100.0, y, 400.0, y + 20.0] }
    }

    fn samples(at: &[(f64, Vec<Obs>)]) -> Vec<Sample> {
        at.iter().map(|(t, o)| Sample { t_ms: *t, obs: o.clone() }).collect()
    }

    #[test]
    fn tracks_merge_flicker_and_scroll() {
        // seen at 1, 2, missed at 3 (OCR flicker), seen at 4; gone at 5
        let s = samples(&[(0.0, vec![]), (1000.0, vec![obs(1, 500.0)]), (2000.0, vec![obs(1, 500.0)]), (3000.0, vec![]), (4000.0, vec![obs(1, 500.0)]), (5000.0, vec![]), (6000.0, vec![])]);
        let h = track(&s, 0, 0.0, 7000.0, 0.5, 1000, 1000);
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].s0, h[0].s1), (500.0, 4500.0));
        // scrolling up 200 px a sample: one finding whose box covers the whole path
        let s = samples(&[(0.0, vec![]), (1000.0, vec![obs(2, 800.0)]), (2000.0, vec![obs(2, 600.0)]), (3000.0, vec![obs(2, 400.0)]), (4000.0, vec![])]);
        let h = track(&s, 0, 0.0, 5000.0, 0.5, 1000, 1000);
        assert_eq!(h.len(), 1);
        assert!(h[0].rect[1] < 0.4 && h[0].rect[1] + h[0].rect[3] > 0.82, "{:?}", h[0].rect);
        // seen from the clip's first sample to its last: the whole clip
        let s = samples(&[(100.0, vec![obs(3, 10.0)]), (1100.0, vec![obs(3, 10.0)])]);
        let h = track(&s, 0, 0.0, 2000.0, 0.5, 1000, 1000);
        assert_eq!((h[0].s0, h[0].s1), (0.0, 2000.0));
    }

    fn finding(a: f64, b: f64, x: f64) -> Finding {
        Finding { src0_ms: a, src1_ms: b, rect: [x, 0.1, 0.2, 0.05], kind: "api_key".into(), preview: "sk-ant...".into() }
    }

    #[test]
    fn beats_share_time_windows() {
        let y = propose_beats(&[finding(1000.0, 5000.0, 0.1), finding(3000.0, 8000.0, 0.5), finding(20003.0, 20996.0, 0.1)]);
        let lines: Vec<&str> = y.lines().filter(|l| l.trim_start().starts_with("- {")).collect();
        assert_eq!(lines.len(), 4, "{y}");
        assert!(lines[0].contains(r#"at: "s:1.00", until: "s:3.00""#) && lines[0].matches('[').count() == 2, "{y}");
        assert!(lines[1].contains(r#"at: "s:3.00", until: "s:5.00""#) && lines[1].matches('[').count() == 3, "two rects: {y}");
        assert!(lines[2].contains(r#"at: "s:5.00", until: "s:8.00""#), "{y}");
        assert!(lines[3].contains(r#"at: "s:20.00", until: "s:21.00""#), "rounded outward: {y}");
        assert!(lines.iter().all(|l| l.contains("kind: blur") && l.contains("lead: 0")));
        assert!(propose_beats(&[]).contains("beats: []"));
    }
}
