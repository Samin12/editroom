"""Decision-model helpers for editroom studio (TypeSafe Jev first, Levanto Sage as fallback).

THE RULE (Samin, Nov 1): never compromise how the edits look and feel. These helpers only make the
existing judges faster: they pre-screen, rank and suggest. They never remove a check, never pass
anything on their own, and route every unsure answer to the agent that owns the decision.

    privacy(work, video)     1 fps sweep of a render for private details the OCR regex misses
                             -> studio/receipts/privacy-candidates.json + a contact sheet + suggested
                                blur rects (NOT in beats/: a reviewer confirms them by eye)
    frames(work, key)        pre-screen of one section film: frames flagged as clipped, overlapping,
                             empty or too small, and the least polished ones -> checks/jev-frames-v<N>.json
                             (the checker looks at these FIRST, then still does its full pass)
    polish(video)            polish score per sampled frame (separated Samin's good tips edit from
                             the rejected draft at AUC 0.85): the weakest frames, for reviewers
    rank_crops(claim, pngs)  rank proof crops for one claim (agreed with the crop the good edit used
                             71% of the time vs 34% chance): a starting order for the brief writer

Measured on real data (tools/jev/BENCHMARKS.md). Everything degrades to {"skipped": ...} without a key.
"""

import concurrent.futures as cf
import json
import os
import subprocess
import tempfile

import decide

# Pre-screen questions and the provider each is asked of: the one with the better precision at full
# recall on planted defects (BENCHMARKS.md). Precision matters here because the checker's full pass stays:
# the list only decides what it looks at first. clipped: Jev .59 vs Sage .45; overlap: Sage .76 vs Jev .48;
# empty: both 1.0 (Jev is faster); tiny: Jev .50 vs Sage .25 (Sage flags most body text).
QA = {
    "clipped": ("jev", "Is any text, card or picture cut off at the left or right edge of the image, so part of it is missing?"),
    "overlap": ("sage", "Do two texts or elements overlap or ghost over each other so that some text becomes hard to read?"),
    "empty": ("jev", "Is the image essentially empty: only a dark dotted background, with no text, card, picture or diagram?"),
    "tiny": ("jev", "Is the main text in this image too small to read comfortably on a phone screen?"),
}
POLISH = {"id": "polish", "kind": "score",
          "instructions": "Rate how polished and well designed this frame of a YouTube explainer looks: real proof or a "
                          "designed graphic that clearly supports the point, clean layout, readable text, nothing empty or cheap-looking.",
          "levels": ["poor: empty, cheap, or cluttered", "weak: plain template or little to look at", "ok: decent but generic",
                     "good: clean, designed, supports the point", "excellent: premium, designed, clear proof or diagram"]}
PRIVATE = ("Is any private information readable in this screenshot of a video frame: an email address (even partly cut off), "
           "an API key or token, an account, workspace or card ID inside a URL, a bank or financial account name, a phone number, "
           "or a private message from another person? Ignore the presenter's face and the 'TIP n OF 10' badge.")
FPS = 30


def skipped():
    return {"skipped": "no decision-model key (TYPESAFE_API_KEY in ~/.editroom/.env or SAGE_API_KEY in ~/.sagecut/.env)"}


def pick(pref):
    have = decide.available()
    if pref in have:
        return pref
    return have[0] if have else None


def grab(video, times, out_dir, width=1568):
    """JPEG frames of `video` at `times` (s), in parallel, cached by name."""
    os.makedirs(out_dir, exist_ok=True)

    def one(t):
        p = os.path.join(out_dir, f"t{t:09.3f}.jpg")
        if not os.path.exists(p):
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", video, "-frames:v", "1",
                            "-vf", f"scale={width}:-2", "-q:v", "3", p], capture_output=True)
        return p if os.path.exists(p) else None

    with cf.ThreadPoolExecutor(6) as ex:
        return list(ex.map(one, times))


def grab_every(video, every, out_dir, width=1568):
    """One decode pass: a frame every `every` s, at its middle (t = i*every + every/2). Returns (times, paths)."""
    os.makedirs(out_dir, exist_ok=True)
    pat = os.path.join(out_dir, f"e{every:g}-%05d.jpg")
    if not os.path.exists(pat % 1):
        subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{every / 2:.3f}", "-i", video, "-vf",
                        f"fps=1/{every:g},scale={width}:-2", "-q:v", "3", pat], capture_output=True)
    paths, i = [], 1
    while os.path.exists(pat % i):
        paths.append(pat % i)
        i += 1
    return [round((k * every) + every / 2, 3) for k in range(len(paths))], paths


def duration(video):
    r = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", video],
                       capture_output=True, text=True)
    try:
        return float(r.stdout.strip())
    except ValueError:
        return 0.0


def has_alpha(video):
    r = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=pix_fmt", "-of", "csv=p=0", video],
                       capture_output=True, text=True)
    pf = r.stdout.strip()
    return any(x in pf for x in ("yuva", "rgba", "argb", "bgra", "abgr", "gbrap", "ya8", "ya16"))


def ask_many(jobs, provider, parallel=12):
    return list(decide.run_jobs(jobs, [provider], parallel))


# ------------------------------------------------------------------------------------------- privacy

def privacy(work, video, every=1.0, segments=None):
    """Sweep `video` (a render of the edit) once per `every` s. Returns the candidate stretches."""
    prov = pick("jev")
    if not prov:
        return skipped()
    rdir = os.path.join(work, "studio", "receipts", "privacy-sweep")
    times, frames = grab_every(video, every, os.path.join(rdir, "frames"))
    qs = [{"id": "private", "kind": "yesno", "instructions": PRIVATE,
           "criteria": {"true": "Some private detail is readable on screen.", "false": "Nothing private is readable."}}]
    if prov == "jev":
        qs.append({"id": "where", "kind": "box", "instructions": "the private detail (an email address, an ID in a URL, or a bank name)"})
    jobs = [{"key": f"{t:.3f}", "image": f, "questions": qs} for t, f in zip(times, frames) if f]
    rows = ask_many(jobs, prov)
    by = {}
    for r in rows:
        by.setdefault(r["key"], {})[r["id"]] = r
    hits = []
    for t in times:
        a = by.get(f"{t:.3f}", {})
        p = a.get("private", {})
        if p.get("error") or p.get("answer") != "no":  # yes, unsure or failed: a person looks
            w = a.get("where") or {}
            hits.append({"t": t, "p": p.get("p"), "answer": p.get("answer") or ("error" if p.get("error") else None),
                         "box": w.get("box"), "box_confidence": w.get("confidence")})
    # merge into stretches (gaps up to 1.5 sampling steps)
    stretches = []
    for h in hits:
        if stretches and h["t"] - stretches[-1]["to"] <= every * 1.5:
            s = stretches[-1]
            s["to"] = h["t"]
            s["frames"].append(h)
        else:
            stretches.append({"from": h["t"], "to": h["t"], "frames": [h]})
    segs = segments if segments is not None else plan_segments(work)
    for s in stretches:
        best = max(s["frames"], key=lambda h: (h["box_confidence"] or 0))
        s["box"] = best["box"]
        s["at"] = best["t"]
        s["max_p"] = max((h["p"] or 0) for h in s["frames"])
        s["suggested_blur"] = suggest_blur(best["box"], best["t"], segs, s["from"] - every / 2, s["to"] + every / 2)
        s.pop("frames")
    out = {"video": video, "every_s": every, "provider": prov, "frames_checked": len(jobs), "flagged_frames": len(hits),
           "stretches": stretches,
           "rule": "candidates only: look at each stretch (sheet below); copy the blurs you confirm into beats/review-privacy.yaml"}
    os.makedirs(rdir, exist_ok=True)
    with open(os.path.join(work, "studio", "receipts", "privacy-candidates.json"), "w") as f:
        json.dump(out, f, indent=1)
    if stretches:
        out["sheet"] = sheet_with_boxes([(s["at"], s["box"]) for s in stretches], video,
                                        os.path.join(rdir, "frames"), os.path.join(rdir, "candidates.jpg"))
    return out


def plan_segments(work):
    c = os.path.expanduser("~/.editroom/bin/cutroom")
    r = subprocess.run([c, "plan", work, "--segments"], capture_output=True, text=True)
    try:
        return json.loads(r.stdout[r.stdout.index("{"):]).get("segments") or []
    except ValueError:
        return []


def suggest_blur(box, t, segs, t0, t1, fw=1920, fh=1080):
    """The frame box as a cutroom blur beat (screen-normalized rect) when the screen recording is what
    shows there; else a note that the detail is inside a film or a card (fix it in that slot)."""
    if not box:
        return None
    seg = next((g for g in segs if g["start_ms"] <= t * 1000 < g["end_ms"]), None)
    if not seg or not seg.get("screen_rect") or seg.get("frame_video") or seg.get("stage"):
        return {"note": "not the screen recording here (a film, card or camera): fix it in that slot"}
    sx, sy, sw, sh = seg["screen_rect"]
    x, y, w, h = box[0] * fw, box[1] * fh, box[2] * fw, box[3] * fh
    pad = 6
    rect = [max(0.0, (x - pad - sx) / sw), max(0.0, (y - pad - sy) / sh), (w + 2 * pad) / sw, (h + 2 * pad) / sh]
    rect = [round(v, 4) for v in rect]
    return {"beat": {"at": f"o:{t0:.2f}", "until": f"o:{t1:.2f}", "kind": "blur", "rects": [rect], "lead": 0},
            "note": "output-time anchors: re-anchor to s: (source) after confirming, so recuts can't shift it"}


def sheet_with_boxes(items, video, frame_dir, out_path):
    try:
        from PIL import Image, ImageDraw  # optional
    except ImportError:
        return None
    tiles = []
    for t, box in items[:24]:
        p = grab(video, [t], frame_dir)[0]
        if not p:
            continue
        im = Image.open(p).convert("RGB")
        if box:
            W, H = im.size
            ImageDraw.Draw(im).rectangle([box[0] * W, box[1] * H, (box[0] + box[2]) * W, (box[1] + box[3]) * H],
                                         outline=(0, 255, 0), width=5)
        tiles.append(im.resize((784, 441)))
    if not tiles:
        return None
    cols = 3
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new("RGB", (784 * cols, 441 * rows))
    for i, im in enumerate(tiles):
        sheet.paste(im, ((i % cols) * 784, (i // cols) * 441))
    sheet.save(out_path, quality=82)
    return out_path


# ------------------------------------------------------------------------------- film pre-screen

def frames(work, key, render, version=None, every=1.0, job_dir=None):
    """Pre-screen a BEHIND section film (opaque). FRONT overlays are skipped: judge them composited."""
    if not decide.available():
        return skipped()
    if has_alpha(render):
        return {"skipped": "transparent overlay: pre-screen only works on opaque films (the checker composites FRONT overlays)"}
    dur = duration(render)
    times = [round(i * every + every / 2, 3) for i in range(max(1, int(dur / every)))]
    fdir = os.path.join(job_dir or tempfile.mkdtemp(), "checks", "jev-frames")
    paths = grab(render, times, fdir)
    jobs_by = {}
    for q, (pref, text) in QA.items():
        prov = pick(pref)
        jobs_by.setdefault(prov, {})
        for t, p in zip(times, paths):
            if p:
                jobs_by[prov].setdefault(t, {"key": f"{t:.3f}", "image": p, "questions": []})["questions"].append(
                    {"id": q, "kind": "yesno", "instructions": text})
    jprov = pick("jev")
    for t, p in zip(times, paths):
        if p:
            jobs_by[jprov].setdefault(t, {"key": f"{t:.3f}", "image": p, "questions": []})["questions"].append(POLISH)
    ans = {}
    for prov, jobs in jobs_by.items():
        for r in ask_many(list(jobs.values()), prov):
            ans.setdefault(r["key"], {})[r["id"]] = r
    out = []
    for t in times:
        a = ans.get(f"{t:.3f}", {})
        flags = [q for q in QA if (a.get(q) or {}).get("answer") == "yes"]
        unsure = [q for q in QA if (a.get(q) or {}).get("answer") not in ("yes", "no")]  # null or failed
        pol = (a.get("polish") or {}).get("answer")
        out.append({"t": t, "f": int(round(t * FPS)), "flags": flags, "unsure": unsure, "polish": pol})
    flagged = [x for x in out if x["flags"]]
    unsure_frames = [x for x in out if x["unsure"] and not x["flags"]]
    weakest = sorted([x for x in out if x["polish"] is not None], key=lambda x: x["polish"])[:max(3, len(out) // 10)]
    res = {"key": key, "version": version, "render": render, "every_s": every, "frames": out,
           "flagged": [{"f": x["f"], "t": x["t"], "flags": x["flags"]} for x in flagged],
           "unsure": [{"f": x["f"], "t": x["t"], "questions": x["unsure"]} for x in unsure_frames],
           "weakest": [{"f": x["f"], "t": x["t"], "polish": round(x["polish"], 2)} for x in weakest],
           "rule": "pre-screen only: look at these frames FIRST, then still do the full 13-point pass on every beat"}
    if job_dir and version is not None:
        os.makedirs(os.path.join(job_dir, "checks"), exist_ok=True)
        with open(os.path.join(job_dir, "checks", f"jev-frames-v{version}.json"), "w") as f:
            json.dump(res, f, indent=1)
    return res


# --------------------------------------------------------------------------------------- polish

def polish(video, every=4.0, out_dir=None):
    prov = pick("jev")
    if not prov:
        return skipped()
    dur = duration(video)
    times = [round(i * every + every / 2, 3) for i in range(max(1, int(dur / every)))]
    paths = grab(video, times, out_dir or tempfile.mkdtemp())
    rows = ask_many([{"key": f"{t:.3f}", "image": p, "questions": [POLISH]} for t, p in zip(times, paths) if p], prov)
    sc = {r["key"]: r["answer"] for r in rows if r.get("answer") is not None}
    pts = [{"t": t, "polish": round(sc[f"{t:.3f}"], 2)} for t in times if f"{t:.3f}" in sc]
    weakest = sorted(pts, key=lambda x: x["polish"])[:12]
    mean = round(sum(p["polish"] for p in pts) / len(pts), 2) if pts else None
    return {"video": video, "provider": prov, "every_s": every, "mean": mean, "weakest": weakest, "points": pts,
            "rule": "a ranking for reviewers (start with the weakest), never a pass/fail gate"}


# ----------------------------------------------------------------------------------- rank crops

def rank_crops(claim, pngs):
    prov = pick("jev")
    if not prov:
        return skipped()
    q = {"id": "fit", "kind": "score",
         "instructions": "How well does the screenshot in the image work as on-screen proof of what the creator says: "
                         "the exact words or number readable at a glance, without clutter?",
         "levels": ["unusable", "weak", "ok", "good", "ideal"]}
    rows = ask_many([{"key": p, "image": p, "text": f"What the creator says: \"{claim}\"", "questions": [q]} for p in pngs], prov)
    ranked = sorted(rows, key=lambda r: -(r.get("answer") if r.get("answer") is not None else -1))
    return {"claim": claim, "provider": prov,
            "ranked": [{"png": r["key"], "score": r.get("answer"), "confidence": r.get("confidence")} for r in ranked],
            "rule": "a starting order: the brief writer still picks the crop that shows his exact words"}
