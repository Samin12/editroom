use cutroom_core::{Source, Track};
use cutroom_ingest::{borumi, borumi_db, files, BorumiMode, Project};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("cutroom-ingest-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ffmpeg(args: &[&str]) {
    let st = Command::new("ffmpeg").args(["-nostdin", "-loglevel", "error", "-y"]).args(args).status().expect("ffmpeg");
    assert!(st.success(), "ffmpeg {args:?}");
}

fn tokens(words: &[(&str, f64, f64)]) -> String {
    let toks: Vec<String> = words.iter().map(|(t, s, e)| format!(r#"{{"text":"{t}","start":{s},"end":{e}}}"#)).collect();
    format!(r#"{{"kind":"Success","transcription":{{"segments":[{{"tokens":[{}]}}],"engine":"Parakeet"}}}}"#, toks.join(","))
}

/// A tiny Borumi bundle: scene A (seq "a~") and scene B (seq "aB", so B sorts first by bytes).
/// Scene B: take tb (cam + mic, mic offset 0.05 s). Scene A: takes ta2 (started earlier) and ta1,
/// plus an imported b-roll take and a take whose mic has no transcript (not narration).
fn bundle(tag: &str) -> PathBuf {
    let dir = tmp(tag).join("Test.bmprojbundle");
    let medias = dir.join("medias");
    std::fs::create_dir_all(&medias).unwrap();
    let cam = medias.join("cam1.mp4");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=160x90:rate=25:duration=4", "-pix_fmt", "yuv420p", cam.to_str().unwrap()]);
    let mic = medias.join("mic1.wav");
    ffmpeg(&["-f", "lavfi", "-i", "sine=frequency=440:duration=4", mic.to_str().unwrap()]);

    let db = Connection::open(dir.join("project.bmproj")).unwrap();
    db.execute_batch(
        "CREATE TABLE scenes (id TEXT PRIMARY KEY, project_id TEXT, seq TEXT, name TEXT, script TEXT);
         CREATE TABLE takes (id TEXT PRIMARY KEY, scene_id TEXT, name TEXT, started_at REAL, ended_at REAL, record_over_mappings TEXT, record_over INTEGER);
         CREATE TABLE clips (id TEXT PRIMARY KEY, take_id TEXT, media TEXT, source TEXT, start_offset REAL, duration REAL, media_track TEXT, start_offset_adjustment REAL);
         CREATE TABLE medias (id TEXT PRIMARY KEY, project_id TEXT, kind TEXT, status TEXT, source TEXT, project_source TEXT, duration REAL, transcription_status TEXT, hidden_cursor INTEGER, prefers_mirrored INTEGER, tracks TEXT, audio_stats TEXT, rotation INTEGER, inferred_video_kind TEXT, inferred_audio_kind TEXT, name TEXT, width INTEGER, height INTEGER);
         CREATE TABLE project_sources (id TEXT PRIMARY KEY, project_id TEXT, name TEXT, kind TEXT, seq TEXT, enabled INTEGER, preferred_source_affinities TEXT, preferred_source_configuration TEXT);
         CREATE TABLE scene_timeline_segments (id TEXT PRIMARY KEY, scene_id TEXT, start REAL, duration REAL, edited INTEGER, segment_group TEXT, kind TEXT, segment_order INTEGER, created_at REAL);
         INSERT INTO project_sources (id, name, kind) VALUES
           ('ps_cam','Camera','{\"kind\":\"Camera\"}'), ('ps_scr','Screen','{\"kind\":\"Screen\",\"is_window\":false}'),
           ('ps_mic','Microphone','{\"kind\":\"Microphone\"}'), ('ps_sys','System Sound','{\"kind\":\"SystemSound\"}');
         INSERT INTO scenes (id, seq, name) VALUES ('sA', 'a~', 'Chapter: Setup'), ('sB', 'aB', '');
         INSERT INTO takes (id, scene_id, name, started_at) VALUES
           ('tb','sB','',10), ('ta1','sA','',300), ('ta2','sA','',200), ('troll','sA','F049_1_2',400), ('tmute','sA','',500);",
    )
    .unwrap();
    let media = |id: &str, ps: Option<&str>, dur: f64, ts: Option<String>, tracks: &str, wh: Option<(i64, i64)>| {
        db.execute(
            "INSERT INTO medias (id, project_source, duration, transcription_status, tracks, width, height) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, ps, dur, ts, tracks, wh.map(|x| x.0), wh.map(|x| x.1)],
        )
        .unwrap();
    };
    media("cam1", Some("ps_cam"), 4.0, None, r#"[{"kind":"Video"}]"#, Some((160, 90)));
    media("mic1", Some("ps_mic"), 4.0, Some(tokens(&[("Hello", 0.5, 0.9), ("there", 0.8, 1.2), ("friend.", 2.0, 2.5)])), r#"[{"kind":"Audio"}]"#, None);
    media("mic2", Some("ps_mic"), 3.0, Some(tokens(&[("Second", 0.2, 0.6), ("take.", 0.6, 1.0)])), r#"[{"kind":"Audio"}]"#, None);
    media("mic3", Some("ps_mic"), 2.0, Some(tokens(&[("Third", 0.1, 0.4)])), r#"[{"kind":"Audio"}]"#, None);
    media("scr3", Some("ps_scr"), 2.5, None, r#"[{"kind":"Video"}]"#, Some((5134, 2454)));
    media("roll", None, 6.0, None, r#"[{"kind":"Video"}]"#, Some((1920, 1080)));
    media("mute", Some("ps_mic"), 5.0, None, r#"[{"kind":"Audio"}]"#, None);
    let clip = |id: &str, take: &str, media: &str, off: f64, dur: Option<f64>, track: &str| {
        db.execute("INSERT INTO clips (id, take_id, media, start_offset, duration, media_track) VALUES (?1,?2,?3,?4,?5,?6)", params![id, take, media, off, dur, format!(r#"{{"kind":"{track}"}}"#)]).unwrap();
    };
    clip("c_cam1", "tb", "cam1", 0.0, None, "Video");
    clip("c_mic1", "tb", "mic1", 0.05, None, "Audio");
    clip("c_mic2", "ta2", "mic2", 0.0, None, "Audio");
    clip("c_mic3", "ta1", "mic3", 0.0, None, "Audio");
    clip("c_scr3", "ta1", "scr3", 0.02, None, "Video");
    clip("c_roll", "troll", "roll", 0.0, Some(5.0), "Video");
    clip("c_mute", "tmute", "mute", 0.0, None, "Audio");
    let seg = |id: &str, scene: &str, start: f64, dur: f64, kind: &str, take: &str, clip: &str, clip_start: f64, content: &str| {
        let k = format!(r#"{{"kind":"{kind}","scene_id":"{scene}","take_id":"{take}","clip_id":"{clip}","clip_start":{clip_start},"content_kind":{{"kind":"{content}"}},"speed":null}}"#);
        db.execute("INSERT INTO scene_timeline_segments (id, scene_id, start, duration, segment_group, kind, segment_order) VALUES (?1,?2,?3,?4,?5,?6,0)", params![id, scene, start, dur, id, k]).unwrap();
    };
    // scene B keeps take-time 0.4..1.4 ("Hello there") and 1.9..2.6 ("friend.")
    seg("g1c", "sB", 0.0, 1.0, "VideoClip", "tb", "c_cam1", 0.4, "Camera");
    seg("g1m", "sB", 0.0, 1.0, "AudioClip", "tb", "c_mic1", 0.4, "Microphone");
    seg("g2c", "sB", 1.0, 0.7, "VideoClip", "tb", "c_cam1", 1.9, "Camera");
    seg("g2m", "sB", 1.0, 0.7, "AudioClip", "tb", "c_mic1", 1.9, "Microphone");
    // scene A shows b-roll then take ta1
    seg("g3", "sA", 0.0, 2.0, "VideoClip", "troll", "c_roll", 1.0, "Screen");
    seg("g4", "sA", 2.0, 1.0, "AudioClip", "ta1", "c_mic3", 0.0, "Microphone");
    db.execute(
        "INSERT INTO scene_timeline_segments (id, scene_id, start, duration, segment_group, kind, segment_order) VALUES ('lay','sA',0,3.5,'lay','{\"kind\":\"Layout\"}',0)",
        [],
    )
    .unwrap();
    dir
}

fn words(s: &Source) -> Vec<(&str, f64, f64)> {
    s.words.iter().map(|w| (w.text.as_str(), (w.start_ms * 10.0).round() / 10.0, (w.end_ms * 10.0).round() / 10.0)).collect()
}

fn sorted_disjoint(s: &Source) -> bool {
    s.words.windows(2).all(|w| w[0].start_ms <= w[1].start_ms && w[0].end_ms <= w[1].start_ms + 1e-9) && s.words.iter().all(|w| w.end_ms >= w.start_ms)
}

#[test]
fn borumi_raw_mode() {
    let b = bundle("raw");
    let before = std::fs::read(b.join("project.bmproj")).unwrap();
    let s = borumi(&b, BorumiMode::Raw).unwrap();
    assert_eq!(std::fs::read(b.join("project.bmproj")).unwrap(), before, "bundle DB must not change");
    assert!(s.validate().is_empty(), "{:?}", s.validate());
    assert!(sorted_disjoint(&s));
    assert_eq!(s.name, "Test");
    // scene B (seq "aB") first: take tb is 4.05 s long (mic offset 0.05 + 4 s)
    // then scene A: ta2 (started earlier, 3 s) then ta1 (2.52 s: screen offset 0.02 + 2.5 s)
    assert_eq!(s.scenes.len(), 2);
    assert_eq!(s.scenes[0].name, "Scene 1");
    assert_eq!((s.scenes[0].start_ms, s.scenes[0].end_ms), (0.0, 4050.0));
    assert_eq!(s.scenes[1].name, "Chapter: Setup");
    assert!((s.scenes[1].end_ms - (4050.0 + 3000.0 + 2520.0)).abs() < 1e-6);
    assert!((s.duration_ms - 9570.0).abs() < 1e-6);
    assert_eq!(
        words(&s),
        vec![("Hello", 550.0, 850.0), ("there", 850.0, 1250.0), ("friend.", 2050.0, 2550.0), ("Second", 4250.0, 4650.0), ("take.", 4650.0, 5050.0), ("Third", 7150.0, 7450.0)]
    );
    // no b-roll, no untranscribed take
    assert!(s.clips.iter().all(|c| c.media != "roll" && c.media != "mute"));
    let mic1 = s.clips.iter().find(|c| c.media == "mic1").unwrap();
    assert_eq!((mic1.track, mic1.start_ms, mic1.media_in_ms, mic1.take), (Track::Mic, 50.0, 0.0, 0));
    let scr = s.clips.iter().find(|c| c.media == "scr3").unwrap();
    assert_eq!((scr.track, scr.take), (Track::Screen, 2));
    assert!((scr.start_ms - 7070.0).abs() < 1e-6);
    // media: real files probed (fps), DB sizes kept
    let cam = s.media("cam1").unwrap();
    assert!(cam.path.ends_with("medias/cam1.mp4"));
    assert_eq!((cam.width, cam.height), (160, 90));
    assert!((cam.fps - 25.0).abs() < 0.01, "fps {}", cam.fps);
    assert!(s.media("mic1").unwrap().path.ends_with("medias/mic1.wav"));
    assert_eq!(s.media("scr3").unwrap().width, 5134);
}

#[test]
fn borumi_timeline_mode() {
    let b = bundle("timeline");
    let s = borumi_db(&b.join("project.bmproj"), &b.join("medias"), BorumiMode::Timeline).unwrap();
    assert!(s.validate().is_empty(), "{:?}", s.validate());
    assert!(sorted_disjoint(&s));
    // scene B: 1.7 s, scene A: 3.0 s (the layout segment running past the clips does not count)
    assert_eq!(s.scenes.len(), 2);
    assert!((s.scenes[0].end_ms - 1700.0).abs() < 1e-6);
    assert!((s.scenes[1].end_ms - 4700.0).abs() < 1e-6);
    // g1m plays mic media 350..1350 (take 400 - offset 50) at 0: Hello 500..900 -> 150..550,
    // there 800..1200 -> 450 (clamped) .. 850; g2m plays 1850..2550 at 1000: friend. 2000..2500
    assert_eq!(words(&s), vec![("Hello", 150.0, 450.0), ("there", 450.0, 850.0), ("friend.", 1150.0, 1650.0), ("Third", 3800.0, 4100.0)]);
    let roll = s.clips.iter().find(|c| c.media == "roll").unwrap();
    assert_eq!((roll.track, roll.start_ms, roll.media_in_ms), (Track::Screen, 1700.0, 1000.0));
    // ground truth helper: kept mic ranges in media time
    let p = Project::open(&b).unwrap();
    let kept = p.mic_kept_ranges();
    assert_eq!(kept["mic1"], vec![(350.0, 1350.0), (1850.0, 2550.0)]);
    assert_eq!(kept["mic3"], vec![(0.0, 1000.0)]);
    assert_eq!(p.narration_takes().iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["tb", "ta2", "ta1"]);
}

#[test]
fn files_with_transcript_uses_camera_audio() {
    let d = tmp("files");
    let cam = d.join("cam.mp4");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=160x90:rate=30:duration=2", "-f", "lavfi", "-i", "sine=duration=2", "-shortest", "-pix_fmt", "yuv420p", cam.to_str().unwrap()]);
    let scr = d.join("screen.mov");
    ffmpeg(&["-f", "lavfi", "-i", "testsrc=size=320x180:rate=60:duration=3", "-pix_fmt", "yuv420p", scr.to_str().unwrap()]);
    let tj = d.join("t.json");
    std::fs::write(&tj, r#"[{"text":"b","start":0.9,"end":1.4},{"text":"a","start":0.1,"end":1.0}]"#).unwrap();
    let s = files(Some(&cam), Some(&scr), None, Some(&tj)).unwrap();
    assert!(s.validate().is_empty());
    assert_eq!(words(&s), vec![("a", 100.0, 900.0), ("b", 900.0, 1400.0)]);
    let mic = s.clips.iter().find(|c| c.track == Track::Mic).unwrap();
    assert_eq!(mic.media, "camera");
    assert!((s.duration_ms - 3000.0).abs() < 50.0, "{}", s.duration_ms);
    let sm = s.media("screen").unwrap();
    assert_eq!((sm.width, sm.height), (320, 180));
    assert!((sm.fps - 60.0).abs() < 0.01);
    assert!(files(None, None, None, None).is_err());
}

#[test]
fn files_whisper_transcribes() {
    // needs macOS `say`, whisper-cli and a model; skipped quietly when any is missing
    if cutroom_ingest::transcript::whisper_model().is_err() || Command::new("say").arg("-v").arg("?").output().is_err() {
        eprintln!("skipping: no whisper model or `say`");
        return;
    }
    let d = tmp("whisper");
    let aiff = d.join("v.aiff");
    let st = Command::new("say").arg("-o").arg(&aiff).arg("I'm gonna be walking you through the whole setup.").status().unwrap();
    assert!(st.success());
    let mic = d.join("mic.wav");
    ffmpeg(&["-i", aiff.to_str().unwrap(), mic.to_str().unwrap()]);
    let s = files(None, None, Some(&mic), None).unwrap();
    let text: Vec<String> = s.words.iter().map(|w| cutroom_core::anchor::norm(&w.text)).collect();
    assert!(text.contains(&"i'm".to_string()) && text.contains(&"walking".to_string()), "{text:?}");
    assert!(sorted_disjoint(&s));
    assert!(s.words.last().unwrap().end_ms <= s.duration_ms + 1.0);
}

/// The real course project (copy of the DB; media read from the live bundle, metadata only).
/// `cargo test -p cutroom-ingest -- --ignored`
#[test]
#[ignore]
fn real_course_copy() {
    // CUTROOM_EVAL_DB = a copy of project.bmproj, CUTROOM_EVAL_MEDIAS = the bundle's medias/
    let db_s = std::env::var("CUTROOM_EVAL_DB").unwrap_or_else(|_| "/private/tmp/cutroom-probe/yc.bmproj".into());
    let medias_s = std::env::var("CUTROOM_EVAL_MEDIAS").unwrap_or_default();
    let db = Path::new(&db_s);
    let medias = Path::new(&medias_s);
    if !db.exists() || !medias.exists() {
        eprintln!("skipping: {} missing", db.display());
        return;
    }
    for mode in [BorumiMode::Raw, BorumiMode::Timeline] {
        let t = std::time::Instant::now();
        let s = borumi_db(db, medias, mode).unwrap();
        let v = s.validate();
        assert!(v.is_empty(), "{mode:?}: {v:?}");
        assert!(sorted_disjoint(&s), "{mode:?} words overlap");
        let hours = s.duration_ms / 3.6e6;
        eprintln!("{mode:?}: {:.2} h, {} words, {} clips, {} media, {} scenes, {:?}", hours, s.words.len(), s.clips.len(), s.media.len(), s.scenes.len(), t.elapsed());
        assert!(s.media.iter().all(|m| m.path.exists()), "missing media files");
        assert!(s.words.iter().all(|w| w.start_ms >= 0.0 && w.end_ms <= s.duration_ms));
        let cams: Vec<_> = s.media.iter().filter(|m| m.width == 1920).collect();
        assert!(cams.iter().any(|m| (m.fps - 30.0).abs() < 1.0), "camera fps");
        match mode {
            BorumiMode::Raw => {
                assert!((3.0..6.0).contains(&hours), "raw duration {hours}");
                assert!(s.words.len() > 20_000);
            }
            BorumiMode::Timeline => assert!((1.0..4.0).contains(&hours), "timeline duration {hours}"),
        }
    }
}
