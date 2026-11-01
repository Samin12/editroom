//! Borumi projects (`*.bmprojbundle`): `project.bmproj` is SQLite, media lives in
//! `medias/<media_id>.<ext>`.
//!
//! Safety: the bundle is never opened for writing. The database is opened read-only and copied
//! with SQLite's online backup API to a private temp file; everything is read from that copy.
//!
//! Time model: a take's clips start at `clips.start_offset` (seconds after the take began; a few
//! ms apart for camera / screen / mic). A timeline segment's `clip_start` is take time, shared by
//! every track of a segment group, so the media time it plays is `clip_start - start_offset`.

use crate::probe;
use anyhow::{anyhow, bail, Context, Result};
use cutroom_core::{Clip, Media, Scene, Source, Track, Word};
use rayon::prelude::*;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorumiMode {
    /// What is on the Borumi timeline now: scenes in order, every Video/AudioClip segment.
    Timeline,
    /// Every narration take's full recording, back to back (scene order, then recording time).
    Raw,
}

/// What a Borumi clip / segment carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Content {
    Camera,
    Screen,
    Microphone,
    SystemSound,
}

impl Content {
    pub fn parse(kind: &str) -> Option<Content> {
        match kind {
            "Camera" => Some(Content::Camera),
            "Screen" => Some(Content::Screen),
            "Microphone" => Some(Content::Microphone),
            "SystemSound" => Some(Content::SystemSound),
            _ => None,
        }
    }
    pub fn track(self) -> Track {
        match self {
            Content::Camera => Track::Camera,
            Content::Screen => Track::Screen,
            Content::Microphone => Track::Mic,
            Content::SystemSound => Track::System,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BScene {
    pub id: String,
    pub name: String,
    /// Fractional-index sort key (compare raw bytes).
    pub seq: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct BTake {
    pub id: String,
    pub scene_id: String,
    pub name: String,
    pub started_at: f64,
}

#[derive(Clone, Debug)]
pub struct BClip {
    pub id: String,
    pub take_id: String,
    pub media: String,
    /// Offset after the take start (ms).
    pub start_offset_ms: f64,
    /// Explicit clip length (ms); `None` = the whole media.
    pub duration_ms: Option<f64>,
    pub video: bool,
}

#[derive(Clone, Debug)]
pub struct BMedia {
    pub id: String,
    /// The recording source (camera, screen, mic, system); `None` for imported files.
    pub content: Option<Content>,
    pub duration_ms: f64,
    pub width: u32,
    pub height: u32,
    /// `Image` medias (stills) are not footage.
    pub image: bool,
    pub video: bool,
    /// Transcript words in MEDIA ms (stutters and repeats kept), when transcription succeeded.
    pub words: Option<Vec<Word>>,
    pub path: PathBuf,
}

/// A Video/AudioClip segment of a scene timeline.
#[derive(Clone, Debug)]
pub struct Segment {
    pub scene_id: String,
    /// Start inside the scene (ms).
    pub start_ms: f64,
    pub dur_ms: f64,
    pub take_id: String,
    pub clip_id: String,
    /// Take time the segment starts playing (ms).
    pub clip_start_ms: f64,
    pub content: Content,
    pub speed: f64,
    pub group: String,
}

/// The tables of a Borumi project, typed. Scenes are sorted by `seq`; segments by scene, start.
#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub origin: String,
    pub scenes: Vec<BScene>,
    pub takes: Vec<BTake>,
    pub clips: Vec<BClip>,
    pub medias: HashMap<String, BMedia>,
    pub segments: Vec<Segment>,
    /// Scene id -> length of its timeline (ms): the end of its last Video/AudioClip segment
    /// (overlays may run past it and are clipped by Borumi).
    pub scene_len_ms: HashMap<String, f64>,
}

/// Ingest a Borumi bundle (read-only; the DB is snapshotted first).
pub fn borumi(bundle: &Path, mode: BorumiMode) -> Result<Source> {
    Project::open(bundle)?.to_source(mode, true)
}

/// Ingest from a database file and a medias folder (e.g. a copy of `project.bmproj`).
pub fn borumi_db(db: &Path, medias_dir: &Path, mode: BorumiMode) -> Result<Source> {
    Project::open_db(db, medias_dir)?.to_source(mode, true)
}

/// Copy a SQLite database with the online backup API. The source is opened read-only (no
/// journal, no WAL, no writes) and released as soon as the copy is done.
pub fn snapshot_db(db: &Path, dst: &Path) -> Result<()> {
    if !db.is_file() {
        bail!("{} not found", db.display());
    }
    let src = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .with_context(|| format!("opening {} read-only", db.display()))?;
    let mut out = Connection::open(dst)?;
    {
        let b = rusqlite::backup::Backup::new(&src, &mut out)?;
        let mut tries = 0;
        loop {
            match b.step(-1)? {
                rusqlite::backup::StepResult::Done => break,
                rusqlite::backup::StepResult::More => {}
                _ => {
                    tries += 1;
                    if tries > 200 {
                        bail!("{} stayed locked; try again", db.display());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    }
    drop(src);
    Ok(())
}

impl Project {
    pub fn open(bundle: &Path) -> Result<Project> {
        let mut p = Project::open_db(&bundle.join("project.bmproj"), &bundle.join("medias"))?;
        p.name = bundle.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        p.origin = bundle.display().to_string();
        Ok(p)
    }

    pub fn open_db(db: &Path, medias_dir: &Path) -> Result<Project> {
        let dir = crate::temp_dir("borumi")?;
        let copy = dir.join("project.bmproj");
        let res = snapshot_db(db, &copy).and_then(|_| {
            let conn = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            read(&conn, medias_dir)
        });
        let _ = std::fs::remove_dir_all(&dir);
        let mut p = res?;
        p.name = db.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        p.origin = db.display().to_string();
        Ok(p)
    }

    pub fn clip(&self, id: &str) -> Option<&BClip> {
        self.clips.iter().find(|c| c.id == id)
    }

    /// Media time (ms) a segment starts playing.
    pub fn media_in_ms(&self, seg: &Segment) -> f64 {
        let off = self.clip(&seg.clip_id).map(|c| c.start_offset_ms).unwrap_or(0.0);
        (seg.clip_start_ms - off).max(0.0)
    }

    /// Narration takes (a mic clip with a transcript), in scene order then recording time.
    pub fn narration_takes(&self) -> Vec<&BTake> {
        let scene_rank: HashMap<&str, usize> = self.scenes.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        let mut takes: Vec<&BTake> = self
            .takes
            .iter()
            .filter(|t| scene_rank.contains_key(t.scene_id.as_str()))
            .filter(|t| self.clips.iter().any(|c| c.take_id == t.id && self.is_narration_mic(&c.media)))
            .collect();
        takes.sort_by(|a, b| scene_rank[a.scene_id.as_str()].cmp(&scene_rank[b.scene_id.as_str()]).then(a.started_at.total_cmp(&b.started_at)));
        takes
    }

    fn is_narration_mic(&self, media: &str) -> bool {
        self.medias.get(media).is_some_and(|m| m.content == Some(Content::Microphone) && m.words.is_some())
    }

    /// Media-time ranges (ms, merged) of each mic media that the current timeline plays.
    pub fn mic_kept_ranges(&self) -> BTreeMap<String, Vec<(f64, f64)>> {
        let mut out: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
        for s in self.segments.iter().filter(|s| s.content == Content::Microphone) {
            let Some(c) = self.clip(&s.clip_id) else { continue };
            let a = self.media_in_ms(s);
            out.entry(c.media.clone()).or_default().push((a, a + s.dur_ms * s.speed));
        }
        for v in out.values_mut() {
            v.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut m: Vec<(f64, f64)> = vec![];
            for r in v.drain(..) {
                match m.last_mut() {
                    Some(l) if r.0 <= l.1 + 1.0 => l.1 = l.1.max(r.1),
                    _ => m.push(r),
                }
            }
            *v = m;
        }
        out
    }

    fn clip_len_ms(&self, c: &BClip) -> f64 {
        c.duration_ms.filter(|d| *d > 0.0).unwrap_or_else(|| self.medias.get(&c.media).map(|m| m.duration_ms).unwrap_or(0.0))
    }

    fn scene_name(&self, idx: usize) -> String {
        let s = &self.scenes[idx];
        let n = s.name.trim();
        if n.is_empty() {
            format!("Scene {}", idx + 1)
        } else {
            n.to_string()
        }
    }

    /// Flatten into a cutroom `Source`. `probe_media` runs ffprobe on footage that exists (fps).
    pub fn to_source(&self, mode: BorumiMode, probe_media: bool) -> Result<Source> {
        let mut src = Source { version: 1, name: self.name.clone(), origin: self.origin.clone(), ..Default::default() };
        match mode {
            BorumiMode::Raw => self.fill_raw(&mut src),
            BorumiMode::Timeline => self.fill_timeline(&mut src),
        }
        if src.clips.is_empty() {
            return Err(anyhow!("{}: nothing to ingest ({mode:?} mode found no clips)", self.origin));
        }
        crate::transcript::tidy(&mut src.words);
        src.media = self.media_list(&src.clips, probe_media);
        Ok(src)
    }

    fn fill_raw(&self, src: &mut Source) {
        let scene_idx: HashMap<&str, usize> = self.scenes.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        let mut t0 = 0.0;
        let mut prev_scene = None;
        for (ti, take) in self.narration_takes().into_iter().enumerate() {
            let clips: Vec<&BClip> = self.clips.iter().filter(|c| c.take_id == take.id).filter(|c| self.medias.get(&c.media).is_some_and(|m| !m.image)).collect();
            let mut len: f64 = 0.0;
            for c in &clips {
                let m = &self.medias[&c.media];
                let dur = self.clip_len_ms(c);
                len = len.max(c.start_offset_ms + dur);
                let content = m.content.unwrap_or(if c.video { Content::Screen } else { Content::SystemSound });
                if dur <= 0.0 {
                    continue;
                }
                src.clips.push(Clip { track: content.track(), media: c.media.clone(), start_ms: t0 + c.start_offset_ms, dur_ms: dur, media_in_ms: 0.0, take: ti as u32 });
                if let (Some(Content::Microphone), Some(words)) = (m.content, &m.words) {
                    for w in words.iter().filter(|w| w.start_ms < dur) {
                        let base = t0 + c.start_offset_ms;
                        src.words.push(Word { text: w.text.clone(), start_ms: base + w.start_ms, end_ms: base + w.end_ms.min(dur) });
                    }
                }
            }
            let si = scene_idx[take.scene_id.as_str()];
            match src.scenes.last_mut() {
                Some(s) if prev_scene == Some(si) => s.end_ms = t0 + len,
                _ => src.scenes.push(Scene { name: self.scene_name(si), start_ms: t0, end_ms: t0 + len }),
            }
            prev_scene = Some(si);
            t0 += len;
        }
        src.duration_ms = t0;
    }

    fn fill_timeline(&self, src: &mut Source) {
        let mut off = 0.0;
        let mut take_idx: HashMap<&str, u32> = HashMap::new();
        for (si, scene) in self.scenes.iter().enumerate() {
            let len = self.scene_len_ms.get(&scene.id).copied().unwrap_or(0.0);
            if len <= 0.0 {
                continue;
            }
            for s in self.segments.iter().filter(|s| s.scene_id == scene.id) {
                let Some(c) = self.clip(&s.clip_id) else { continue };
                let Some(m) = self.medias.get(&c.media) else { continue };
                if m.image || s.dur_ms <= 0.0 {
                    continue;
                }
                let n = take_idx.len() as u32;
                let ti = *take_idx.entry(s.take_id.as_str()).or_insert(n);
                let media_in = self.media_in_ms(s);
                let start = off + s.start_ms;
                src.clips.push(Clip { track: s.content.track(), media: c.media.clone(), start_ms: start, dur_ms: s.dur_ms, media_in_ms: media_in, take: ti });
                if s.content == Content::Microphone {
                    if let Some(words) = &m.words {
                        let span = s.dur_ms * s.speed;
                        let lo = words.partition_point(|w| w.end_ms <= media_in);
                        for w in words[lo..].iter().take_while(|w| w.start_ms < media_in + span) {
                            let mid = (w.start_ms + w.end_ms) / 2.0;
                            if mid < media_in || mid >= media_in + span {
                                continue;
                            }
                            let a = start + (w.start_ms - media_in).max(0.0) / s.speed;
                            let b = start + (w.end_ms - media_in).min(span) / s.speed;
                            src.words.push(Word { text: w.text.clone(), start_ms: a, end_ms: b.max(a) });
                        }
                    }
                }
            }
            src.scenes.push(Scene { name: self.scene_name(si), start_ms: off, end_ms: off + len });
            off += len;
        }
        src.duration_ms = off;
    }

    fn media_list(&self, clips: &[Clip], probe_media: bool) -> Vec<Media> {
        let mut ids: Vec<&str> = clips.iter().map(|c| c.media.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        ids.par_iter()
            .map(|id| {
                let m = &self.medias[*id];
                let mut out = Media { id: m.id.clone(), path: m.path.clone(), width: m.width, height: m.height, fps: 0.0, duration_ms: m.duration_ms };
                if probe_media && m.video && m.path.is_file() {
                    if let Ok(p) = probe::probe(&m.path) {
                        out.fps = p.fps;
                        if out.width == 0 {
                            out.width = p.width;
                            out.height = p.height;
                        }
                        if out.duration_ms <= 0.0 {
                            out.duration_ms = p.duration_ms;
                        }
                    }
                }
                out
            })
            .collect()
    }
}

fn kind_of(v: &str) -> Option<String> {
    let j: Value = serde_json::from_str(v).ok()?;
    j.get("kind").and_then(Value::as_str).map(str::to_string)
}

fn read(conn: &Connection, medias_dir: &Path) -> Result<Project> {
    let mut scenes: Vec<BScene> = conn
        .prepare("SELECT id, COALESCE(name,''), CAST(COALESCE(seq,'') AS BLOB) FROM scenes")?
        .query_map([], |r| Ok(BScene { id: r.get(0)?, name: r.get(1)?, seq: r.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    scenes.sort_by(|a, b| a.seq.cmp(&b.seq).then(a.id.cmp(&b.id)));

    let takes: Vec<BTake> = conn
        .prepare("SELECT id, COALESCE(scene_id,''), COALESCE(name,''), COALESCE(started_at,0) FROM takes")?
        .query_map([], |r| Ok(BTake { id: r.get(0)?, scene_id: r.get(1)?, name: r.get(2)?, started_at: r.get(3)? }))?
        .collect::<rusqlite::Result<_>>()?;

    let clips: Vec<BClip> = conn
        .prepare("SELECT id, COALESCE(take_id,''), COALESCE(media,''), COALESCE(start_offset,0), duration, COALESCE(media_track,'') FROM clips")?
        .query_map([], |r| {
            let track: String = r.get(5)?;
            Ok(BClip {
                id: r.get(0)?,
                take_id: r.get(1)?,
                media: r.get(2)?,
                start_offset_ms: r.get::<_, f64>(3)? * 1000.0,
                duration_ms: r.get::<_, Option<f64>>(4)?.map(|d| d * 1000.0),
                video: kind_of(&track).as_deref() == Some("Video"),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let sources: HashMap<String, Content> = conn
        .prepare("SELECT id, COALESCE(kind,'') FROM project_sources")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .filter_map(|(id, k)| Some((id, kind_of(&k).and_then(|k| Content::parse(&k))?)))
        .collect();

    let files = media_files(medias_dir);
    type MediaRow = (String, Option<String>, Option<f64>, Option<String>, Option<String>, Option<i64>, Option<i64>);
    let rows: Vec<MediaRow> = conn
        .prepare("SELECT id, project_source, duration, transcription_status, tracks, width, height FROM medias")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let medias: HashMap<String, BMedia> = rows
        .into_par_iter()
        .map(|(id, psrc, dur, ts, tracks, w, h)| {
            let content = psrc.as_deref().and_then(|p| sources.get(p).copied());
            let tracks: Vec<String> = tracks
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok())
                .and_then(|v| v.as_array().map(|a| a.iter().filter_map(|x| x.get("kind").and_then(Value::as_str).map(str::to_string)).collect()))
                .unwrap_or_default();
            let image = tracks.iter().any(|t| t == "Image");
            let video = tracks.iter().any(|t| t == "Video");
            let words = ts.as_deref().and_then(parse_status);
            let path = files.get(&id).cloned().unwrap_or_else(|| {
                let ext = if image {
                    "png"
                } else if video {
                    "mov"
                } else {
                    "wav"
                };
                medias_dir.join(format!("{id}.{ext}"))
            });
            let m = BMedia { id: id.clone(), content, duration_ms: dur.unwrap_or(0.0) * 1000.0, width: w.unwrap_or(0).max(0) as u32, height: h.unwrap_or(0).max(0) as u32, image, video, words, path };
            (id, m)
        })
        .collect();

    let mut scene_len_ms: HashMap<String, f64> = HashMap::new();
    let mut segments = vec![];
    let mut stmt = conn.prepare("SELECT COALESCE(scene_id,''), COALESCE(start,0), COALESCE(duration,0), COALESCE(kind,''), COALESCE(segment_group,''), COALESCE(segment_order,0) FROM scene_timeline_segments")?;
    let seg_rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?)))?;
    let mut order = vec![];
    for row in seg_rows {
        let (scene_id, start, dur, kind, group, ord) = row?;
        let Ok(k) = serde_json::from_str::<Value>(&kind) else { continue };
        let kk = k.get("kind").and_then(Value::as_str).unwrap_or("");
        if kk != "VideoClip" && kk != "AudioClip" {
            continue;
        }
        let e = scene_len_ms.entry(scene_id.clone()).or_insert(0.0);
        *e = e.max((start + dur) * 1000.0);
        let content = k.pointer("/content_kind/kind").and_then(Value::as_str).and_then(Content::parse);
        let (Some(content), Some(take_id), Some(clip_id)) = (content, k.get("take_id").and_then(Value::as_str), k.get("clip_id").and_then(Value::as_str)) else { continue };
        let speed = k.get("speed").and_then(Value::as_f64).filter(|s| *s > 0.0).unwrap_or(1.0);
        segments.push(Segment {
            scene_id,
            start_ms: start * 1000.0,
            dur_ms: dur * 1000.0,
            take_id: take_id.to_string(),
            clip_id: clip_id.to_string(),
            clip_start_ms: k.get("clip_start").and_then(Value::as_f64).unwrap_or(0.0) * 1000.0,
            content,
            speed,
            group,
        });
        order.push(ord);
    }
    let rank: HashMap<&str, usize> = scenes.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
    let mut idx: Vec<usize> = (0..segments.len()).collect();
    idx.sort_by(|&a, &b| {
        let (sa, sb) = (&segments[a], &segments[b]);
        rank.get(sa.scene_id.as_str()).cmp(&rank.get(sb.scene_id.as_str())).then(sa.start_ms.total_cmp(&sb.start_ms)).then(order[a].cmp(&order[b]))
    });
    let segments = idx.into_iter().map(|i| segments[i].clone()).collect();
    Ok(Project { name: String::new(), origin: String::new(), scenes, takes, clips, medias, segments, scene_len_ms })
}

/// `transcription_status` JSON -> words (media ms), only for `{"kind":"Success"}`.
fn parse_status(ts: &str) -> Option<Vec<Word>> {
    let v: Value = serde_json::from_str(ts).ok()?;
    if v.get("kind").and_then(Value::as_str) != Some("Success") {
        return None;
    }
    let segs = v.pointer("/transcription/segments")?.as_array()?;
    let mut w = crate::transcript::parakeet_segments(segs).ok()?;
    crate::transcript::tidy(&mut w);
    Some(w)
}

/// `medias/` listing: id -> file (cursor sidecars ignored).
fn media_files(dir: &Path) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        let (Some(stem), Some(ext)) = (p.file_stem().and_then(|s| s.to_str()), p.extension().and_then(|s| s.to_str())) else { continue };
        if ext == "cursor" {
            continue;
        }
        out.insert(stem.to_string(), p.clone());
    }
    out
}
