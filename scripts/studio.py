#!/usr/bin/env python3
"""editroom studio glue: the deterministic steps between the studio workflows.

Studio mode is the method of the good "10 Opus 5.5 tips" edit (proof pack, beat-by-beat section briefs, one
Remotion section film per section checked by an independent checker, a continuity pass), with the cutroom
engine composing and rendering. This script never calls a model: it prepares inputs for the workflows, records
their state on disk (so every stage resumes) and wires the finished films into the edit.

  studio.py init <W> [--title T] [--creator C] [--direction D] [--today YYYY-MM-DD]
  studio.py resolve <W>                 sections-plan.json -> 00-chapters.yaml, slots, timing files, job.json
  studio.py verify <W>                  exit 3 when the cuts/chapters changed under rendered slots (lists them)
  studio.py status <W> [--json]         per slot: brief, render, check, stale
  studio.py args <stage> <W> [...]      JSON args for a workflow: research | plan | briefs | design | continuity | final-qa
  studio.py wire <W> [--allow-failing]  passing renders -> beats/studio-films.yaml (+ badge groups, card style)
  studio.py timing <W> (--slot KEY | --from wA --to wB) [--json]
  studio.py boxes <png> --phrase "..." [--all] [--json]
  studio.py sheet-video <video> (--every S | --at t1,t2,..) [--out sheet.png] [--cols 4]
  studio.py check-render <W> <KEY> [--render PATH] [--json]
  studio.py log <W> <stage> start|end [--note TEXT]
  studio.py report <W>                  markdown summary (slots, renders, checks, stage timings)

Work dir layout (W = "~/Borumi Projects/<name> editroom"):
  W/studio/state.json            lock (map hash), stage timings, flags
  W/studio/sections-plan.json    the section planner's plan (word anchors, modes, groups, kicker)
  W/studio/sections.json         resolved slots in OUTPUT time (snapped to frames)
  W/studio/timing/<KEY>.json     kept words of a slot: output ms, frame in the slot's clock, on-screen source
  W/studio/proof/                proof pack (groups.json, <NN-group>/, proof-index.json, README.md)
  W/studio/briefs/<KEY>.md       section brief (+ .critic.json, .revised.json)
  W/studio/jobs/<KEY>/           job.json, Scene.tsx, renders, design-report.json, checks/
  W/studio/receipts/             continuity.json, final-qa.json, logs
  W/beats/studio-films.yaml      the films as `video` beats (fit: frame)
"""
import argparse
import datetime as dt
import glob
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ER = os.path.dirname(HERE)
EDITROOM_HOME = os.path.expanduser(os.environ.get("EDITROOM_HOME", "~/.editroom"))
FPS = 30.0
# cutroom's default layout (crates/core/src/spec.rs LayoutSpec): the films keep these areas clear
PANEL = [0.01, 0.0178, 0.7676, 0.9644]
CAMERA = [0.7966, 0.2126, 0.1843, 0.5747]
BADGE = [0.7966, 0.045, 0.184375, 0.138889]
FRAME_W, FRAME_H = 1920, 1080
DRAFT_BEATS = re.compile(r"^(ch\d+|screen|scene-.*|00-chapters)\.ya?ml$")


# ---------------------------------------------------------------------------------------------- helpers

def die(msg, code=1):
    print(f"studio: {msg}", file=sys.stderr)
    sys.exit(code)


def cutroom_bin():
    for c in [os.environ.get("CUTROOM"), os.path.join(EDITROOM_HOME, "bin", "cutroom")]:
        if c and os.path.exists(c):
            return c
    found = shutil.which("cutroom")
    if found:
        return found
    die("no cutroom engine; run scripts/install.sh first")


def run(cmd, check=False, inp=None):
    r = subprocess.run(cmd, capture_output=True, text=True, input=inp)
    if check and r.returncode:
        die(f"{' '.join(cmd[:3])} failed ({r.returncode}):\n{(r.stdout + r.stderr)[-1500:]}")
    return r


_HAS = {}


def has_cmd(name):
    """True when the installed engine has a subcommand (`slots`, `timing`, `boxes`, `check-render`, ...)."""
    if name not in _HAS:
        _HAS[name] = run([cutroom_bin(), name, "--help"]).returncode == 0
    return _HAS[name]


def helpers_merged():
    """The helpers branch adds `slots`; with it come chapter `group`, `output.card_style`, video `fade`."""
    return has_cmd("slots")


def jload(p, default=None):
    try:
        with open(p) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def jsave(p, obj):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    tmp = p + ".tmp"
    with open(tmp, "w") as f:
        json.dump(obj, f, indent=1, ensure_ascii=False)
    os.replace(tmp, p)


def now():
    return dt.datetime.now().astimezone().isoformat(timespec="seconds")


def clock(ms):
    s = max(0.0, ms / 1000.0)
    return f"{int(s // 60)}:{s % 60:04.1f}"


def flow(obj):
    """One compact JSON line (valid YAML flow mapping), like the agent writes."""
    return json.dumps(obj, ensure_ascii=False, separators=(",", ":"))


def studio(w, *p):
    return os.path.join(w, "studio", *p)


def state(w):
    return jload(studio(w, "state.json"), {}) or {}


def save_state(w, st):
    jsave(studio(w, "state.json"), st)


def plan_json(w):
    r = run([cutroom_bin(), "plan", w])
    if r.returncode:
        die(f"cutroom plan failed:\n{(r.stdout + r.stderr)[-1500:]}")
    out = r.stdout
    return json.loads(out[out.index("{"):])


def map_hash(plan):
    m = json.dumps(plan.get("map", {}), sort_keys=True)
    ch = json.dumps([[c.get("at_ms"), c.get("card")] for c in plan.get("chapters", [])], sort_keys=True)
    return hashlib.sha256((m + ch).encode()).hexdigest()[:16]


# ---------------------------------------------------------------------------------------------- time map

class TimeMap:
    """Source ms -> output ms through cutroom's plan map (the same math the engine uses for word anchors)."""

    def __init__(self, w, plan=None):
        self.w = w
        self.plan = plan or plan_json(w)
        self.src = jload(os.path.join(w, "source.json"))
        if not self.src:
            die(f"{w}/source.json missing: run the ingest stage first")
        self.pieces = [p for p in self.plan["map"]["pieces"] if p.get("t") == "src"]
        self.duration = float(self.plan["map"]["duration_ms"])
        self.media = {m["id"]: m for m in self.src.get("media", [])}
        self.screens = [c for c in self.src.get("clips", []) if c.get("track") == "screen"]

    def out_ms(self, s, end=False):
        for p in self.pieces:
            if (p["src0"] <= s < p["src1"]) if not end else (p["src0"] < s <= p["src1"] + 0.5):
                return p["out0"] + (min(s, p["src1"]) - p["src0"])
        return None

    def source_at(self, s):
        for c in self.screens:
            if c["start_ms"] <= s < c["start_ms"] + c["dur_ms"]:
                return "screen"
        return "camera"

    def words(self):
        """Kept words in output order: (index, word dict, out_start, out_end)."""
        out = []
        for i, wd in enumerate(self.src["words"]):
            a = self.out_ms(wd["start_ms"])
            if a is None:
                continue
            b = self.out_ms(wd["end_ms"], end=True)
            out.append((i, wd, a, b if b is not None else a + (wd["end_ms"] - wd["start_ms"])))
        return out

    def screen_segments(self, a, b):
        """Where a screen recording is on screen inside [a, b) output ms: media path + media time mapping."""
        segs = []
        for p in self.pieces:
            for c in self.screens:
                s0 = max(p["src0"], c["start_ms"])
                s1 = min(p["src1"], c["start_ms"] + c["dur_ms"])
                if s1 <= s0:
                    continue
                o0 = p["out0"] + (s0 - p["src0"])
                o1 = p["out0"] + (s1 - p["src0"])
                o0, o1 = max(o0, a), min(o1, b)
                if o1 - o0 < 1:
                    continue
                m = self.media.get(c["media"], {})
                media0 = c.get("media_in_ms", 0.0) + (s0 - c["start_ms"]) + (o0 - (p["out0"] + (s0 - p["src0"])))
                segs.append({"out0": round(o0, 1), "out1": round(o1, 1), "media": m.get("path"), "media_id": c["media"],
                             "media0": round(media0, 1), "width": m.get("width"), "height": m.get("height"),
                             "rule": "media_ms = media0 + (out_ms - out0)"})
        return sorted(segs, key=lambda x: x["out0"])


def word_index(anchor):
    m = re.fullmatch(r"w(\d+)", str(anchor).strip())
    if not m:
        die(f"anchor {anchor!r}: the plan uses word ids (w123)")
    return int(m.group(1))


def snap(ms):
    """Output frame at or before `ms` (slot boundaries floor, so a slot always starts at or before its first word)."""
    return math.floor(ms * FPS / 1000.0 + 1e-6)


def slot_timing(tm, key, slot0, end, kept=None):
    kept = kept if kept is not None else tm.words()
    words, sentences, cur = [], [], None
    for i, wd, a, b in kept:
        if a < slot0 - 1 or a >= end:
            continue
        f = round((a - slot0) * FPS / 1000.0)
        src_s = wd["start_ms"]
        words.append({"id": f"w{i}", "text": wd["text"], "ms": round(a, 1), "end_ms": round(b, 1), "f": f,
                      "source": tm.source_at(src_s)})
        if cur is None:
            cur = {"from": f"w{i}", "ms": round(a, 1), "f": f, "text": wd["text"]}
        else:
            cur["text"] += " " + wd["text"]
        if re.search(r"[.!?]$", wd["text"]):
            sentences.append(cur)
            cur = None
    if cur:
        sentences.append(cur)
    return {"key": key, "slot0": round(slot0, 3), "end": round(end, 3), "fps": FPS,
            "frames": snap(end) - snap(slot0),
            "frame_rule": f"f = round((output_ms - {round(slot0, 3)}) * 30 / 1000); land beats 2-4 frames early",
            "words": words, "sentences": sentences, "screen": tm.screen_segments(slot0, end),
            "computed_by": "studio.py (cutroom plan map + source.json)"}


# ---------------------------------------------------------------------------------------------- init

def cmd_init(a):
    w = os.path.abspath(a.work)
    if not os.path.exists(os.path.join(w, "source.json")):
        die(f"{w} has no source.json: run `pipeline.sh <project> --until setup` first")
    for d in ["proof", "briefs", "jobs", "timing", "receipts"]:
        os.makedirs(studio(w, d), exist_ok=True)
    moved = []
    beats = os.path.join(w, "beats")
    if os.path.isdir(beats) and not a.keep_draft_beats:
        stash = os.path.join(w, "beats-draft", dt.datetime.now().strftime("%Y%m%d-%H%M%S"))
        for f in sorted(os.listdir(beats)):
            if DRAFT_BEATS.match(f):
                os.makedirs(stash, exist_ok=True)
                shutil.move(os.path.join(beats, f), os.path.join(stash, f))
                moved.append(f)
    st = state(w)
    st.setdefault("created", now())
    st["title"] = a.title or st.get("title") or os.path.basename(w).replace(" editroom", "")
    st["creator"] = a.creator or st.get("creator") or "Samin"
    st["today"] = a.today or st.get("today") or dt.date.today().isoformat()
    save_state(w, st)
    db = studio(w, "designer-brief.md")
    if not os.path.exists(db):
        t = open(os.path.join(ER, "studio", "templates", "designer-brief.md")).read()
        captured = dt.date.fromisoformat(st["today"]).strftime("%b %-d, %Y")
        fill = {"TITLE": st["title"], "CREATOR": st["creator"], "W": w, "ER": ER,
                "KIT": os.path.join(ER, "studio", "kit"), "CAPTURED": captured,
                "DIRECTION": (f" For this video: {a.direction}" if a.direction else ""),
                "THIS_VIDEO": "(not written yet: the section planner fills this block)"}
        for k, v in fill.items():
            t = t.replace("{{" + k + "}}", v)
        with open(db, "w") as f:
            f.write(t)
    print(json.dumps({"work": w, "studio": studio(w), "designer_brief": db, "moved_draft_beats": moved,
                      "helpers_merged": helpers_merged()}, indent=1))


# ---------------------------------------------------------------------------------------------- resolve

def write_chapters(w, plan):
    """beats/00-chapters.yaml from the section plan (backs up any existing file)."""
    beats = os.path.join(w, "beats")
    os.makedirs(beats, exist_ok=True)
    p = os.path.join(beats, "00-chapters.yaml")
    if os.path.exists(p):
        bak = os.path.join(w, "beats-draft", "00-chapters.studio-prev.yaml")
        os.makedirs(os.path.dirname(bak), exist_ok=True)
        shutil.copy2(p, bak)
    rich = helpers_merged()
    out = {}
    if plan.get("kicker") is not None:
        out["kicker"] = plan["kicker"]
    if plan.get("intro_title"):
        out["intro"] = plan["intro_title"]
    if rich and plan.get("card_style"):
        out["card_style"] = plan["card_style"]
    lines = ["# written by studio.py resolve from studio/sections-plan.json (edit the plan, then resolve again)"]
    if out:
        lines.append(f"output: {flow(out)}")
    lines.append("chapters:")
    for c in plan.get("chapters", []):
        row = {"at": c["at"], "title": c["title"]}
        if c.get("short"):
            row["short"] = c["short"]
        if rich and c.get("group"):
            row["group"] = c["group"]
        lines.append(f"  - {flow(row)}")
    with open(p, "w") as f:
        f.write("\n".join(lines) + "\n")
    return p, (not rich and any(c.get("group") for c in plan.get("chapters", [])))


def resolve_slots(w, plan, tm):
    chapters = tm.plan.get("chapters", [])  # resolved: at_ms, card [a, b] in output ms
    kept = tm.words()
    first_out = {f"w{i}": a for i, _, a, _ in kept}
    last_out = {f"w{i}": b for i, _, _, b in kept}

    def nearest_kept(anchor, forward=True):
        n = word_index(anchor)
        idx = sorted(int(k[1:]) for k in first_out)
        cand = [i for i in idx if (i >= n if forward else i <= n)]
        if not cand:
            die(f"{anchor}: no kept word {'at or after' if forward else 'at or before'} it")
        return f"w{cand[0] if forward else cand[-1]}"

    def chapter_of(ms):
        k = 0
        for n, c in enumerate(chapters, 1):
            if (c.get("card") or [c["at_ms"]])[0] <= ms + 1:
                k = n
        return k

    raw = []
    for s in plan.get("slots", []):
        f = s["from"] if s["from"] in first_out else nearest_kept(s["from"], True)
        t = s["to"] if s["to"] in last_out else nearest_kept(s["to"], False)
        raw.append({**s, "from": f, "to": t, "a": first_out[f], "b": last_out[t]})
    raw.sort(key=lambda x: x["a"])
    for s in raw:
        s["chapter"] = chapter_of(s["a"])
    slots = []
    for i, s in enumerate(raw):
        ch = s["chapter"]
        prev = raw[i - 1] if i else None
        nxt = raw[i + 1] if i + 1 < len(raw) else None
        if prev and prev["chapter"] == ch:
            start = None  # set from the previous slot's end below
        elif ch == 0:
            start = 0.0
        else:
            c = chapters[ch - 1]
            start = (c.get("card") or [c["at_ms"], c["at_ms"]])[1]
        if nxt and nxt["chapter"] == ch:
            end = (s["b"] + nxt["a"]) / 2.0  # split the gap between them
        elif ch < len(chapters):
            c = chapters[ch]
            end = (c.get("card") or [c["at_ms"]])[0]
        else:
            end = tm.duration
        s["start"], s["end"] = start, end
        slots.append(s)
    for i, s in enumerate(slots):
        if s["start"] is None:
            s["start"] = slots[i - 1]["end"]
    for s in slots:
        f0, f1 = snap(s["start"]), snap(s["end"])
        s["slot0"] = f0 * 1000.0 / FPS
        s["end_snapped"] = f1 * 1000.0 / FPS
        s["frames"] = f1 - f0
    return slots, chapters


def cmd_resolve(a):
    w = os.path.abspath(a.work)
    pp = studio(w, "sections-plan.json")
    plan = jload(pp)
    if not plan:
        die(f"{pp} missing or not JSON: run the plan-sections workflow first")
    for k in ["chapters", "slots"]:
        if not isinstance(plan.get(k), list) or not plan[k]:
            die(f"sections-plan.json needs a non-empty `{k}` list")
    keys = [s.get("key") for s in plan["slots"]]
    if len(set(keys)) != len(keys) or not all(keys):
        die("every slot needs a unique `key`")
    for s in plan["slots"]:
        if s.get("mode") not in ("behind", "front", "none"):
            die(f"slot {s.get('key')}: mode must be behind, front or none")
    chap_path, groups_skipped = write_chapters(w, plan)
    tm = TimeMap(w)
    issues = [i for i in tm.plan.get("issues", []) if i.get("severity") == "error"]
    if issues:
        die("cutroom plan reports errors after writing chapters:\n" + "\n".join(str(i) for i in issues[:10]))
    slots, chapters = resolve_slots(w, plan, tm)
    kept = tm.words()
    old = {s["key"]: s for s in (jload(studio(w, "sections.json"), {}) or {}).get("slots", [])}
    out_slots = []
    for s in slots:
        key = s["key"]
        timing = slot_timing(tm, key, s["slot0"], s["end_snapped"], kept)
        tpath = studio(w, "timing", f"{key}.json")
        jsave(tpath, timing)
        row = {"key": key, "title": s.get("title") or key, "mode": s["mode"], "chapter": s["chapter"],
               "group": s.get("group"), "from": s["from"], "to": s["to"], "slot0": round(s["slot0"], 3),
               "end": round(s["end_snapped"], 3), "frames": s["frames"], "clock": f"{clock(s['slot0'])}-{clock(s['end_snapped'])}",
               "timing": tpath, "brief": studio(w, "briefs", f"{key}.md"), "job_dir": studio(w, "jobs", key),
               "depends_on": s.get("depends_on"), "note": s.get("note", ""),
               "words_sig": hashlib.sha256(json.dumps([(x["id"], x["f"]) for x in timing["words"]]).encode()).hexdigest()[:12]}
        row["sig"] = hashlib.sha256(f"{row['slot0']}|{row['frames']}|{row['words_sig']}|{row['mode']}".encode()).hexdigest()[:12]
        prev = old.get(key)
        row["moved"] = bool(prev and prev.get("sig") and prev.get("sig") != row["sig"])
        out_slots.append(row)
        if s["mode"] != "none":
            write_job(w, row, s)
    st = state(w)
    st["lock"] = {"map_hash": map_hash(tm.plan), "at": now(), "duration_ms": tm.duration}
    save_state(w, st)
    jsave(studio(w, "sections.json"), {"resolved_at": now(), "map_hash": st["lock"]["map_hash"],
                                       "duration_ms": tm.duration, "fps": FPS, "kicker": plan.get("kicker"),
                                       "chapters": [{"n": c.get("n"), "title": c.get("title"), "at_ms": c.get("at_ms"),
                                                     "card": c.get("card")} for c in chapters],
                                       "slots": out_slots})
    tv = plan.get("this_video")
    db = studio(w, "designer-brief.md")
    if tv and os.path.exists(db):
        t = open(db).read()
        marker = "(not written yet: the section planner fills this block)"
        if marker in t:
            t = t.replace(marker, tv.strip())
        else:
            t = re.sub(r"(## This video\n(?:<!--.*?-->\n)?)(.*)\Z", lambda m: m.group(1) + tv.strip() + "\n", t, flags=re.S)
        with open(db, "w") as f:
            f.write(t)
    cover = sum(s["frames"] for s in out_slots if s["mode"] != "none")
    total = snap(tm.duration)
    print(json.dumps({"chapters_file": chap_path, "chapters": len(chapters), "slots": len(out_slots),
                      "films": sum(1 for s in out_slots if s["mode"] == "behind"),
                      "overlays": sum(1 for s in out_slots if s["mode"] == "front"),
                      "moved": [s["key"] for s in out_slots if s["moved"]],
                      "frames_covered": cover, "frames_total": total,
                      "groups_skipped_old_engine": groups_skipped, "map_hash": st["lock"]["map_hash"]}, indent=1))


def write_job(w, row, s):
    d = row["job_dir"]
    os.makedirs(os.path.join(d, "checks"), exist_ok=True)
    p = os.path.join(d, "job.json")
    job = jload(p, {}) or {}
    behind = row["mode"] == "behind"
    job.update({
        "id": job.get("id") or f"studio-{row['key']}",
        "key": row["key"], "title": (row["title"] or row["key"])[:40], "mode": row["mode"], "style": "velour",
        "audio": False, "background": "solid" if behind else "transparent",
        "fps": int(FPS), "width": FRAME_W, "height": FRAME_H,
        "frames": row["frames"], "durationInFrames": row["frames"],
        "slot0": row["slot0"], "startMs": row["slot0"], "endMs": row["end"],
        "codec": "h264" if behind else "prores4444",
        "jobDir": d, "outDir": d, "brief": row["brief"], "timing": row["timing"],
        "designerBrief": studio(w, "designer-brief.md"), "proofDir": studio(w, "proof"), "work": w,
        "layout": {"panel": PANEL, "camera": CAMERA, "badge": BADGE},
        "canvas": {"bg": "#121212", "dot_pitch_px": 40, "drift_px_per_s": 2.5,
                   "phase": "output time; this job starts at slot0 ms"},
    })
    if s.get("depends_on"):
        job["dependsOn"] = s["depends_on"]
    jsave(p, job)


# ---------------------------------------------------------------------------------------------- verify / status

def cmd_verify(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json"))
    if not sec:
        die("no sections.json yet (resolve first)")
    tm = TimeMap(w)
    h = map_hash(tm.plan)
    if h == sec.get("map_hash"):
        print(json.dumps({"locked": True, "map_hash": h, "stale": []}))
        return
    kept = tm.words()
    stale = []
    for s in sec["slots"]:
        t = slot_timing(tm, s["key"], s["slot0"], s["end"], kept)
        wsig = hashlib.sha256(json.dumps([(x["id"], x["f"]) for x in t["words"]]).encode()).hexdigest()[:12]
        if wsig != s.get("words_sig"):
            stale.append(s["key"])
    print(json.dumps({"locked": False, "map_hash": h, "locked_hash": sec.get("map_hash"), "stale": stale,
                      "fix": "re-run `studio.py resolve` (slots move), then re-brief and re-design the stale keys"}, indent=1))
    sys.exit(3 if stale else 0)


def read_sig(p):
    try:
        return open(p).read().strip() or None
    except OSError:
        return None


def brief_sig_path(s):
    return s["brief"][:-3] + ".sig"


def job_sig_path(s):
    return os.path.join(s["job_dir"], "sig.txt")


def stamp(p, sig):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(sig + "\n")


def slot_status(w, s):
    d = s["job_dir"]
    rep = jload(os.path.join(d, "design-report.json"), {}) or {}
    ver = rep.get("version")
    chk = jload(os.path.join(d, "checks", f"check-v{ver}.json"), None) if ver is not None else None
    brief = s["brief"]
    critic = jload(brief[:-3] + ".critic.json", None)
    revised = os.path.exists(brief[:-3] + ".revised.json")
    brief_ok = os.path.exists(brief) and ((critic or {}).get("pass") is True or revised)
    render = rep.get("render_path")
    passed = bool(chk and (chk.get("pass") is True or not any(i.get("severity") == "blocking" for i in chk.get("issues", []))))
    return {"key": s["key"], "mode": s["mode"], "clock": s.get("clock"), "frames": s["frames"],
            "brief": "final" if brief_ok else ("draft" if os.path.exists(brief) else "missing"),
            "version": ver, "render": render if render and os.path.exists(render) else None,
            "check": None if chk is None else ("pass" if passed else "fail"),
            "open_blocking": [i.get("problem") for i in (chk or {}).get("issues", []) if i.get("severity") == "blocking"][:5],
            "stale_brief": bool(os.path.exists(brief) and read_sig(brief_sig_path(s)) not in (None, s.get("sig"))),
            "stale_render": bool(render and read_sig(job_sig_path(s)) not in (None, s.get("sig")))}


def cmd_status(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json"))
    if not sec:
        die("no sections.json yet (resolve first)")
    rows = [slot_status(w, s) for s in sec["slots"] if s["mode"] != "none"]
    if a.json:
        print(json.dumps(rows, indent=1))
        return
    for r in rows:
        flags = " ".join(x for x, on in (("STALE-BRIEF", r["stale_brief"]), ("STALE-RENDER", r["stale_render"])) if on)
        print(f"{r['key']:>6} {r['mode']:6} {r['clock']:>15} brief {r['brief']:7} v{r['version'] or '-':<3} "
              f"check {r['check'] or '-':4} {flags}")


# ---------------------------------------------------------------------------------------------- args for workflows

def references():
    root = os.path.join(ER, "studio", "kit", "references")
    if not os.path.isdir(root):
        return {"readme": None, "jobs": []}
    jobs = sorted(d for d in os.listdir(root) if os.path.isdir(os.path.join(root, d)))
    return {"readme": os.path.join(root, "README.md") if os.path.exists(os.path.join(root, "README.md")) else None,
            "jobs": [os.path.join(root, d) for d in jobs]}


def common_args(w):
    st = state(w)
    return {"er": ER, "work": w, "title": st.get("title"), "creator": st.get("creator", "Samin"),
            "today": st.get("today") or dt.date.today().isoformat(), "cutroom": cutroom_bin(),
            "studio_py": os.path.join(HERE, "studio.py"), "kit": os.path.join(ER, "studio", "kit"),
            "job_cmd": f"node {os.path.join(ER, 'studio', 'jobs', 'job.mjs')}",
            "designer_brief": studio(w, "designer-brief.md"), "proof_dir": studio(w, "proof"),
            "references": references(), "agents_dir": os.path.join(ER, "agents"),
            "helpers_merged": helpers_merged()}


def cmd_args(a):
    w = os.path.abspath(a.work)
    A = common_args(w)
    stage = a.stage
    if stage == "research":
        groups = jload(studio(w, "proof", "groups.json"))
        done = sorted(os.path.basename(os.path.dirname(p)) for p in glob.glob(studio(w, "proof", "*", "VERIFIED.json")))
        A.update({"groups": groups or None, "skip_folders": done,
                  "assembled": os.path.exists(studio(w, "proof", "proof-index.json")),
                  "narration_cmd": f'"{cutroom_bin()}" brief "{w}" --raw --width 60',
                  "kept_cmd": f'"{cutroom_bin()}" brief "{w}" --width 60'})
    elif stage == "plan":
        A.update({"plan_path": studio(w, "sections-plan.json"),
                  "proof_index": studio(w, "proof", "proof-index.json") if os.path.exists(studio(w, "proof", "proof-index.json")) else None,
                  "kept_cmd": f'"{cutroom_bin()}" brief "{w}" --width 60',
                  "review_dir": studio(w, "receipts", "plan-sheets"),
                  "resolve_cmd": f'python3 "{os.path.join(HERE, "studio.py")}" resolve "{w}"'})
    else:
        sec = jload(studio(w, "sections.json"))
        if not sec:
            die("no sections.json yet: run the plan-sections workflow, then `studio.py resolve`")
        live = [s for s in sec["slots"] if s["mode"] != "none"]
        stat = {r["key"]: r for r in (slot_status(w, s) for s in live)}
        if stage == "briefs":
            tasks, skip = [], []
            for i, s in enumerate(live):
                if stat[s["key"]]["brief"] == "final" and not stat[s["key"]]["stale_brief"]:
                    skip.append(s["key"])
                    continue
                nb = [x["key"] for x in live[max(0, i - 1):i + 2] if x["key"] != s["key"]]
                tasks.append({**slim(s), "neighbours": nb})
                if not a.no_stamp:
                    stamp(brief_sig_path(s), s["sig"])  # this brief is (re)written for the current slot geometry
            A.update({"template": os.path.join(ER, "studio", "templates", "section-brief.md"), "tasks": tasks,
                      "skipped_final": skip})
            if a.chain_design:
                A["design"] = {k: A[k] for k in ["er", "work", "title", "creator", "studio_py", "cutroom", "kit", "job_cmd",
                                                 "agents_dir", "designer_brief", "proof_dir", "references"]}
                A["design"].update({"max_rounds": a.max_rounds,
                                    "design_script": os.path.join(ER, "studio", "workflows", "design.js")})
                if not a.no_stamp:
                    for t in tasks:
                        stamp(job_sig_path(t), t["sig"])
        elif stage == "design":
            only = set(a.only.split(",")) if a.only else None
            tasks, skip = [], []
            for s in live:
                st = stat[s["key"]]
                if only and s["key"] not in only:
                    continue
                if st["check"] == "pass" and not st["stale_render"] and not only:
                    skip.append(s["key"])
                    continue
                if st["brief"] != "final" or st["stale_brief"]:
                    skip.append(s["key"] + " (no final brief for the current slot)")
                    continue
                t = slim(s)
                rep = jload(os.path.join(s["job_dir"], "design-report.json"))
                chk = jload(os.path.join(s["job_dir"], "checks", f"check-v{(rep or {}).get('version')}.json")) if rep else None
                if rep and rep.get("render_path") and os.path.exists(rep["render_path"]) and not st["stale_render"]:
                    t["resume"] = {"version": rep.get("version"), "render_path": rep.get("render_path"),
                                   "check": chk}
                tasks.append(t)
                if not a.no_stamp:
                    stamp(job_sig_path(s), s["sig"])  # renders from now on are for the current slot geometry
            A.update({"tasks": tasks, "skipped": skip, "max_rounds": a.max_rounds})
        elif stage == "continuity":
            renders = []
            for s in live:
                st = stat[s["key"]]
                if st["render"]:
                    renders.append({**slim(s), "render": st["render"], "version": st["version"], "pass": st["check"] == "pass"})
            A.update({"renders": renders, "out": studio(w, "receipts", "continuity.json"),
                      "design_script": os.path.join(ER, "studio", "workflows", "design.js"),
                      "cards": [{"n": c.get("n"), "title": c.get("title"), "card": c.get("card")} for c in sec.get("chapters", [])]})
        elif stage == "final-qa":
            vids = sorted(glob.glob(os.path.join(w, "out", "*.mp4")), key=os.path.getmtime)
            video = a.video or (vids[-1] if vids else None)
            if not video:
                die("no rendered video in out/: run the render stage first (or pass --video)")
            A.update({"video": video, "proof_index": studio(w, "proof", "proof-index.json"),
                      "chapters_file": video[:-4] + ".chapters.txt", "captions": video[:-4] + ".srt",
                      "cards": [{"n": c.get("n"), "title": c.get("title"), "card_s": [x / 1000.0 for x in (c.get("card") or [])]}
                                for c in sec.get("chapters", [])],
                      "slots": [{"key": s["key"], "mode": s["mode"], "clock": s.get("clock"),
                                 "slot0_s": s["slot0"] / 1000.0, "end_s": s["end"] / 1000.0} for s in live],
                      "scratch": studio(w, "receipts", "final-qa"), "out": studio(w, "receipts", "final-qa.json"),
                      "qa_cmd": f'python3 "{os.path.join(HERE, "qa.py")}" "{video}" --work "{w}"'})
        else:
            die(f"unknown stage {stage}")
    print(json.dumps(A, indent=1, ensure_ascii=False))


def slim(s):
    return {k: s[k] for k in ["key", "title", "mode", "chapter", "group", "from", "to", "slot0", "end", "frames",
                              "clock", "timing", "brief", "job_dir", "depends_on", "note", "sig"] if k in s}


# ---------------------------------------------------------------------------------------------- wire

def cmd_wire(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json"))
    if not sec:
        die("no sections.json yet")
    tm = TimeMap(w)
    if map_hash(tm.plan) != sec.get("map_hash"):
        die("the cuts or chapters changed since `resolve` (run `studio.py verify`); re-resolve and re-render the stale slots "
            "before wiring", 3)
    rich = helpers_merged()
    lines = ["# written by studio.py wire: one `video` beat per finished section film / demo overlay.",
             "# Frame-exact: starts at the slot's first output frame, lasts exactly its frames, no lead, no loop.",
             "beats:"]
    wired, missing, failing = [], [], []
    for s in sec["slots"]:
        if s["mode"] == "none":
            continue
        st = slot_status(w, s)
        if not st["render"]:
            missing.append(s["key"])
            continue
        if st["stale_render"]:
            failing.append(f"{s['key']} (rendered for an older slot: re-design it)")
            continue
        if st["check"] != "pass" and not a.allow_failing:
            failing.append(s["key"])
            continue
        frames = probe_frames(st["render"])
        if frames is not None and abs(frames - s["frames"]) > 1:
            failing.append(f"{s['key']} (render has {frames} frames, slot {s['frames']})")
            continue
        # start a hair (1 us) before the slot's first frame, as `cutroom slots` does, so time rounding can
        # never drop frame 0
        beat = {"at": f"o:{(s['slot0'] - 0.001) / 1000.0:.7f}", "hold": f"{s['frames'] * 1000.0 / FPS:.4f}ms", "lead": 0, "kind": "video",
                "src": os.path.relpath(st["render"], w), "fit": "frame", "loop": False}
        if s["mode"] == "front":
            beat["alpha"] = True
        if rich:
            beat["fade"] = False
        lines.append(f"  - {flow(beat)}  # {s['key']} {s.get('clock')} v{st['version']}")
        wired.append(s["key"])
    if not wired:
        lines[-1] = "beats: []"
    p = os.path.join(w, "beats", "studio-films.yaml")
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write("\n".join(lines).rstrip() + "\n")
    r = run([cutroom_bin(), "check", w])
    tail = (r.stdout + r.stderr).strip().splitlines()[-1:] if (r.stdout or r.stderr) else []
    print(json.dumps({"file": p, "wired": wired, "missing_render": missing, "not_passing": failing,
                      "check": tail[0] if tail else None, "check_exit": r.returncode,
                      "fade_field": rich}, indent=1))
    if missing or failing:
        sys.exit(2)


def probe_frames(path):
    r = run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-count_packets", "-show_entries",
             "stream=nb_read_packets", "-of", "csv=p=0", path])
    try:
        return int(r.stdout.strip().split(",")[0])
    except ValueError:
        return None


# ---------------------------------------------------------------------------------------------- timing

def cmd_timing(a):
    w = os.path.abspath(a.work)
    if a.slot:
        p = studio(w, "timing", f"{a.slot}.json")
        t = jload(p)
        if not t:
            die(f"no timing for slot {a.slot} (resolve first)")
    else:
        if not (a.frm and a.to):
            die("give --slot KEY or --from wA --to wB")
        tm = TimeMap(w)
        kept = tm.words()
        a0 = next((x[2] for x in kept if x[0] >= word_index(a.frm)), None)
        b0 = next((x[3] for x in reversed(kept) if x[0] <= word_index(a.to)), None)
        if a0 is None or b0 is None or b0 <= a0:
            die("that range has no kept words")
        s0 = snap(a0) * 1000.0 / FPS
        t = slot_timing(tm, f"{a.frm}-{a.to}", s0, (snap(b0) + 1) * 1000.0 / FPS, kept)
    if a.json:
        print(json.dumps(t, indent=1, ensure_ascii=False))
        return
    print(f"{t['key']}: slot0 {t['slot0']} ms ({clock(t['slot0'])}), {t['frames']} frames. {t['frame_rule']}")
    for s in t["sentences"]:
        print(f"  {s['ms'] / 1000:8.2f}s f{s['f']:<5} {s['from']:>6}  {s['text']}")


# ---------------------------------------------------------------------------------------------- boxes

def _norm(tok):
    return re.sub(r"[^\w$%/@#.+-]", "", tok.lower()).strip(".")


def _tesseract_words(png, invert=False):
    src = png
    tmp = None
    if invert:
        tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False).name
        r = run(["ffmpeg", "-v", "error", "-y", "-i", png, "-vf", "format=gray,negate", tmp])
        if r.returncode:
            return []
        src = tmp
    r = run(["tesseract", src, "stdout", "--psm", "11", "tsv"])
    if tmp:
        os.unlink(tmp)
    words = []
    for line in r.stdout.splitlines()[1:]:
        c = line.split("\t")
        if len(c) < 12 or not c[11].strip() or c[0] != "5":
            continue
        try:
            conf = float(c[10])
        except ValueError:
            conf = -1
        x, y, wd, ht = map(int, c[6:10])
        words.append({"t": c[11], "n": _norm(c[11]), "x0": x, "y0": y, "x1": x + wd, "y1": y + ht, "conf": conf,
                      "line": (int(c[2]), int(c[3]), int(c[4]))})
    return words  # tesseract's reading order (block, paragraph, line, word)


def _find(words, phrase, find_all):
    toks = [t for t in (_norm(x) for x in phrase.split()) if t]
    hits = []
    if not toks:
        return hits
    for i in range(len(words)):
        j, k = i, 0
        seq = []
        while j < len(words) and k < len(toks):
            if not words[j]["n"]:
                j += 1
                continue
            if words[j]["n"] != toks[k] and not (k == len(toks) - 1 and words[j]["n"].startswith(toks[k])):
                break
            seq.append(words[j])
            j += 1
            k += 1
        if k == len(toks):
            hits.append(seq)
            if not find_all:
                break
    return hits


def _segments(seq):
    """Union box per visual line (a wrapped phrase becomes one segment per line)."""
    segs = []
    for wd in seq:
        if segs and abs(wd["y0"] - segs[-1]["y0"]) < 0.6 * (segs[-1]["y1"] - segs[-1]["y0"]) and wd["x0"] >= segs[-1]["x0"]:
            s = segs[-1]
            s.update(x0=min(s["x0"], wd["x0"]), y0=min(s["y0"], wd["y0"]), x1=max(s["x1"], wd["x1"]), y1=max(s["y1"], wd["y1"]))
            s["text"] += " " + wd["t"]
        else:
            segs.append({"x0": wd["x0"], "y0": wd["y0"], "x1": wd["x1"], "y1": wd["y1"], "text": wd["t"]})
    return segs


def cmd_boxes(a):
    if has_cmd("boxes") and not a.fallback:
        cmd = [cutroom_bin(), "boxes", a.png, "--phrase", a.phrase] + (["--all"] if a.all else []) + (["--json"] if a.json else [])
        r = run(cmd)
        if r.returncode == 0:
            sys.stdout.write(r.stdout)
            return
    dims = run(["ffprobe", "-v", "error", "-show_entries", "stream=width,height", "-of", "csv=p=0", a.png]).stdout.strip()
    try:
        W, H = (int(v) for v in dims.split(",")[:2])
    except ValueError:
        die(f"can't read {a.png}")
    hits = []
    for inv in (False, True):
        hits = _find(_tesseract_words(a.png, inv), a.phrase, a.all)
        if hits:
            break
    out = {"png": a.png, "size": [W, H], "phrase": a.phrase, "found": bool(hits), "by": "tesseract (studio.py fallback)",
           "matches": []}
    for seq in hits:
        segs = _segments(seq)
        note = None
        lead, tail = seq[0]["t"], seq[-1]["t"]
        if re.match(r"^[\"'“‘(\[]", lead) or re.search(r"[\"'”’)\].,:;!?]$", tail):
            note = (f"OCR word boxes include punctuation glued to the phrase ({lead!r} ... {tail!r}): "
                    "trim the box edge by about one character width there if the mark should stay unmarked")
        out["matches"].append({"note": note, "segments": [{"text": s["text"], "px": [s["x0"], s["y0"], s["x1"], s["y1"]],
                                             "norm": [round(s["x0"] / W, 4), round(s["y0"] / H, 4),
                                                      round((s["x1"] - s["x0"]) / W, 4), round((s["y1"] - s["y0"]) / H, 4)]}
                                            for s in segs]})
    if a.json:
        print(json.dumps(out, indent=1))
    else:
        if not hits:
            print(f"not found: {a.phrase!r} in {a.png} (OCR missed it: measure on the image, or try a shorter phrase)")
        for m in out["matches"]:
            for s in m["segments"]:
                print(f"{s['px']}  {s['text']}")
            if m.get("note"):
                print(f"  note: {m['note']}")
    if not hits:
        sys.exit(4)


# ---------------------------------------------------------------------------------------------- sheet-video

def duration_s(path):
    r = run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path])
    try:
        return float(r.stdout.strip())
    except ValueError:
        return 0.0


def has_alpha(path):
    r = run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=pix_fmt", "-of", "csv=p=0", path])
    return r.stdout.strip().startswith(("yuva", "rgba", "argb", "bgra", "abgr", "gbrap", "ya"))


def grab(video, t, out, width=480, alpha=None):
    alpha = has_alpha(video) if alpha is None else alpha
    if alpha:
        fc = (f"color=c=0x121212:s={FRAME_W}x{FRAME_H}:d=1[bg];[0:v]format=rgba[fg];[bg][fg]overlay=0:0:shortest=1,"
              f"scale={width}:-2")
        cmd = ["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", video, "-frames:v", "1", "-filter_complex", fc, out]
    else:
        cmd = ["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", video, "-frames:v", "1", "-vf", f"scale={width}:-2", out]
    return run(cmd).returncode == 0


def cmd_sheet_video(a):
    if has_cmd("sheet-video") and not a.fallback:
        cmd = [cutroom_bin(), "sheet-video", a.video] + (["--every", str(a.every)] if a.every else []) + \
              (["--at", a.at] if a.at else []) + (["-o", a.out] if a.out else [])
        r = run(cmd)
        if r.returncode == 0:
            sys.stdout.write(r.stdout)
            return
    dur = duration_s(a.video)
    if a.at:
        times = [float(x) for x in a.at.split(",") if x.strip()]
    else:
        step = a.every or max(1.0, dur / 12)
        times = [min(dur - 0.05, step * i + 0.05) for i in range(int(dur // step) + 1)]
    times = [t for t in times if 0 <= t < dur] or [0.0]
    out = a.out or os.path.splitext(a.video)[0] + ".sheet.png"
    tmp = tempfile.mkdtemp(prefix="sheet-")
    alpha = has_alpha(a.video)
    cols = a.cols
    rows = math.ceil(len(times) / cols)
    n = 0
    for i, t in enumerate(times):
        if grab(a.video, t, os.path.join(tmp, f"f{i:04d}.png"), alpha=alpha):
            n += 1
    for i in range(len(times), rows * cols):
        run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i", "color=c=black:s=480x270:d=1", "-frames:v", "1",
             os.path.join(tmp, f"f{i:04d}.png")])
    r = run(["ffmpeg", "-v", "error", "-y", "-i", os.path.join(tmp, "f%04d.png"), "-vf",
             f"scale=480:270:force_original_aspect_ratio=decrease,pad=480:270:(ow-iw)/2:(oh-ih)/2,tile={cols}x{rows}:padding=4:color=0x333333",
             "-frames:v", "1", out])
    shutil.rmtree(tmp, ignore_errors=True)
    if r.returncode:
        die(f"tile failed: {r.stderr[-500:]}")
    md = os.path.splitext(out)[0] + ".md"
    with open(md, "w") as f:
        f.write(f"# {os.path.basename(out)}: {os.path.basename(a.video)}, left to right, top to bottom ({cols} per row)\n\n")
        for i, t in enumerate(times, 1):
            f.write(f"{i}. {t:.2f}s (f{round(t * FPS)})\n")
    print(json.dumps({"sheet": out, "frames": len(times), "list": md, "alpha_over_canvas": alpha}))


# ---------------------------------------------------------------------------------------------- check-render

def _rect_px(r):
    return [int(r[0] * FRAME_W), int(r[1] * FRAME_H), int(r[2] * FRAME_W), int(r[3] * FRAME_H)]


def _stat_series(video, vf, key):
    r = run(["ffmpeg", "-v", "info", "-i", video, "-an", "-vf", vf + f",metadata=print:key={key}", "-f", "null", "-"])
    vals = []
    for m in re.finditer(rf"pts_time:([\d.]+).*?\n.*?{re.escape(key)}=([\d.]+)", r.stderr, re.S):
        vals.append((float(m.group(1)), float(m.group(2))))
    return vals


def cmd_check_render(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json")) or {}
    s = next((x for x in sec.get("slots", []) if x["key"] == a.key), None)
    if not s:
        die(f"no slot {a.key} in sections.json")
    rep = jload(os.path.join(s["job_dir"], "design-report.json"), {}) or {}
    render = a.render or rep.get("render_path")
    if not render or not os.path.exists(render):
        die(f"no render for {a.key} (pass --render)")
    if has_cmd("check-render") and not a.fallback:
        r = run([cutroom_bin(), "check-render", render, "--slot", a.key, "--work", w] + (["--json"] if a.json else []))
        if r.returncode in (0, 1):
            sys.stdout.write(r.stdout)
            sys.exit(r.returncode)
    findings = []
    frames = probe_frames(render)
    if frames != s["frames"]:
        findings.append({"severity": "blocking", "what": f"frame count {frames}, slot needs {s['frames']}",
                         "fix": "keep job.json frames; re-render"})
    alpha = has_alpha(render)
    dur = frames / FPS if frames else duration_s(render)
    if not alpha:
        r = run(["ffmpeg", "-v", "info", "-i", render, "-an", "-vf", "blackdetect=d=0.2:pix_th=0.04", "-f", "null", "-"])
        for m in re.finditer(r"black_start:([\d.]+) black_end:([\d.]+)", r.stderr):
            t0, t1 = float(m.group(1)), float(m.group(2))
            findings.append({"severity": "blocking", "what": f"dip to black {t0:.2f}-{t1:.2f}s (f{round(t0 * FPS)})",
                             "fix": "crossfade over the outgoing layer, never through black"})
        r = run(["ffmpeg", "-v", "info", "-i", render, "-an", "-vf", "scale=480:-2,freezedetect=n=0.0015:d=6", "-f", "null", "-"])
        st = [float(x) for x in re.findall(r"freeze_start: ([\d.]+)", r.stderr)]
        en = [float(x) for x in re.findall(r"freeze_end: ([\d.]+)", r.stderr)]
        for i, t0 in enumerate(st):
            t1 = en[i] if i < len(en) else dur
            findings.append({"severity": "minor", "what": f"static for {t1 - t0:.1f}s from {t0:.2f}s (f{round(t0 * FPS)})",
                             "fix": "add a slow push or the next beat (nothing static over ~4 s)"})
        # the camera tile and badge must stay clear (only canvas there)
        for name, rect in (("camera tile", CAMERA), ("badge", BADGE)):
            x, y, wd, ht = _rect_px(rect)
            vals = _stat_series(render, f"fps=2,crop={wd}:{ht}:{x}:{y},signalstats", "lavfi.signalstats.YMAX")
            bad = [t for t, v in vals if v > 120]
            if bad:
                findings.append({"severity": "blocking",
                                 "what": f"content inside the {name} area at {len(bad)} sampled frames (first {bad[0]:.1f}s)",
                                 "fix": "keep everything in the panel (final x < 1500): cutroom draws the camera and badge there"})
    else:
        x0 = int(0.7766 * FRAME_W)
        vals = _stat_series(render, f"fps=2,alphaextract,format=gray,crop={FRAME_W - x0}:{FRAME_H}:{x0}:0,signalstats", "lavfi.signalstats.YMAX")
        bad = [t for t, v in vals if v > 8]
        if bad:
            findings.append({"severity": "blocking", "what": f"overlay not transparent at x >= {x0} ({len(bad)} sampled frames)",
                             "fix": "nothing outside the panel in a FRONT overlay"})
        cov = _stat_series(render, "fps=1,alphaextract,format=gray,signalstats", "lavfi.signalstats.YAVG")
        if cov:
            mean = sum(v for _, v in cov) / len(cov) / 255.0
            peak = max(v for _, v in cov) / 255.0
            if peak > 0.45:
                findings.append({"severity": "minor", "what": f"overlay covers up to {peak:.0%} of the frame (mean {mean:.0%})",
                                 "fix": "small cards in empty areas; the footage is the main visual"})
    timing = jload(s["timing"], {}) or {}
    wf = {x["id"]: x["f"] for x in timing.get("words", [])}
    for b in rep.get("beats", []) or []:
        wid, fr = b.get("word_id"), b.get("frame")
        if wid in wf and isinstance(fr, (int, float)):
            lead = wf[wid] - fr
            if lead > 6 or lead < -3:
                findings.append({"severity": "minor", "what": f"beat at f{fr} keyed to {wid} lands {lead} frames "
                                 f"{'early' if lead > 0 else 'late'} (target 2-4 early)", "fix": f"move it to f{wf[wid] - 3}"})
    res = {"key": a.key, "render": render, "frames": frames, "slot_frames": s["frames"], "alpha": alpha,
           "findings": findings, "blocking": sum(1 for f in findings if f["severity"] == "blocking"), "by": "studio.py fallback"}
    ver = rep.get("version")
    if ver is not None:
        jsave(os.path.join(s["job_dir"], "checks", f"mech-v{ver}.json"), res)
    if a.json:
        print(json.dumps(res, indent=1))
    else:
        print(f"{a.key}: {frames} frames (slot {s['frames']}), {'alpha' if alpha else 'opaque'}, {res['blocking']} blocking")
        for f in findings:
            print(f"  [{f['severity']}] {f['what']} -> {f['fix']}")
    sys.exit(1 if res["blocking"] else 0)


# ------------------------------------------------------------------------------ decision models (optional)
# Jev / Sage pre-screens (tools/jev/studio_jev.py). Never a replacement for an agent's judgment: they flag,
# rank and suggest; every unsure answer goes to the agent. Without a key they print {"skipped": ...}.

def _jev():
    sys.path.insert(0, os.path.join(ER, "tools", "jev"))
    import studio_jev  # noqa: E402
    return studio_jev


def cmd_jev_privacy(a):
    w = os.path.abspath(a.work)
    video = a.video
    if not video:
        outs = sorted(glob.glob(os.path.join(w, "out", "*.mp4")), key=os.path.getmtime)
        video = outs[-1] if outs else None
    if not video:
        die("no render to sweep: pass --video (the latest render of the edit)")
    print(json.dumps(_jev().privacy(w, os.path.abspath(video), a.every), indent=1))


def cmd_jev_frames(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json")) or {}
    s = next((x for x in sec.get("slots", []) if x["key"] == a.key), None)
    if not s:
        die(f"no slot {a.key} in sections.json")
    rep = jload(os.path.join(s["job_dir"], "design-report.json"), {}) or {}
    render = a.render or rep.get("render_path")
    if not render or not os.path.exists(render):
        die(f"no render for {a.key} (pass --render)")
    res = _jev().frames(w, a.key, render, rep.get("version"), a.every, s["job_dir"])
    print(json.dumps({k: v for k, v in res.items() if k != "frames"}, indent=1))


def cmd_polish(a):
    print(json.dumps({k: v for k, v in _jev().polish(os.path.abspath(a.video), a.every).items() if k != "points"}, indent=1))


def cmd_rank_crops(a):
    print(json.dumps(_jev().rank_crops(a.claim, [os.path.abspath(p) for p in a.pngs]), indent=1))


# ---------------------------------------------------------------------------------------------- log / report

def cmd_log(a):
    w = os.path.abspath(a.work)
    p = studio(w, "timings.json")
    rows = jload(p, []) or []
    rows.append({"stage": a.stage, "event": a.event, "at": now(), "note": a.note})
    jsave(p, rows)
    print(f"{a.stage} {a.event}")


def cmd_report(a):
    w = os.path.abspath(a.work)
    sec = jload(studio(w, "sections.json")) or {}
    lines = ["## Studio", ""]
    rows = [slot_status(w, s) for s in sec.get("slots", []) if s["mode"] != "none"]
    if rows:
        lines += ["| slot | mode | time | brief | version | check |", "|---|---|---|---|---|---|"]
        for r in rows:
            lines.append(f"| {r['key']} | {r['mode']} | {r['clock']} | {r['brief']} | {r['version'] or '-'} | {r['check'] or '-'} |")
        lines.append("")
    t = jload(studio(w, "timings.json"), []) or []
    starts = {}
    spans = []
    for e in t:
        if e["event"] == "start":
            starts[e["stage"]] = e["at"]
        elif e["event"] == "end" and e["stage"] in starts:
            d = dt.datetime.fromisoformat(e["at"]) - dt.datetime.fromisoformat(starts.pop(e["stage"]))
            spans.append((e["stage"], d.total_seconds()))
    if spans:
        lines += ["Stage times:", ""] + [f"- {k}: {v / 60:.1f} min" for k, v in spans] + [""]
    cont = jload(studio(w, "receipts", "continuity.json"))
    if cont:
        lines += [f"Continuity: {len(cont.get('issues', []))} issues ({sum(1 for i in cont.get('issues', []) if i.get('severity') == 'blocking')} blocking)", ""]
    flags = []
    for p in sorted(glob.glob(studio(w, "receipts", "final-qa-*.json"))):
        name = os.path.basename(p)[len("final-qa-"):-5]
        for i in (jload(p, {}) or {}).get("issues", []):
            flags.append((name, i))
    if flags:
        lines += [f"Final QA: {len(flags)} flags", ""] + [
            f"- {i.get('t_s', '')}s [{i.get('severity')}] ({name}) {i.get('problem')} -> {i.get('fix', '')}" for name, i in flags[:30]] + [""]
    print("\n".join(lines))


# ---------------------------------------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("init")
    p.add_argument("work"); p.add_argument("--title"); p.add_argument("--creator"); p.add_argument("--direction")
    p.add_argument("--today"); p.add_argument("--keep-draft-beats", action="store_true")
    p = sub.add_parser("resolve"); p.add_argument("work")
    p = sub.add_parser("verify"); p.add_argument("work")
    p = sub.add_parser("status"); p.add_argument("work"); p.add_argument("--json", action="store_true")
    p = sub.add_parser("args")
    p.add_argument("stage", choices=["research", "plan", "briefs", "design", "continuity", "final-qa"])
    p.add_argument("work"); p.add_argument("--only", help="design: comma-separated keys to (re)design")
    p.add_argument("--max-rounds", type=int, default=3); p.add_argument("--video")
    p.add_argument("--no-stamp", action="store_true", help="don't record the slot signature on issued tasks (dry runs)")
    p.add_argument("--chain-design", action="store_true", help="briefs: continue each final brief straight into design")
    p = sub.add_parser("wire"); p.add_argument("work"); p.add_argument("--allow-failing", action="store_true")
    p = sub.add_parser("timing"); p.add_argument("work"); p.add_argument("--slot")
    p.add_argument("--from", dest="frm"); p.add_argument("--to"); p.add_argument("--json", action="store_true")
    p = sub.add_parser("boxes"); p.add_argument("png"); p.add_argument("--phrase", required=True)
    p.add_argument("--all", action="store_true"); p.add_argument("--json", action="store_true")
    p.add_argument("--fallback", action="store_true", help="use tesseract here even if cutroom has `boxes`")
    p = sub.add_parser("sheet-video"); p.add_argument("video"); p.add_argument("--every", type=float)
    p.add_argument("--at"); p.add_argument("--out", "-o"); p.add_argument("--cols", type=int, default=4)
    p.add_argument("--fallback", action="store_true")
    p = sub.add_parser("check-render"); p.add_argument("work"); p.add_argument("key"); p.add_argument("--render")
    p.add_argument("--json", action="store_true"); p.add_argument("--fallback", action="store_true")
    p = sub.add_parser("log"); p.add_argument("work"); p.add_argument("stage"); p.add_argument("event", choices=["start", "end"])
    p.add_argument("--note")
    p = sub.add_parser("report"); p.add_argument("work")
    p = sub.add_parser("jev-privacy", help="decision-model privacy sweep of a render -> candidates for a reviewer")
    p.add_argument("work"); p.add_argument("--video"); p.add_argument("--every", type=float, default=1.0)
    p = sub.add_parser("jev-frames", help="decision-model pre-screen of a section film -> frames the checker looks at first")
    p.add_argument("work"); p.add_argument("key"); p.add_argument("--render"); p.add_argument("--every", type=float, default=1.0)
    p = sub.add_parser("polish", help="polish score per sampled frame of a video -> weakest frames for reviewers")
    p.add_argument("video"); p.add_argument("--every", type=float, default=4.0)
    p = sub.add_parser("rank-crops", help="rank proof crops for one claim (a starting order for the brief writer)")
    p.add_argument("--claim", required=True); p.add_argument("pngs", nargs="+")
    a = ap.parse_args()
    {"init": cmd_init, "resolve": cmd_resolve, "verify": cmd_verify, "status": cmd_status, "args": cmd_args,
     "wire": cmd_wire, "timing": cmd_timing, "boxes": cmd_boxes, "sheet-video": cmd_sheet_video,
     "check-render": cmd_check_render, "log": cmd_log, "report": cmd_report, "jev-privacy": cmd_jev_privacy,
     "jev-frames": cmd_jev_frames, "polish": cmd_polish, "rank-crops": cmd_rank_crops}[a.cmd](a)


if __name__ == "__main__":
    main()
