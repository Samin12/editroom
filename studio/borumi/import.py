#!/usr/bin/env python3
"""Import a finished editroom video into its own Borumi project.

    python3 import.py "<work dir>"                       # newest out/<name> - editroom vN.mp4
    python3 import.py "<work dir>" --video final.mp4 --name "tips raw - editroom"
    python3 import.py "<work dir>" --films               # also add the section films as Media
    python3 import.py "<work dir>" --dry-run             # say what would happen, write nothing

What it does, in one checked Borumi transaction:
  1. finds or creates the Borumi project "<name> - editroom" (never any other project)
  2. clears the previous editroom clip from that project's "editroom" scene (re-runs replace it)
  3. imports the mp4, places it as one full-frame take (screen + system sound, no camera, no
     face tracking, no voice enhancement), pins a fullscreen layout over it
  4. checks the staged timeline (duration, segments, a rendered frame against the mp4) and
     commits; on any mismatch it aborts and nothing changes
Then it reads the committed project back and, with --export-check, exports a 10 s range through
Borumi and compares a frame with the mp4.

Safety: writes go only to projects editroom created (listed in ~/.editroom/borumi-projects.json)
whose name ends with " - editroom". The source project is never opened for writing. Borumi's MCP
cannot delete projects or Media, so nothing is ever deleted outside the editroom scene's clips.
"""
import argparse
import glob
import json
import os
import re
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bmcp import Borumi, BorumiError  # noqa: E402

REGISTRY = os.path.expanduser(os.environ.get("EDITROOM_HOME", "~/.editroom") + "/borumi-projects.json")
PROJECTS_DIR = os.path.expanduser("~/Borumi Projects")
SUFFIX = " - editroom"
SCENE = "editroom"
GUIDES = ["project_edits", "scripting", "editing", "importing_media", "editing_canvas",
          "editing_segment_take_add", "editing_layouts", "editing_segment_layout_add",
          "editing_segment_video_update", "editing_segment_audio_update"]


# ---------------------------------------------------------------- helpers
def say(msg):
    print(f"borumi-import: {msg}", flush=True)


def ffprobe(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-print_format", "json", "-show_format", "-show_streams", path],
                         capture_output=True, text=True, check=True).stdout
    d = json.loads(out)
    v = next((s for s in d["streams"] if s["codec_type"] == "video"), None)
    a = next((s for s in d["streams"] if s["codec_type"] == "audio"), None)
    return {"duration_ms": float(d["format"]["duration"]) * 1000, "width": v and v["width"], "height": v and v["height"],
            "has_audio": a is not None}


def psnr(a, b):
    """PSNR (dB) between two images, b scaled to a's size."""
    r = subprocess.run(["ffmpeg", "-hide_banner", "-i", a, "-i", b, "-lavfi",
                        "[1][0]scale2ref[b][a];[a]format=yuv420p[a2];[b]format=yuv420p[b2];[a2][b2]psnr", "-f", "null", "-"],
                       capture_output=True, text=True)
    m = re.search(r"average:([\d.]+|inf)", r.stderr)
    return float("inf") if m and m.group(1) == "inf" else (float(m.group(1)) if m else 0.0)


def frame_of(video, t_ms, out):
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t_ms / 1000:.3f}", "-i", video, "-frames:v", "1", out], check=True)
    return out


def fingerprint(path):
    """Size, mtime and a hash of the first and last 4 MB: enough to tell renders apart without
    reading 800 MB."""
    import hashlib
    st = os.stat(path)
    h = hashlib.sha1()
    with open(path, "rb") as f:
        h.update(f.read(4 << 20))
        if st.st_size > 8 << 20:
            f.seek(-(4 << 20), os.SEEK_END)
            h.update(f.read())
    return f"{st.st_size}:{h.hexdigest()[:16]}"


def load_registry():
    if os.path.exists(REGISTRY):
        return json.load(open(REGISTRY))
    return {"projects": []}


def save_registry(reg):
    os.makedirs(os.path.dirname(REGISTRY), exist_ok=True)
    tmp = REGISTRY + ".tmp"
    json.dump(reg, open(tmp, "w"), indent=1)
    os.replace(tmp, REGISTRY)


def registered(reg, path):
    return next((e for e in reg["projects"] if os.path.realpath(e["path"]) == os.path.realpath(path)), None)


def assert_writable(reg, proj):
    """The one gate in front of every Borumi write."""
    if not proj["name"].endswith(SUFFIX):
        raise SystemExit(f"refusing to write to Borumi project {proj['name']!r}: its name does not end with {SUFFIX!r}")
    if not registered(reg, proj["path"]):
        raise SystemExit(f"refusing to write to Borumi project {proj['name']!r}: editroom did not create it "
                         f"(not in {REGISTRY}); pass a different --name")


def newest_render(work):
    outs = glob.glob(os.path.join(work, "out", "*.mp4"))

    def ver(p):
        m = re.search(r"editroom v(\d+)\.mp4$", p)
        return (int(m.group(1)) if m else -1, os.path.getmtime(p))

    outs = [p for p in outs if not p.endswith(".preview.mp4")]
    return max(outs, key=ver) if outs else None


def section_films(work):
    """Finished section films: render-status.json `done` paths under studio/jobs, else studio/films/*."""
    films = []
    for st in sorted(glob.glob(os.path.join(work, "studio", "jobs", "*", "render-status.json"))):
        try:
            s = json.load(open(st))
        except ValueError:
            continue
        p = s.get("path")
        if s.get("state") == "done" and p and os.path.exists(p) and not s.get("draft"):
            films.append(p)
    if not films:
        films = sorted(glob.glob(os.path.join(work, "studio", "films", "*.mp4")) + glob.glob(os.path.join(work, "studio", "films", "*.mov")))
    return films


def default_name(work):
    base = os.path.basename(os.path.normpath(work))
    base = re.sub(r"\s+editroom$", "", base)
    return base + SUFFIX


# ---------------------------------------------------------------- Borumi steps
def find_project(bm, name):
    """The project with exactly this name, from the open and recent lists (ids are aliases that only
    exist on this connection, so always look up by path or name, never reuse a stored id)."""
    seen = {}
    for p in bm.call("list_recent_projects").get("projects", []):
        seen[p["path"]] = dict(p, is_open=False)
    for p in bm.call("list_open_projects").get("projects", []):
        seen[p["path"]] = dict(seen.get(p["path"], {}), **p, is_open=True)
    return [p for p in seen.values() if p.get("name") == name]


def ensure_project(bm, reg, name, work, dry):
    matches = find_project(bm, name)
    path = os.path.join(PROJECTS_DIR, name + ".bmprojbundle")
    if len(matches) > 1:
        raise SystemExit(f"more than one Borumi project is named {name!r}; pass --name")
    if matches:
        proj = matches[0]
        assert_writable(reg, proj)
    else:
        if os.path.exists(path) and not registered(reg, path):
            raise SystemExit(f"{path} exists but editroom did not create it; pass a different --name")
        if dry:
            say(f"would create Borumi project {name!r}")
            return {"name": name, "path": path, "id": None, "created": True}
        proj = bm.call("create_project", {"name": name})["project"]
        reg["projects"].append({"name": name, "path": proj["path"], "work": work, "by": "editroom borumi import",
                                "created": time.strftime("%Y-%m-%dT%H:%M:%S")})
        save_registry(reg)
        say(f"created Borumi project {name!r} at {proj['path']}")
        proj["created"] = True
    if not any(p["path"] == proj["path"] for p in bm.call("list_open_projects")["projects"]):
        bm.call("open_project", {"path": proj["path"]})
    proj["id"] = next(p["id"] for p in bm.call("list_open_projects")["projects"] if p["path"] == proj["path"])
    return proj


def hash_of(bm, tx):
    return bm.call("get_project_overview", {"tx_id": tx, "slices": ["project"]})["timeline_hash"]


def scene_of(bm, tx):
    ov = bm.call("get_project_overview", {"tx_id": tx, "slices": ["project", "scenes", "canvas"]})
    return ov, next((s for s in ov.get("scenes", []) if s.get("name") == SCENE), None)


def timeline(bm, tx, start, end):
    if end <= start:
        return {"layers": [], "scenes": []}
    return bm.call("get_timeline", {"tx_id": tx, "range": {"start_ms": int(start), "end_ms": int(end)},
                                    "detail": "segments", "segment_fields": ["all"]})


def stage_import(bm, proj, video, info, log):
    """One transaction: clear the editroom scene, import, place, pin the layout, verify. Returns the
    staged facts; the caller commits."""
    tx = bm.call("begin_project_edit", {"project_id": proj["id"]})["tx_id"]
    try:
        ov, scene = scene_of(bm, tx)
        # canvas follows the video (1920x1080 is the default landscape preset)
        fmt = ov["canvas"]["format"]
        if (fmt.get("width_px"), fmt.get("height_px")) != (info["width"], info["height"]):
            bm.call("update_canvas", {"tx_id": tx, "format": {"type": "custom", "width_px": info["width"], "height_px": info["height"]}})
            log.append(f"canvas set to {info['width']}x{info['height']}")
        if scene is None:
            bm.call("create_scenes", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "destination": {"type": "start"},
                                      "scenes": [{"name": SCENE, "script_markdown": "The finished edit, rendered by editroom. Re-running the import replaces this scene's clip."}]})
            ov, scene = scene_of(bm, tx)
            log.append("created scene 'editroom'")
        else:
            # previous editroom clip(s): every content segment inside this scene
            tl = timeline(bm, tx, scene["start_ms"], scene["end_ms"])
            old = [s["id"] for L in tl.get("layers", []) if L.get("category") == "content"
                   for s in L.get("segments", []) if s.get("scene_id") == scene["id"]]
            if old:
                # content only: Borumi removes the layout that pointed at the clip along with it, and
                # naming that layout in the same call makes the whole delete fail
                bm.call("delete_segments", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "segment_ids": old, "ripple": True})
                log.append(f"removed {len(old)} previous editroom segment(s)")
            ov, scene = scene_of(bm, tx)
            tl = timeline(bm, tx, scene["start_ms"], scene["end_ms"])
            left = [s["id"] for L in tl.get("layers", []) if L.get("id") == "layout" for s in L.get("segments", [])
                    if s["start_ms"] >= scene["start_ms"] - 1 and s["end_ms"] <= scene["end_ms"] + 1]
            if left:
                bm.call("delete_segments", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "segment_ids": left, "ripple": False})
                ov, scene = scene_of(bm, tx)
        at = int(scene["start_ms"])

        im = bm.call("import_media", {"tx_id": tx, "file_path": os.path.abspath(video)}, timeout=1800)
        while im.get("status") in ("queued", "running"):
            time.sleep(max(1.0, im.get("poll_after_ms", 1000) / 1000))
            im = bm.call("get_media_import_status", {"import_id": im["import_id"]})
        if im.get("status") != "completed":
            raise RuntimeError(f"import {im.get('status')}: {json.dumps(im)[:500]}")
        media = im["media"]
        if abs(media.get("duration_ms", 0) - info["duration_ms"]) > 50 or not {"video", "audio"} <= set(media.get("tracks", [])):
            raise RuntimeError(f"Borumi read the file differently: {json.dumps(media)[:400]}")
        log.append(f"imported {os.path.basename(video)} ({media['duration_ms'] / 1000:.1f}s)")

        bm.call("add_segments", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "additions": [
            {"type": "take", "placement": {"at_ms": at},
             "content": {"media_id": media["id"], "video_kind": "screen", "audio_kind": "system_sound"}}]})
        end = at + media["duration_ms"]
        tl = timeline(bm, tx, at, end)
        mine = [(L, s) for L in tl["layers"] if L.get("category") == "content" for s in L["segments"]
                if (s.get("media") or {}).get("media_id") == media["id"]]
        vids = [(L, s) for L, s in mine if s["type"] == "video"]
        auds = [(L, s) for L, s in mine if s["type"] == "audio"]
        if len(vids) != 1 or len(auds) != 1:
            raise RuntimeError(f"expected one video and one audio segment, got {len(vids)} and {len(auds)}")
        (vl, vs), (al, as_) = vids[0], auds[0]
        bm.call("update_segments", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "updates": [
            {"type": "video", "segment_id": vs["id"], "properties": {"face_tracking_mode": "disabled", "transition": {"kind": "cut"}}},
            {"type": "audio", "segment_id": as_["id"], "properties": {"voice_enhancement_filter": "disabled"}}]})
        bm.call("add_segments", {"tx_id": tx, "timeline_hash": hash_of(bm, tx), "additions": [
            {"type": "layout", "placement": {"range": [at, int(end)]},
             "properties": {"kind": "fullscreen", "layer_id": vl["id"], "fit": "cover", "transition": {"kind": "instant"}}}]})

        # ---- verify the staged result before committing
        tl = timeline(bm, tx, at, end)
        segs = {s["id"]: s for L in tl["layers"] for s in L["segments"]}
        v, a = segs.get(vs["id"]), segs.get(as_["id"])
        lays = [s for L in tl["layers"] if L["id"] == "layout" for s in L["segments"]]
        problems = []
        if not v or abs(v["start_ms"] - at) > 1 or abs(v["end_ms"] - end) > 34:
            problems.append(f"video segment {v and (v['start_ms'], v['end_ms'])} != {(at, end)}")
        if not a or abs(a["end_ms"] - v["end_ms"]) > 34:
            problems.append("audio does not span the video")
        if (v or {}).get("properties", {}).get("face_tracking_mode") != "disabled":
            problems.append("face tracking still on")
        if not lays or lays[0]["properties"].get("kind") != "fullscreen":
            problems.append("fullscreen layout missing")
        mid = at + media["duration_ms"] / 2
        fr = bm.call("inspect_timeline", {"tx_id": tx, "range": {"start_ms": int(mid), "end_ms": int(mid) + 34},
                                          "view": {"type": "render"}, "presentation": "frames", "quality": "max",
                                          "sampling": {"type": "uniform", "max_frames": 1}, "save": True, "include_images": False},
                     timeout=300)["frames"][0]
        tmp = tempfile.mkdtemp(prefix="editroom-borumi-")
        # the preview is a JPEG on Borumi's own 30 fps clock: best match within +-2 frames
        db = max(psnr(frame_of(video, max(0, fr["time_ms"] - at + off), os.path.join(tmp, f"src{off}.png")), fr["path"])
                 for off in (-67, -33, 0, 33, 67))
        if db < 30:
            problems.append(f"staged frame differs from the mp4 (PSNR {db:.1f} dB)")
        if problems:
            raise RuntimeError("staged import failed verification: " + "; ".join(problems))
        log.append(f"staged frame matches the mp4 (PSNR {db:.1f} dB at {fr['time_ms'] / 1000:.1f}s)")
        return tx, {"at_ms": at, "end_ms": end, "media": media, "psnr_staged": round(db, 1), "frame": fr["path"]}
    except BaseException:
        try:
            bm.call("abort_project_edit", {"tx_id": tx})
        except BorumiError:
            pass
        raise


def stage_films(bm, proj, films, log):
    tx = bm.call("begin_project_edit", {"project_id": proj["id"]})["tx_id"]
    try:
        names = []
        for f in films:
            im = bm.call("import_media", {"tx_id": tx, "file_path": os.path.abspath(f)}, timeout=1800)
            while im.get("status") in ("queued", "running"):
                time.sleep(max(1.0, im.get("poll_after_ms", 1000) / 1000))
                im = bm.call("get_media_import_status", {"import_id": im["import_id"]})
            if im.get("status") != "completed":
                raise RuntimeError(f"{os.path.basename(f)}: import {im.get('status')}")
            names.append(im["media"]["name"])
        log.append(f"imported {len(names)} section film(s) as Media (not on the timeline)")
        return tx, names
    except BaseException:
        try:
            bm.call("abort_project_edit", {"tx_id": tx})
        except BorumiError:
            pass
        raise


def export_check(bm, proj, video, at, dur_ms, out_dir, seconds=10):
    """Export a short range of the committed project through Borumi and compare a frame to the mp4."""
    bm.guides("exporting")
    s0 = int(at + max(0, dur_ms / 2 - seconds * 500))
    s1 = int(min(at + dur_ms, s0 + seconds * 1000))
    path = os.path.join(out_dir, "borumi-export-check.mp4")
    ex = bm.call("export_video", {"project_id": proj["id"], "range": {"type": "time", "start_ms": s0, "end_ms": s1},
                                  "settings": {"type": "custom", "width_px": 1920, "height_px": 1080, "fps": 30, "quality": "high"},
                                  "output_path": path})
    t0 = time.time()
    st = ex
    while st.get("status") not in ("completed", "failed", "canceled") and time.time() - t0 < 900:
        time.sleep(max(1.0, ex.get("poll_after_ms", 5000) / 1000))
        st = bm.call("get_export_status", {"export_id": ex["export_id"]})
    if st.get("status") != "completed":
        raise RuntimeError(f"export {st.get('status')}: {json.dumps(st)[:400]}")
    art = (st.get("artifact") or {}).get("path") or path
    tmp = tempfile.mkdtemp(prefix="editroom-borumi-")
    mid = (s1 - s0) / 2
    exp = frame_of(art, mid, os.path.join(tmp, "exp.png"))
    # Borumi resamples the timeline on its own 30 fps clock and a range export can start a frame or
    # two early, so compare against the mp4 within +-2 frames and keep the best match
    best = max(((psnr(frame_of(video, s0 - at + mid + off, os.path.join(tmp, f"src{off}.png")), exp), off)
                for off in (-67, -33, 0, 33, 67)))
    d = ffprobe(art)
    return {"export": art, "range_ms": [s0, s1], "export_duration_ms": round(d["duration_ms"]), "psnr_db": round(best[0], 1),
            "offset_ms": best[1], "frame": exp, "seconds": round(time.time() - t0, 1)}


# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("work", help="editroom work dir (~/Borumi Projects/<name> editroom)")
    ap.add_argument("--video", help="the finished mp4 (default: newest out/* - editroom vN.mp4)")
    ap.add_argument("--name", help=f"Borumi project name (must end with {SUFFIX!r}; default '<name>{SUFFIX}')")
    ap.add_argument("--films", action="store_true", help="also import the section films as Media")
    ap.add_argument("--films-from", help="folder of films to import instead of studio/jobs renders")
    ap.add_argument("--export-check", action="store_true", help="export 10 s through Borumi and compare a frame")
    ap.add_argument("--focus", action="store_true", help="show the project in Borumi when done")
    ap.add_argument("--force", action="store_true", help="re-import even when this exact video is already in the project")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()

    work = os.path.abspath(os.path.expanduser(a.work))
    video = os.path.abspath(a.video) if a.video else newest_render(work)
    if not video or not os.path.exists(video):
        raise SystemExit(f"no finished video: pass --video, or render one into {work}/out/")
    name = a.name or default_name(work)
    if not name.endswith(SUFFIX):
        raise SystemExit(f"--name must end with {SUFFIX!r} (got {name!r})")
    info = ffprobe(video)
    if not info["width"] or not info["has_audio"]:
        raise SystemExit(f"{video}: needs a video and an audio stream")
    films = []
    if a.films:
        films = (sorted(glob.glob(os.path.join(a.films_from, "*.mp4")) + glob.glob(os.path.join(a.films_from, "*.mov")))
                 if a.films_from else section_films(work))
        if not films:
            say("no section films found (studio/jobs/*/render-status.json or studio/films/); skipping --films")

    reg = load_registry()
    report = {"work": work, "video": video, "name": name, "started": time.strftime("%Y-%m-%dT%H:%M:%S"), "log": []}
    t0 = time.time()
    with Borumi(log=os.path.join(tempfile.gettempdir(), "editroom-borumi-mcp.jsonl")) as bm:
        bm.guides(*GUIDES)
        proj = ensure_project(bm, reg, name, work, a.dry_run)
        report["project"] = {"name": proj["name"], "path": proj["path"], "created": bool(proj.get("created"))}
        if a.dry_run:
            say(f"would import {os.path.basename(video)} ({info['duration_ms'] / 1000:.1f}s) into {proj['path']}"
                + (f" and {len(films)} film(s)" if films else ""))
            return
        entry = registered(reg, proj["path"])
        fp = fingerprint(video)
        ov0 = bm.call("get_project_overview", {"project_id": proj["id"], "slices": ["scenes"]})
        sc0 = next((x for x in ov0.get("scenes", []) if x.get("name") == SCENE), None)
        up_to_date = (entry.get("last_import", {}).get("fingerprint") == fp and sc0 is not None
                      and abs((sc0["end_ms"] - sc0["start_ms"]) - info["duration_ms"]) <= 50)
        if up_to_date and not a.force:
            report["log"].append(f"{os.path.basename(video)} is already the editroom scene's clip; nothing to import (--force to redo)")
        else:
            tx, staged = stage_import(bm, proj, video, info, report["log"])
            summary = (f"Replace the editroom scene with the finished render {os.path.basename(video)} "
                       f"({info['duration_ms'] / 1000:.1f} s), full frame.")
            c = bm.call("commit_project_edit", {"tx_id": tx, "change_summary": summary[:480]})
            report["commit"] = c.get("commit_id")
            report["staged"] = {k: v for k, v in staged.items() if k != "media"}
            report["log"].append("committed")
            entry["last_import"] = {"video": video, "fingerprint": fp, "duration_ms": round(info["duration_ms"]),
                                    "at": time.strftime("%Y-%m-%dT%H:%M:%S"), "commit": report["commit"]}
            save_registry(reg)
        if films:
            done = entry.setdefault("films", {})
            todo = [f for f in films if done.get(fingerprint(f)) is None or a.force]
            if todo:
                ftx, names = stage_films(bm, proj, todo, report["log"])
                bm.call("commit_project_edit", {"tx_id": ftx, "change_summary": f"Add {len(names)} section films as Media for later edits."})
                for f, n in zip(todo, names):
                    done[fingerprint(f)] = {"file": f, "media_name": n}
                save_registry(reg)
                report["films"] = names
            else:
                report["log"].append(f"all {len(films)} section films are already in the project")
        # read the committed project back
        ov = bm.call("get_project_overview", {"project_id": proj["id"], "slices": ["project", "scenes", "content"]})
        sc = next(s for s in ov["scenes"] if s["name"] == SCENE)
        report["committed"] = {"duration_ms": ov["project"]["duration_ms"], "scene_ms": [sc["start_ms"], sc["end_ms"]],
                               "content": ov.get("content")}
        if abs((sc["end_ms"] - sc["start_ms"]) - info["duration_ms"]) > 50:
            raise SystemExit(f"committed scene is {(sc['end_ms'] - sc['start_ms']) / 1000:.2f}s, the video {info['duration_ms'] / 1000:.2f}s")
        if a.export_check:
            report["export_check"] = export_check(bm, proj, video, sc["start_ms"], info["duration_ms"],
                                                  os.path.join(work, "studio") if os.path.isdir(os.path.join(work, "studio")) else tempfile.mkdtemp())
        if a.focus:
            bm.guides("ui_navigation")
            bm.call("focus_project", {"project_id": proj["id"]})
    report["seconds"] = round(time.time() - t0, 1)
    os.makedirs(os.path.join(work, "studio"), exist_ok=True)
    json.dump(report, open(os.path.join(work, "studio", "borumi-import.json"), "w"), indent=1)
    for line in report["log"]:
        say(line)
    say(f"done in {report['seconds']}s: {proj['path']} ({report['committed']['duration_ms'] / 1000:.1f}s)"
        + (f"; export check PSNR {report['export_check']['psnr_db']} dB" if a.export_check else ""))


if __name__ == "__main__":
    try:
        main()
    except BorumiError as e:
        raise SystemExit(f"borumi-import: Borumi refused: {e}")
