#!/usr/bin/env python3
"""editroom pipeline: raw recording -> finished video, every deterministic step in order.

  pipeline.py "tips raw"                       # a Borumi project by (fuzzy) name, or a .bmprojbundle path
  pipeline.py --files --camera c.mp4 --screen s.mov --name "my video"
  pipeline.py "tips raw" --from-stage check    # resume at a stage
  pipeline.py "tips raw" --dry-run             # print the plan, run nothing

Stages: ingest, setup, analyze (privacy + agent at the same time), proofs, drop, check, sheet,
render, qa, report. State lives in <work>/editroom-state.json, per-stage timings in
<work>/editroom-log.json; a rerun continues after the last finished stage.

Gates (the calling agent acts, then resumes with --from-stage):
  exit 10  proofs pending   -> capture work/proofs-pending.json, then --from-stage drop
  exit 11  check has errors -> fix the beats (work/logs/check.txt), then --from-stage check
  exit 12  review sheets    -> look at work/review/*.png, fix, then --from-stage render
Any other non-zero exit is a real failure; the log names the stage and its log file.
"""
import argparse
import concurrent.futures as cf
import datetime as dt
import glob
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
EDITROOM_HOME = os.path.expanduser(os.environ.get("EDITROOM_HOME", "~/.editroom"))
STAGES = ["ingest", "setup", "analyze", "proofs", "drop", "check", "sheet", "render", "qa", "report"]
GATE_PROOFS, GATE_CHECK, GATE_REVIEW = 10, 11, 12
MAX_ROUNDS = 2  # per gate; then proofs are dropped, the review is skipped, check asks for removal


def clock(s):
    h, m = int(s // 3600), int(s % 3600 // 60)
    return f"{h}:{m:02d}:{s % 60:04.1f}" if h else f"{m}:{s % 60:04.1f}"


def now():
    return dt.datetime.now().isoformat(timespec="seconds")


def cutroom_bin():
    env = os.environ.get("CUTROOM")
    for c in [env, os.path.join(EDITROOM_HOME, "bin", "cutroom")]:
        if c and os.access(c, os.X_OK):
            return c
    found = shutil.which("cutroom")
    if found:
        return found
    sys.exit("editroom: no cutroom engine. Run scripts/install.sh first.")


class Run:
    def __init__(self, a):
        self.a = a
        self.cut = cutroom_bin()
        self.work = None
        self.state = {}

    def require_new_engine(self):
        """The plugin needs the merged engine (agent passes, proofs, music, privacy rules)."""
        r1 = subprocess.run([self.cut, "proofs", "--help"], capture_output=True, text=True)
        r2 = subprocess.run([self.cut, "agent", "--help"], capture_output=True, text=True)
        if r1.returncode != 0 or "--cut-model" not in r2.stdout:
            sys.exit(f"editroom: {self.cut} is an old cutroom build. Run: bash {os.path.join(HERE, 'install.sh')} --force")

    # --- state and log ---
    def load_state(self):
        p = os.path.join(self.work, "editroom-state.json")
        self.state = json.load(open(p)) if os.path.exists(p) else {"done": {}}

    def save_state(self):
        with open(os.path.join(self.work, "editroom-state.json"), "w") as f:
            json.dump(self.state, f, indent=1)

    def log(self, entry):
        p = os.path.join(self.work, "editroom-log.json")
        rows = json.load(open(p)) if os.path.exists(p) else []
        rows.append(entry)
        with open(p, "w") as f:
            json.dump(rows, f, indent=1)

    def sh(self, cmd, logname=None, check=True):
        """Run a command, tee to work/logs/<logname>.log, return (code, output)."""
        if self.a.dry_run:
            print("  $ " + " ".join(shlex.quote(c) for c in cmd))
            return 0, ""
        t = time.time()
        r = subprocess.run(cmd, capture_output=True, text=True)
        out = (r.stdout or "") + (r.stderr or "")
        if logname:
            os.makedirs(os.path.join(self.work, "logs"), exist_ok=True)
            with open(os.path.join(self.work, "logs", logname + ".log"), "w") as f:
                f.write(f"$ {' '.join(shlex.quote(c) for c in cmd)}\n# {time.time() - t:.1f}s exit {r.returncode}\n{out}")
        if check and r.returncode != 0:
            raise StageError(f"{os.path.basename(cmd[0])} {cmd[1] if len(cmd) > 1 else ''} failed (exit {r.returncode}); see logs/{logname}.log\n{out[-1500:]}")
        return r.returncode, out


class StageError(Exception):
    pass


class Gate(Exception):
    def __init__(self, code, msg):
        super().__init__(msg)
        self.code = code


# ---------------------------------------------------------------------------------------------
# Resolving the input
# ---------------------------------------------------------------------------------------------

def resolve_project(a):
    if a.files:
        if not (a.camera or a.screen):
            sys.exit("--files needs --camera and/or --screen")
        name = a.name or os.path.splitext(os.path.basename(a.camera or a.screen))[0]
        base = os.path.dirname(os.path.abspath(a.camera or a.screen))
        return {"kind": "files", "name": name, "work_default": os.path.join(base, f"{name} editroom")}
    q = a.project
    if not q:
        sys.exit("name a Borumi project (or a .bmprojbundle path), or use --files")
    p = os.path.expanduser(q.rstrip("/"))
    if p.endswith(".bmprojbundle") and os.path.isdir(p):
        bundle = os.path.abspath(p)
    else:
        r = subprocess.run([sys.executable, "-I", os.path.join(HERE, "find_project.py"), q, "--json", "-n", "3"],
                           capture_output=True, text=True)
        hits = json.loads(r.stdout or "[]") if r.returncode == 0 else []
        if not hits:
            sys.exit(f"editroom: no Borumi project matches {q!r}. Run: find_project.py")
        top = hits[0]
        if top["score"] < 0.9 and len(hits) > 1 and hits[1]["score"] >= top["score"] - 0.05:
            names = "\n".join(f"  {h['name']}  ({h['path']})" for h in hits)
            sys.exit(f"editroom: {q!r} is ambiguous:\n{names}\nPass the bundle path.")
        bundle = top["path"]
    name = a.name or os.path.basename(bundle)[: -len(".bmprojbundle")]
    return {"kind": "borumi", "bundle": bundle, "name": name,
            "work_default": os.path.join(os.path.dirname(bundle), f"{name} editroom")}


# ---------------------------------------------------------------------------------------------
# Stages
# ---------------------------------------------------------------------------------------------

def st_ingest(r, proj):
    a = r.a
    if proj["kind"] == "borumi":
        cmd = [r.cut, "ingest", "borumi", proj["bundle"], r.work]
        if a.timeline:
            cmd.append("--timeline")
        if a.scenes:
            cmd += ["--scenes", a.scenes]
    else:
        cmd = [r.cut, "ingest", "files", r.work]
        for k in ["camera", "screen", "mic", "transcript"]:
            v = getattr(a, k)
            if v:
                cmd += [f"--{k}", os.path.abspath(v)]
    _, out = r.sh(cmd, "ingest")
    lines = [l for l in out.strip().splitlines() if " words, " in l]
    return lines[-1].split(": ", 1)[-1] if lines else ""


def st_setup(r, proj):
    """Defaults the engine reads from the work dir: privacy rules, creator style, music."""
    a = r.a
    tdir = os.path.join(ROOT, "templates")
    for name in ["privacy-rules.yaml", "style.md"]:
        dst = os.path.join(r.work, name)
        src = os.path.join(tdir, name)
        if os.path.exists(src) and not os.path.exists(dst) and not a.dry_run:
            shutil.copy(src, dst)
    if a.music and not a.dry_run:
        os.makedirs(os.path.join(r.work, "music"), exist_ok=True)
        dst = os.path.join("music", os.path.basename(a.music))
        shutil.copy(a.music, os.path.join(r.work, dst))
        set_music(os.path.join(r.work, "edit.yaml"), dst)
    return "privacy rules, style" + (", music" if a.music else "")


def set_music(edit_path, src):
    """Add output.music to edit.yaml without a YAML library (the template's output is a flow map)."""
    txt = open(edit_path).read()
    music = f"music: {{src: {json.dumps(src)}, gain_db: -18, duck_db: 12, fade_ms: 1500}}"
    if "music:" in txt:
        txt = re.sub(r"music:\s*\{[^}]*\}", music, txt)
    elif re.search(r"^output:\s*\{.*\}\s*$", txt, re.M):
        txt = re.sub(r"^(output:\s*\{.*?)\}\s*$", lambda m: m.group(1).rstrip() + ", " + music + "}", txt, count=1, flags=re.M)
    elif re.search(r"^output:\s*$", txt, re.M):
        txt = re.sub(r"^output:\s*$", "output:\n  " + music, txt, count=1, flags=re.M)
    else:
        txt += f"output: {{{music}}}\n"
    open(edit_path, "w").write(txt)


def st_analyze(r, proj):
    """Privacy and the editorial agent at the same time; both must finish."""
    a = r.a
    priv = [r.cut, "privacy", r.work]
    for v in a.allow or []:
        priv += ["--allow", v]
    agent = [r.cut, "agent", r.work, "--model", a.model, "--jobs", str(a.jobs)]
    agent += ["--cut-model", a.cut_model]
    if a.agent_args:
        agent += shlex.split(a.agent_args)
    if a.dry_run:
        r.sh(priv)
        r.sh(agent)
        return ""
    with cf.ThreadPoolExecutor(2) as ex:
        t0 = time.time()
        fp = ex.submit(lambda: (r.sh(priv, "privacy", check=False), time.time() - t0))
        fa = ex.submit(lambda: (r.sh(agent, "agent", check=False), time.time() - t0))
        (pc, pout), pt = fp.result()
        (ac, aout), at = fa.result()
    r.log({"stage": "analyze.privacy", "seconds": round(pt, 1), "exit": pc})
    r.log({"stage": "analyze.agent", "seconds": round(at, 1), "exit": ac})
    if pc != 0:
        raise StageError(f"privacy failed (exit {pc}); see logs/privacy.log\n{pout[-1200:]}")
    if ac != 0:
        raise StageError(f"agent failed (exit {ac}); see logs/agent.log\n{aout[-1200:]}")
    usage = os.path.join(r.work, "agent", "usage.json")
    u = json.load(open(usage)) if os.path.exists(usage) else {}
    return f"privacy {pt:.0f}s, agent {at:.0f}s ({u.get('calls', '?')} calls, ${u.get('cost_usd', 0):.2f})"


def pending_proofs(r):
    code, out = r.sh([r.cut, "proofs", r.work], "proofs", check=False)
    if code != 0:
        raise StageError(f"cutroom proofs failed; see logs/proofs.log\n{out[-800:]}")
    m = re.search(r"\[.*\]", out, re.S)
    data = json.loads(m.group(0)) if m else []
    with open(os.path.join(r.work, "proofs-pending.json"), "w") as f:
        json.dump(data, f, indent=1)
    return data


def gate_round(r, stage):
    """How many times this gate already stopped the run (each gate gets at most MAX_ROUNDS)."""
    return r.state.setdefault("gate_rounds", {}).get(stage, 0)


def st_proofs(r, proj):
    if r.a.dry_run:
        r.sh([r.cut, "proofs", r.work])
        return ""
    data = pending_proofs(r)
    if not data:
        return "no proof requests"
    os.makedirs(os.path.join(r.work, "proof"), exist_ok=True)
    if r.a.no_proof:
        return f"{len(data)} requests, skipped (--no-proof): drop turns them into text beats"
    if r.a.auto_proof:
        auto_capture(r, data)
        left = pending_proofs(r)
        return f"{len(data)} requests, {len(data) - len(left)} captured headless"
    n = gate_round(r, "proofs")
    if n >= MAX_ROUNDS:
        return f"{len(data)} still pending after {n} capture rounds: drop turns them into text beats"
    raise Gate(GATE_PROOFS, f"round {n + 1} of {MAX_ROUNDS}: {len(data)} proof requests in "
                            f"{os.path.join(r.work, 'proofs-pending.json')}. Capture each (one proof-capturer per "
                            "request, all in parallel), then rerun (it continues at drop)")


def auto_capture(r, reqs):
    """Headless proof capture without Claude Code subagents: one `claude -p` per request, 6 at a time."""
    body = open(os.path.join(ROOT, "agents", "proof-capturer.md")).read().split("---", 2)[-1]
    proof_dir = os.path.join(r.work, "proof")

    def one(i, req):
        prompt = (body + "\n\n## Your request\n\n```json\n" + json.dumps(req, indent=1) + "\n```\n"
                  f"\nWork dir: {r.work}\nProof dir: {proof_dir}\nCapture script: node {os.path.join(HERE, 'capture_proof.mjs')}\n")
        cmd = ["claude", "-p", "--model", "sonnet", "--add-dir", r.work,
               "--allowedTools", "Bash(node:*) Bash(gh:*) Bash(curl:*) Bash(ffmpeg:*) Bash(ffprobe:*) Bash(python3:*) Read Write WebSearch WebFetch"]
        res = subprocess.run(cmd, input=prompt, capture_output=True, text=True, timeout=900)
        with open(os.path.join(r.work, "logs", f"proof-{i:02d}.log"), "w") as f:
            f.write(res.stdout + res.stderr)
        return res.returncode

    os.makedirs(os.path.join(r.work, "logs"), exist_ok=True)
    with cf.ThreadPoolExecutor(6) as ex:
        list(ex.map(lambda p: one(*p), enumerate(reqs)))


def st_drop(r, proj):
    """Apply captured proof (highlight + source pill) to its beats, then replace what is still missing."""
    _, out = r.sh([sys.executable, "-I", os.path.join(HERE, "apply_proofs.py"), r.work], "apply-proofs")
    applied = out.strip().splitlines()[-1] if out.strip() else ""
    _, out = r.sh([r.cut, "proofs", r.work, "--drop-missing"], "drop")
    last = out.strip().splitlines()[-1] if out.strip() else ""
    return f"{applied}; {last}"


def st_check(r, proj):
    code, out = r.sh([r.cut, "check", r.work], "check", check=False)
    if r.a.dry_run:
        return ""
    with open(os.path.join(r.work, "logs", "check.txt"), "w") as f:
        f.write(out)
    last = out.strip().splitlines()[-1] if out.strip() else ""
    if code != 0:
        n = gate_round(r, "check")
        todo = ("fix each error named in logs/check.txt" if n < MAX_ROUNDS else
                "round 3+: REMOVE every beat named by an error in logs/check.txt (move it to agent/removed-by-check.yaml)")
        raise Gate(GATE_CHECK, f"round {n + 1}: check found errors ({last}); {todo}, then rerun (it continues at check)")
    return last


def st_sheet(r, proj):
    """Contact sheets, about 20 beats each, so every frame is big enough to read."""
    review = os.path.join(r.work, "review")
    if r.a.dry_run:
        r.sh([r.cut, "plan", r.work])
        r.sh([r.cut, "sheet", r.work, "--at", "o:...", "--cols", "4", "-o", os.path.join(review, "sheet-01.png")])
        return ""
    os.makedirs(review, exist_ok=True)
    for old in glob.glob(os.path.join(review, "sheet-*")):
        os.remove(old)
    _, out = r.sh([r.cut, "plan", r.work], "plan")
    plan = json.loads(out[out.index("{"):])

    def label(b):
        txt = b.get("text") or b.get("title") or b.get("value") or b.get("label") or b.get("src") or ""
        if b.get("kind") == "scene":
            parts = [l.get("text") or l.get("label") or l.get("title") or (f"{l.get('prefix') or ''}{l.get('to')}" if l.get("type") == "counter" else "")
                     for l in b.get("layers", [])]
            txt = f"{len(b.get('layers', []))} layers, {b.get('place', 'stage')}: " + " | ".join(t for t in parts if t)
        if not txt and b.get("items"):
            txt = "; ".join(i if isinstance(i, str) else i.get("text", "") for i in b["items"])
        return f"{b.get('kind')}: {str(txt)[:90]}  ({b.get('origin', '')})"

    def settled(b):
        """Beats that build up (word-by-word titles, bullets, counters, scenes, typing code) are judged
        once built: just before they end. Everything else at its midpoint."""
        s0, s1 = b["start_ms"], b["end_ms"]
        if b.get("kind") in ("scene", "bullets", "title", "number", "code", "quote"):
            return s1 - min(700, 0.15 * (s1 - s0))
        return (s0 + s1) / 2

    marks = [(settled(b), label(b)) for b in plan.get("beats", []) if b.get("kind") != "blur"]
    marks += [((c["card"][0] + c["card"][1]) / 2, f"chapter card {c.get('n')}: {c.get('title')}")
              for c in plan.get("chapters", []) if c.get("card")]
    marks.sort()
    if not marks:
        r.sh([r.cut, "sheet", r.work, "--every", "30", "-o", os.path.join(review, "sheet-01.png")], "sheet")
        n = 1
    else:
        chunks = [marks[i:i + 20] for i in range(0, len(marks), 20)]
        jobs = []
        for i, ch in enumerate(chunks, 1):
            at = ",".join(f"o:{ms / 1000:.2f}" for ms, _ in ch)
            jobs.append([r.cut, "sheet", r.work, "--at", at, "--cols", "4", "-o", os.path.join(review, f"sheet-{i:02d}.png")])
            with open(os.path.join(review, f"sheet-{i:02d}.md"), "w") as f:
                f.write(f"# sheet-{i:02d}.png: frames left to right, top to bottom (4 per row)\n\n")
                for k, (ms, lab) in enumerate(ch, 1):
                    f.write(f"{k}. o:{ms / 1000:.2f} ({clock(ms / 1000)}) {lab}\n")
        with cf.ThreadPoolExecutor(min(4, len(jobs))) as ex:
            list(ex.map(lambda j: r.sh(j, os.path.basename(j[-1])[:-4]), jobs))
        n = len(jobs)
    if r.a.no_review:
        return f"{n} sheets (review skipped)"
    k = gate_round(r, "sheet")
    if k >= MAX_ROUNDS:
        return f"{n} sheets ({k} review rounds done)"
    raise Gate(GATE_REVIEW, f"round {k + 1} of {MAX_ROUNDS}: {n} contact sheets in {review} (frame lists in sheet-NN.md). "
                            "Review them, apply fixes, then rerun: --from-stage check if you changed beats, else plain rerun (it continues at render)")


def next_version(out_dir, name):
    os.makedirs(out_dir, exist_ok=True)
    vs = [int(m.group(1)) for f in os.listdir(out_dir) for m in [re.search(r" - editroom v(\d+)\.mp4$", f)] if m]
    return max(vs, default=0) + 1


def st_render(r, proj):
    a = r.a
    out_dir = os.path.join(r.work, "out")
    v = next_version(out_dir, proj["name"]) if not a.dry_run else 1
    out = os.path.join(out_dir, f"{proj['name']} - editroom v{v}.mp4")
    cmd = [r.cut, "render", r.work, "-o", out, "--loudnorm", "--codec", a.codec]
    if a.height:
        cmd += ["--height", str(a.height)]
    if a.render_args:
        cmd += shlex.split(a.render_args)
    r.sh(cmd, "render")
    r.state["video"] = out
    rep = out[:-4] + ".report.json"
    if os.path.exists(rep):
        j = json.load(open(rep))
        return f"{os.path.basename(out)}: {j.get('output_s', 0) / 60:.1f} min in {j.get('wall_s', 0):.0f}s ({j.get('realtime_x', 0):.1f}x realtime)"
    return os.path.basename(out)


def st_qa(r, proj):
    video = r.state.get("video")
    if r.a.dry_run:
        r.sh([sys.executable, "-I", os.path.join(HERE, "qa.py"), "<video>", "--work", r.work])
        return ""
    if not video or not os.path.exists(video):
        raise StageError("no rendered video in state; run --from-stage render")
    code, out = r.sh([sys.executable, "-I", os.path.join(HERE, "qa.py"), video, "--work", r.work], "qa", check=False)
    if code not in (0, 1):
        raise StageError(f"qa.py crashed; see logs/qa.log\n{out[-800:]}")
    return out.strip().splitlines()[-1] if out.strip() else ""


def st_report(r, proj):
    if r.a.dry_run:
        return ""
    w = r.work
    video = r.state.get("video", "")
    _, stats = r.sh([r.cut, "stats", w], "stats", check=False)
    lines = [f"# {proj['name']}: editroom report", "", f"- Video: `{video}`"]
    for ext, label in [(".chapters.txt", "YouTube chapters"), (".srt", "Captions")]:
        p = video[:-4] + ext if video else ""
        if p and os.path.exists(p):
            lines.append(f"- {label}: `{p}`")
    chap = video[:-4] + ".chapters.txt" if video else ""
    if chap and os.path.exists(chap):
        lines += ["", "## YouTube chapters", "", "```", open(chap).read().strip(), "```"]
    lines += ["", "## Numbers", "", "```", stats.strip(), "```"]
    priv = os.path.join(w, "privacy.json")
    if os.path.exists(priv):
        found = json.load(open(priv))
        kinds = {}
        for f in found:
            kinds[f.get("kind", "?")] = kinds.get(f.get("kind", "?"), 0) + 1
        lines += ["", f"## Privacy: {len(found)} finding{'s' if len(found) != 1 else ''} blurred", ""] + [f"- {k}: {n}" for k, n in sorted(kinds.items())]
    cuts = os.path.join(w, "cuts.yaml")
    if os.path.exists(cuts):
        who = lambda name: sum(1 for l in open(cuts) if re.search(r"""["']?\bby["']?\s*:\s*["']?""" + name + r"\b", l))
        agent_cuts, human_cuts = who("agent"), who("human")
        lines += ["", f"## Cuts: {agent_cuts} editorial cuts by the agent, {human_cuts} by hand", "",
                  f"`{os.path.join(w, 'review.md')}` lists every editorial cut with its time in the video and in the raw "
                  "recording, the words removed and why. The agent can occasionally cut something he meant to keep "
                  "(an instruction, an aside). To restore one: delete its line (`by: agent`) from "
                  f"`{cuts}`, then rerun the pipeline with `--from-stage check` (not `analyze`: rerunning the agent "
                  "regenerates its cuts). To force a cut: add `- {from: wA, to: wB, why: ..., by: human}` (kept forever). "
                  "There is no keep list; changing `by:` to another value does not restore the words."]
    dropped = os.path.join(w, "agent", "dropped.yaml")
    if os.path.exists(dropped):
        lines += ["", "## Beats the agent dropped (failed checks twice)", "", "```yaml", open(dropped).read().strip()[:4000], "```"]
    rows = []
    for p in sorted(glob.glob(os.path.join(w, "proof", "*.source.json"))):
        try:
            rows.append(json.load(open(p)))
        except ValueError:
            pass
    if rows:
        flagged = [x for x in rows if x.get("flag")]
        lines += ["", f"## Proof: {len(rows)} captures, {len(flagged)} flags", ""]
        for x in rows:
            flag = f"  **FLAG:** {x['flag']}" if x.get("flag") else ""
            lines.append(f"- `{x.get('src', '?')}` ({x.get('verdict', '?')}): {x.get('claim', '')} <{x.get('url', '')}>{flag}")
        fixed = [x for x in rows if x.get("caption")]
        if fixed:
            lines += ["", f"### On-screen corrections ({len(fixed)})", "",
                      "The transcript misspelled these; the captions now use the spelling from the source:", ""]
            lines += [f"- `{x.get('src')}`: \"{x['caption']}\"" for x in fixed]
    qa = os.path.join(w, "qa.md")
    if os.path.exists(qa):
        lines += ["", open(qa).read().strip()]
    log = os.path.join(w, "editroom-log.json")
    if os.path.exists(log):
        rows = [x for x in json.load(open(log)) if "seconds" in x]
        lines += ["", "## Time per stage (latest run)", ""]
        seen = {}
        for x in rows:
            seen[x["stage"]] = x["seconds"]
        lines += [f"- {k}: {v:.1f}s" for k, v in seen.items()]
    with open(os.path.join(w, "report.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    return os.path.join(w, "report.md")


RUN = {"ingest": st_ingest, "setup": st_setup, "analyze": st_analyze, "proofs": st_proofs, "drop": st_drop,
       "check": st_check, "sheet": st_sheet, "render": st_render, "qa": st_qa, "report": st_report}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("project", nargs="?", help="Borumi project name or .bmprojbundle path (optional with --work on a started edit)")
    ap.add_argument("--files", action="store_true", help="plain files instead of a Borumi project")
    ap.add_argument("--camera"); ap.add_argument("--screen"); ap.add_argument("--mic"); ap.add_argument("--transcript")
    ap.add_argument("--name", help="video name (default: the project name)")
    ap.add_argument("--work", help="work dir (default: '<project> editroom' next to the bundle)")
    ap.add_argument("--timeline", action="store_true", help="edit what is on the Borumi timeline now, not the raw takes")
    ap.add_argument("--scenes", help="only these scenes, 1-based: 3-5")
    ap.add_argument("--from-stage", choices=STAGES)
    ap.add_argument("--until", choices=STAGES, help="stop after this stage")
    ap.add_argument("--restart", action="store_true", help="forget earlier progress")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--model", default="sonnet", help="model for chapters and visuals")
    ap.add_argument("--cut-model", default="opus", help="model for the editorial cut pass")
    ap.add_argument("--jobs", type=int, default=12, help="parallel agent calls")
    ap.add_argument("--agent-args", help="extra flags for `cutroom agent`")
    ap.add_argument("--allow", action="append", help="privacy: a value that may stay visible (repeatable)")
    ap.add_argument("--music", help="music bed file (ducked under the narration)")
    ap.add_argument("--auto-proof", action="store_true", help="capture proof headless with claude -p (no subagents)")
    ap.add_argument("--no-proof", action="store_true", help="skip proof capture; drop turns requests into text beats")
    ap.add_argument("--no-review", action="store_true", help="skip the contact-sheet review gate")
    ap.add_argument("--codec", default="h264", choices=["h264", "hevc", "x264"])
    ap.add_argument("--height", type=int)
    ap.add_argument("--render-args", help="extra flags for `cutroom render` (never --force)")
    a = ap.parse_args()
    if a.render_args and "--force" in a.render_args:
        sys.exit("editroom never renders with --force: fix what check reports instead")

    wdir = os.path.abspath(os.path.expanduser(a.work)) if a.work else ""
    saved = os.path.join(wdir, "editroom-state.json")
    if not a.project and not a.files and wdir and os.path.exists(saved):
        proj = json.load(open(saved)).get("project") or resolve_project(a)  # resume a work dir by path alone
    elif not a.project and not a.files and wdir and os.path.exists(os.path.join(wdir, "source.json")):
        base = os.path.basename(wdir)  # a started edit without state: its folder names it
        proj = {"kind": "existing", "name": a.name or (base[: -len(" editroom")] if base.endswith(" editroom") else base)}
    else:
        proj = resolve_project(a)
    r = Run(a)
    r.require_new_engine()
    r.work = os.path.abspath(os.path.expanduser(a.work or proj["work_default"]))
    print(f"editroom: {proj['name']} -> {r.work}")
    if not a.dry_run:
        os.makedirs(r.work, exist_ok=True)
        r.load_state()
        if a.restart:
            r.state = {"done": {}}
        r.state["project"] = proj
        r.save_state()

    if a.from_stage:
        start = STAGES.index(a.from_stage)
    elif not a.dry_run and r.state.get("done"):
        last = max(STAGES.index(s) for s in r.state["done"] if s in STAGES)
        start = last + 1
        if r.state["done"][STAGES[last]].get("status") == "gate":
            # proofs -> drop (captured), check -> check again (fixed), sheet -> render (reviewed)
            start = STAGES.index({"proofs": "drop", "check": "check", "sheet": "render"}.get(STAGES[last], STAGES[last]))
        if start >= len(STAGES):
            print(f"editroom: already finished; report at {os.path.join(r.work, 'report.md')}. Use --from-stage to redo a stage.")
            return
    else:
        start = 0
    end = STAGES.index(a.until) + 1 if a.until else len(STAGES)
    if not a.dry_run:
        for later in STAGES[start:]:
            r.state["done"].pop(later, None)
    if not os.path.exists(os.path.join(r.work, "source.json")) and start > 0 and not a.dry_run:
        sys.exit("editroom: nothing ingested yet in this work dir; start without --from-stage")

    t_all = time.time()
    for st in STAGES[start:end]:
        t = time.time()
        print(f"[{st}]", flush=True)
        try:
            note = RUN[st](r, proj)
        except Gate as g:
            secs = time.time() - t
            if not a.dry_run:
                r.log({"stage": st, "at": now(), "seconds": round(secs, 1), "status": "gate"})
                r.state["done"][st] = {"at": now(), "seconds": round(secs, 1), "status": "gate"}
                r.state["gate"] = {"stage": st, "code": g.code, "message": str(g)}
                r.state.setdefault("gate_rounds", {})[st] = r.state["gate_rounds"].get(st, 0) + 1
                r.save_state()
            print(f"GATE {st}: {g}", flush=True)
            sys.exit(g.code)
        except StageError as e:
            if not a.dry_run:
                r.log({"stage": st, "at": now(), "seconds": round(time.time() - t, 1), "status": "failed", "error": str(e)[:2000]})
                r.save_state()
            print(f"FAILED {st}: {e}", file=sys.stderr, flush=True)
            sys.exit(1)
        secs = time.time() - t
        if not a.dry_run:
            r.log({"stage": st, "at": now(), "seconds": round(secs, 1), "status": "ok", "note": note})
            r.state["done"][st] = {"at": now(), "seconds": round(secs, 1), "status": "ok"}
            r.state.pop("gate", None)
            r.save_state()
        if note:
            print(f"  {note}  ({secs:.1f}s)", flush=True)
    print(f"editroom: done in {time.time() - t_all:.0f}s" + (f"; report {os.path.join(r.work, 'report.md')}" if end == len(STAGES) else ""))


if __name__ == "__main__":
    main()
