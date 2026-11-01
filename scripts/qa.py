#!/usr/bin/env python3
"""Mechanical QA of a rendered video: the checks no reviewer should have to do by eye.

  qa.py final.mp4 --work <work dir>      # writes <work>/qa.json and <work>/qa.md

Checks: streams and duration against the render report and the plan; black frames; a frozen camera
tile (he is always moving, so a still camera means a stalled decode); long stretches where the whole
frame is unchanged (a static screen worth a zoom or a camera layout); loudness (integrated, true
peak, 10 s windows that jump more than 4 LU); dead air longer than 2 s. The video and audio passes
run at the same time on a small proxy decode, so a 3-hour video takes a few minutes.

Exit 0 when clean, 1 when there are findings (they are in qa.md), 2 on a crash.
"""
import argparse
import concurrent.futures as cf
import json
import os
import re
import subprocess
import sys

TARGET_LUFS, LUFS_TOL, TP_MAX = -14.0, 1.5, -1.0


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def clock(s):
    s = max(0, s)
    h, m = int(s // 3600), int(s % 3600 // 60)
    return f"{h}:{m:02d}:{s % 60:04.1f}" if h else f"{m}:{s % 60:04.1f}"


def probe(path):
    code, out = run(["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", path])
    if code:
        raise RuntimeError(out)
    return json.loads(out)


CAMERA = [0.7966, 0.2126, 0.1843, 0.5747]  # cutroom's default camera tile; the plan overrides it


def layout(work):
    """(camera tile, segments) from `cutroom plan --segments`: the tile normalized to the frame, and
    every stretch's mode and camera rect in output pixels. Falls back to the default tile and no
    segments with an old engine."""
    home = os.path.expanduser(os.environ.get("EDITROOM_HOME", "~/.editroom"))
    for c in [os.environ.get("CUTROOM"), os.path.join(home, "bin", "cutroom"), "cutroom"]:
        if not c:
            continue
        for args in ([c, "plan", work, "--segments"], [c, "plan", work]):
            try:
                code, out = run(args)
            except OSError:
                break
            if code:
                continue
            try:
                p = json.loads(out[out.index("{"):])
            except ValueError:
                break
            segs = p.get("segments") or []
            tile = p.get("layout", {}).get("camera") or CAMERA
            fw, fh = p.get("width") or 1920, p.get("height") or 1080
            panel = [sg["camera_rect"] for sg in segs if sg.get("mode") == "panel" and sg.get("camera_rect")]
            if panel:  # the tile the renderer actually drew most
                r = max(set(map(tuple, panel)), key=panel.count)
                tile = [r[0] / fw, r[1] / fh, r[2] / fw, r[3] / fh]
            return tile, segs, (fw, fh)
    return CAMERA, [], (1920, 1080)


def overlap(a0, a1, spans):
    """Seconds of [a0, a1] inside the (start_s, end_s) spans."""
    return sum(max(0.0, min(a1, e) - max(a0, b)) for b, e in spans)


def video_pass(path, cam, dur=None, chunks=None):
    """Black frames, a frozen camera tile, a frozen camera layout and long static frames, on 480p
    proxy decodes. Long videos are split into overlapping chunks decoded in parallel."""
    x, y, w, h = cam
    fc = ("[0:v]scale=480:-2,split=3[a][b1][b2];"
          "[a]blackdetect@whole=d=0.4:pix_th=0.06,freezedetect@whole=n=0.003:d=20[o1];"
          f"[b1]crop=iw*{w}:ih*{h}:iw*{x}:ih*{y},freezedetect@cam=n=0.002:d=2[o2];"
          "[b2]freezedetect@full=n=0.002:d=2[o3]")
    pad = 25.0  # overlap so a freeze or static stretch across a chunk edge is still seen whole

    def one(t0, t1):
        rng = ["-ss", f"{t0:.3f}", "-t", f"{t1 - t0:.3f}"] if t1 else []
        cmd = (["ffmpeg", "-hide_banner", "-nostats", "-hwaccel", "videotoolbox"] + rng + ["-i", path, "-an", "-filter_complex", fc,
               "-map", "[o1]", "-f", "null", "-", "-map", "[o2]", "-f", "null", "-", "-map", "[o3]", "-f", "null", "-"])
        code, out = run(cmd)
        if code:  # no VideoToolbox: software decode
            code, out = run([c for c in cmd if c not in ("-hwaccel", "videotoolbox")])
        off = t0 if t1 else 0.0

        def spans(tag):
            st = [float(v) + off for v in re.findall(rf"freezedetect@{tag} @ \S+\] lavfi\.freezedetect\.freeze_start: ([\d.]+)", out)]
            en = [float(v) + off for v in re.findall(rf"freezedetect@{tag} @ \S+\] lavfi\.freezedetect\.freeze_end: ([\d.]+)", out)]
            return list(zip(st, en + [(t1 or None)] * (len(st) - len(en))))

        black = [(float(a) + off, float(b) + off) for a, b in re.findall(r"black_start:([\d.]+) black_end:([\d.]+)", out)]
        return {"black": black, "camera_freeze": spans("cam"), "full_freeze": spans("full"), "static": spans("whole")}

    n = chunks or max(1, min(8, int((dur or 0) // 180)))
    if n == 1 or not dur:
        return one(0.0, None)
    step = dur / n
    parts = [(max(0.0, i * step - (pad if i else 0)), min(dur, (i + 1) * step)) for i in range(n)]
    with cf.ThreadPoolExecutor(n) as ex:
        res = list(ex.map(lambda p: one(*p), parts))

    def merge(spans):
        out = []
        for a, b in sorted(spans, key=lambda s: s[0]):
            if out and a <= (out[-1][1] or a) + 0.05:
                out[-1] = (out[-1][0], None if b is None or out[-1][1] is None else max(out[-1][1], b))
            else:
                out.append((a, b))
        return out

    return {k: merge([s for r in res for s in r[k]]) for k in ("black", "camera_freeze", "full_freeze", "static")}


def audio_pass(path):
    """Loudness (EBU R128 with true peak) and long silences, from one decode."""
    af = "ebur128=peak=true:framelog=verbose,silencedetect=noise=-45dB:d=2"
    code, out = run(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-vn", "-af", af, "-f", "null", "-"])
    short = [(float(t), float(s)) for t, s in re.findall(r"t:\s*([\d.]+)\s+TARGET:.*?S:\s*(-?[\d.]+|-inf)", out) if s != "-inf"]
    summ = out[out.rfind("Summary:"):] if "Summary:" in out else ""
    m_i = re.search(r"I:\s*(-?[\d.]+) LUFS", summ)
    m_tp = re.search(r"Peak:\s*(-?[\d.]+|-inf) dBFS", summ)
    sil_s = [float(x) for x in re.findall(r"silence_start: (-?[\d.]+)", out)]
    sil_e = [(float(a), float(b)) for a, b in re.findall(r"silence_end: ([\d.]+) \| silence_duration: ([\d.]+)", out)]
    windows = {}
    for t, s in short:
        windows.setdefault(int(t // 10), []).append(s)
    win = [(k * 10, sum(v) / len(v)) for k, v in sorted(windows.items()) if v]
    return {"integrated": float(m_i.group(1)) if m_i else None,
            "true_peak": float(m_tp.group(1)) if m_tp and m_tp.group(1) != "-inf" else None,
            "windows": win,
            "silences": [(e - d, e, d) for e, d in sil_e] or [(s, None, None) for s in sil_s]}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("video")
    ap.add_argument("--work", help="cutroom work dir (for the render report and plan); default: next to the video")
    a = ap.parse_args()
    v = os.path.abspath(a.video)
    work = os.path.abspath(a.work) if a.work else os.path.dirname(v)
    findings = []

    def add(sev, what, at=None, fix=""):
        findings.append({"severity": sev, "what": what, "at": at, "at_clock": clock(at) if at is not None else None, "fix": fix})

    info = probe(v)
    vs = [s for s in info["streams"] if s["codec_type"] == "video"]
    aus = [s for s in info["streams"] if s["codec_type"] == "audio"]
    dur = float(info["format"].get("duration", 0))
    facts = {"video": v, "duration_s": round(dur, 3), "size_mb": round(int(info["format"].get("size", 0)) / 1e6, 1)}
    if not vs:
        add("blocker", "no video stream")
    else:
        s = vs[0]
        facts.update({"width": s.get("width"), "height": s.get("height"), "codec": s.get("codec_name"),
                      "fps": s.get("avg_frame_rate"), "pix_fmt": s.get("pix_fmt")})
        if s.get("pix_fmt") not in ("yuv420p", "yuvj420p", "nv12"):
            add("minor", f"pixel format {s.get('pix_fmt')} (YouTube prefers yuv420p)")
    if not aus:
        add("blocker", "no audio stream", fix="render without --no-audio; check the narration source in source.json")
    else:
        facts.update({"audio_codec": aus[0].get("codec_name"), "sample_rate": aus[0].get("sample_rate"), "channels": aus[0].get("channels")})

    rep_p = v[:-4] + ".report.json"
    if os.path.exists(rep_p):
        rep = json.load(open(rep_p))
        facts["planned_s"] = rep.get("output_s")
        if rep.get("output_s") and abs(rep["output_s"] - dur) > 0.5:
            add("blocker", f"duration {dur:.2f}s differs from the plan {rep['output_s']:.2f}s", fix="re-render; check render.log for a failed chunk")

    cam, segs, (fw, fh) = layout(work)
    tile_px = [round(cam[0] * fw), round(cam[1] * fh), round(cam[2] * fw), round(cam[3] * fh)]
    # where each check applies: the tile while the tile is on screen, the whole frame in camera layout
    tile_on = [(g["start_ms"] / 1000, g["end_ms"] / 1000) for g in segs
               if g.get("camera_rect") and g.get("mode") != "camera"
               and all(abs(a - b) <= 2 for a, b in zip(g["camera_rect"], tile_px))]
    cam_full = [(g["start_ms"] / 1000, g["end_ms"] / 1000) for g in segs if g.get("mode") == "camera"]
    for g in segs:
        if g.get("mode") == "panel" and not g.get("camera_rect") and g["end_ms"] - g["start_ms"] >= 500:
            add("major", f"no camera for {(g['end_ms'] - g['start_ms']) / 1000:.1f}s in the panel layout", g["start_ms"] / 1000,
                "a gap in the camera clips: cut the stretch, or switch it to screen layout")
    with cf.ThreadPoolExecutor(2) as ex:
        fv = ex.submit(video_pass, v, cam, dur)
        fa = ex.submit(audio_pass, v) if aus else None
        vr = fv.result()
        ar = fa.result() if fa else None

    for b0, b1 in vr["black"]:
        if b1 - b0 >= 0.4:
            add("major", f"black for {b1 - b0:.1f}s", b0, "a missing clip or a cut landing on an empty frame: look at a still there")
    freezes = [(f0, f1 or dur, "camera tile", tile_on) for f0, f1 in vr["camera_freeze"]]
    freezes += [(f0, f1 or dur, "camera (full frame)", cam_full) for f0, f1 in vr["full_freeze"]] if segs else []
    for f0, f1, where, spans in freezes:
        # without segments (old engine) every tile freeze counts; with them only where that view is on screen
        length = overlap(f0, f1, spans) if segs else f1 - f0
        if length >= 2.0:
            add("major", f"{where} frozen for {length:.1f}s", f0,
                "a stalled or missing camera clip, or a still held over the camera: look at a still there")
    for f0, f1 in vr["static"]:
        length = (f1 or dur) - f0
        add("minor", f"whole frame unchanged for {length:.0f}s", f0,
            "a static screen: add a zoom or ring on what he is talking about, or a camera layout for the stretch")

    if ar:
        facts.update({"integrated_lufs": ar["integrated"], "true_peak_dbfs": ar["true_peak"]})
        if ar["integrated"] is not None and abs(ar["integrated"] - TARGET_LUFS) > LUFS_TOL:
            add("minor", f"integrated loudness {ar['integrated']:.1f} LUFS (target {TARGET_LUFS:.0f})", fix="render with --loudnorm")
        if ar["true_peak"] is not None and ar["true_peak"] > TP_MAX:
            add("minor", f"true peak {ar['true_peak']:.1f} dBFS (max {TP_MAX:.0f})", fix="render with --loudnorm")
        w = ar["windows"]
        for (t0, l0), (t1, l1) in zip(w, w[1:]):
            if l0 > -40 and l1 > -40 and abs(l1 - l0) > 4:
                add("minor", f"loudness jumps {l1 - l0:+.1f} LU between neighbouring 10 s windows", t1,
                    "a louder or quieter source here (system audio, a different mic take): check its gain")
        for s0, s1, d in ar["silences"]:
            if d and d >= 2 and s0 > 1 and (s1 or dur) < dur - 1:
                add("minor", f"dead air {d:.1f}s", s0, "cut it in edit.yaml (cuts: from/to word ids) unless it shows an on-screen action")

    sev_rank = {"blocker": 0, "major": 1, "minor": 2}
    findings.sort(key=lambda f: (sev_rank[f["severity"]], f["at"] or 0))
    out = {"facts": facts, "findings": findings}
    with open(os.path.join(work, "qa.json"), "w") as f:
        json.dump(out, f, indent=1)

    counts = {k: sum(1 for f in findings if f["severity"] == k) for k in sev_rank}
    md = [f"## QA: {counts['blocker']} blocker, {counts['major']} major, {counts['minor']} minor", "",
          f"- {facts.get('width')}x{facts.get('height')} {facts.get('codec')} @ {facts.get('fps')}, {clock(dur)}, {facts['size_mb']} MB",
          f"- Audio: {facts.get('audio_codec')} {facts.get('sample_rate')} Hz, {facts.get('integrated_lufs')} LUFS, true peak {facts.get('true_peak_dbfs')} dBFS"]
    if findings:
        md += [""] + [f"- **{f['severity']}** {('at ' + f['at_clock'] + ': ') if f['at_clock'] else ''}{f['what']}" + (f" ({f['fix']})" if f['fix'] else "")
                      for f in findings[:60]]
        if len(findings) > 60:
            md.append(f"- ... {len(findings) - 60} more in qa.json")
    with open(os.path.join(work, "qa.md"), "w") as f:
        f.write("\n".join(md) + "\n")
    print("\n".join(md[2:4]))
    print(f"qa: {counts['blocker']} blocker, {counts['major']} major, {counts['minor']} minor -> {os.path.join(work, 'qa.md')}")
    sys.exit(1 if counts["blocker"] or counts["major"] else 0)


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # noqa: BLE001
        print(f"qa.py crashed: {e}", file=sys.stderr)
        sys.exit(2)
