//! One take from plain files: camera and/or screen video, a mic track, an optional transcript.

use crate::probe::{probe, Probe};
use crate::transcript;
use anyhow::{bail, Context, Result};
use cutroom_core::{Clip, Media, Scene, Source, Track};
use std::path::Path;

/// Build a one-take `Source` from plain files. Narration audio comes from `mic`, else the
/// camera's (then the screen recording's) audio track. Words come from `transcript` (JSON, see
/// [`transcript::parse_transcript`]) or, without one, from whisper-cli on the narration audio.
pub fn files(camera: Option<&Path>, screen: Option<&Path>, mic: Option<&Path>, transcript: Option<&Path>) -> Result<Source> {
    let mut src = Source { version: 1, origin: "files".into(), ..Default::default() };
    let mut probes: Vec<(&str, &Path, Probe)> = vec![];
    for (id, p) in [("camera", camera), ("screen", screen), ("mic", mic)] {
        if let Some(p) = p {
            if !p.is_file() {
                bail!("{id} file {} not found", p.display());
            }
            let pr = probe(p).with_context(|| format!("probing {id} {}", p.display()))?;
            if src.name.is_empty() {
                src.name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            }
            probes.push((id, p, pr));
        }
    }
    if probes.is_empty() {
        bail!("give at least one of camera, screen, mic");
    }
    for (id, p, pr) in &probes {
        src.media.push(Media { id: id.to_string(), path: p.to_path_buf(), width: pr.width, height: pr.height, fps: pr.fps, duration_ms: pr.duration_ms });
        let track = match *id {
            "camera" => Some(Track::Camera),
            "screen" => Some(Track::Screen),
            _ => None,
        };
        if let Some(track) = track {
            if !pr.has_video {
                bail!("{id} file {} has no video stream", p.display());
            }
            src.clips.push(Clip { track, media: id.to_string(), start_ms: 0.0, dur_ms: pr.duration_ms, media_in_ms: 0.0, take: 0 });
        }
        src.duration_ms = src.duration_ms.max(pr.duration_ms);
    }
    // narration: the mic file, else the first video file that carries audio
    let audio = probes.iter().find(|(id, _, pr)| *id == "mic" && pr.has_audio).or_else(|| probes.iter().find(|(id, _, pr)| *id != "mic" && pr.has_audio));
    if let Some((id, _, pr)) = audio {
        src.clips.push(Clip { track: Track::Mic, media: id.to_string(), start_ms: 0.0, dur_ms: pr.duration_ms, media_in_ms: 0.0, take: 0 });
    }
    src.words = match (transcript, audio) {
        (Some(t), _) => transcript::load_transcript(t)?,
        (None, Some((_, p, _))) => transcript::whisper(p).with_context(|| format!("transcribing {}", p.display()))?,
        (None, None) => vec![],
    };
    transcript::tidy(&mut src.words);
    src.clips.retain(|c| c.dur_ms > 0.0);
    src.scenes.push(Scene { name: src.name.clone(), start_ms: 0.0, end_ms: src.duration_ms });
    Ok(src)
}
