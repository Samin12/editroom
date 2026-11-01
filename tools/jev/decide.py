#!/usr/bin/env python3
"""decide: one client for typed-decision models (TypeSafe Jev, Levanto Sage).

A decision model answers a question about content (text and/or one image) with a typed answer:
yes/no with a probability, a choice, a score, or (Jev only) a box locating something in the image.
It never writes prose, so it is fast and cheap: the right tool for "is X visible in this frame",
"does this screenshot support the claim", "which of these is best".

    decide.py ask --image frame.png --yesno private:"Is a personal email address readable?"
    decide.py ask --text "..." --choice tone:"How does it read?" --option calm --option angry
    decide.py ask --image proof.png --box hl:"the phrase 'vibe coding'"        (Jev only)
    decide.py run jobs.jsonl -o answers.jsonl --provider both --jobs 16
    decide.py --selftest

A job line (`run`) is {"key": "...", "text": "...", "image": "path.png", "questions": [Q, ...]}, where
Q is {"id", "kind": yesno|choice|score|box, "instructions", "criteria": {"true","false"} (yesno),
"options": [{"option","description"}] (choice), "levels": ["level 0 description", ...] (score)}.

Every answer is normalized to one JSON line:
    {"key","id","provider","model","kind","answer","p","confidence","box","probabilities",
     "latency_ms","input_tokens","cached","error"}
`answer` is "yes"/"no"/null (yesno: null when 0.4 < p < 0.6 for Jev; Sage's own null), the chosen
option, the expected score, or "box". Keys come from $TYPESAFE_API_KEY / $SAGE_API_KEY or from
~/.editroom/.env and ~/.sagecut/.env; they are never printed. Answers are cached on disk under
~/.editroom/cache/decide/, keyed by a hash of the provider, model and the exact request.
"""

import argparse
import base64
import concurrent.futures as cf
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid

JEV_URL = "https://api.typesafe.ai/v1/systemone"
SAGE_URL = "https://sage.levanto.ai"
JEV_MODEL = os.environ.get("JEV_MODEL", "jev-1.14.0")
# $ per 1M input tokens (output is free for answers on both); Sage at the Developer/Free rate
PRICE = {"jev": 0.042, "sage": 0.05}
CACHE = os.path.expanduser(os.environ.get("DECIDE_CACHE", "~/.editroom/cache/decide"))
UNSURE = (0.4, 0.6)  # Jev yes/no probabilities in this band read as "not sure" (null), like Sage's null


# ---------- keys ----------

def _key(var, files):
    v = os.environ.get(var)
    if v:
        return v.strip()
    for f in files:
        try:
            for line in open(os.path.expanduser(f)):
                line = line.strip()
                if line.startswith(var + "="):
                    return line.split("=", 1)[1].strip().strip('"').strip("'")
        except OSError:
            pass
    return None


def jev_key():
    return _key("TYPESAFE_API_KEY", ["~/.editroom/.env"])


def sage_key():
    return _key("SAGE_API_KEY", ["~/.sagecut/.env", "~/.editroom/.env"])


def available():
    return [p for p, k in (("jev", jev_key()), ("sage", sage_key())) if k]


# ---------- images ----------

def prepared_image(path, provider):
    """A copy of the image inside the provider's limits (Jev: 280..1568 px a side, <= 2.46 MP; Sage:
    <= 4096 px, 4 MiB), as PNG or JPEG bytes plus its format. Cached by content hash."""
    data = open(path, "rb").read()
    w, hgt = header_size(data)
    lo, hi = (280, 1568) if provider == "jev" else (1, 4096)
    if data[:3] == b"\xff\xd8\xff" and w and lo <= min(w, hgt) and max(w, hgt) <= hi and w * hgt <= 2_458_624 and len(data) < 4 << 20:
        return data, "jpeg"  # already a JPEG inside the limits: send as is
    h = hashlib.sha256(data).hexdigest()[:20]
    os.makedirs(os.path.join(CACHE, "img"), exist_ok=True)
    limit = 1568 if provider == "jev" else 2048
    out = os.path.join(CACHE, "img", f"{h}-{limit}.jpg")
    if not os.path.exists(out):
        w, hgt = image_size(path)
        tmp = tempfile.mktemp(suffix=".jpg", dir=os.path.join(CACHE, "img"))
        args = ["sips", "-s", "format", "jpeg", "-s", "formatOptions", "90"]
        if max(w, hgt) > limit:
            args += ["-Z", str(limit)]
        r = subprocess.run(args + [path, "--out", tmp], capture_output=True)
        if r.returncode:
            raise RuntimeError(f"sips failed on {path}: {r.stderr.decode()[:200]}")
        if provider == "jev":
            w2, h2 = image_size(tmp)
            if min(w2, h2) < 280:  # pad tiny crops up to the minimum side
                pad = os.path.join(CACHE, "img", f"{h}-pad.jpg")
                subprocess.run(["sips", "-p", str(max(h2, 280)), str(max(w2, 280)), tmp, "--out", pad], capture_output=True)
                shutil.move(pad, tmp)
        os.replace(tmp, out)
    return open(out, "rb").read(), "jpeg"


def header_size(data):
    """Width and height from PNG/JPEG headers, without decoding (0, 0 if unknown)."""
    import struct
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return struct.unpack(">II", data[16:24])
    if data[:2] == b"\xff\xd8":
        i = 2
        while i + 9 < len(data):
            if data[i] != 0xFF:
                i += 1
                continue
            m = data[i + 1]
            if m in (0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF):
                hgt, w = struct.unpack(">HH", data[i + 5:i + 9])
                return w, hgt
            i += 2 + struct.unpack(">H", data[i + 2:i + 4])[0]
    return 0, 0


def image_size(path):
    r = subprocess.run(["sips", "-g", "pixelWidth", "-g", "pixelHeight", path], capture_output=True, text=True)
    w = h = 0
    for line in r.stdout.splitlines():
        if "pixelWidth" in line:
            w = int(line.split()[-1])
        if "pixelHeight" in line:
            h = int(line.split()[-1])
    return w, h


# ---------- HTTP ----------

def post(url, body, headers, timeout=60, tries=4):
    data = json.dumps(body).encode()
    last = None
    for i in range(tries):
        req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json", **headers})
        t0 = time.time()
        try:
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return json.loads(r.read()), (time.time() - t0) * 1000
        except urllib.error.HTTPError as e:
            msg = e.read().decode(errors="replace")[:600]
            last = f"HTTP {e.code}: {msg}"
            if e.code in (429, 500, 502, 503, 504):
                time.sleep(float(e.headers.get("retry-after") or 0) or (1.5 * (i + 1)))
                continue
            raise RuntimeError(last)
        except (urllib.error.URLError, TimeoutError) as e:
            last = str(e)
            time.sleep(1.5 * (i + 1))
    raise RuntimeError(last or "request failed")


# ---------- providers ----------

def jev_request(job):
    """One Jev request for all of a job's questions (they share the state, and the image)."""
    state, assets = {}, {}
    ref = None
    if job.get("image"):
        raw, fmt = prepared_image(job["image"], "jev")
        ref = "img-" + uuid.uuid4().hex[:12]
        assets[ref] = {"type": "image", "format": fmt, "data_base64": base64.b64encode(raw).decode()}
        state["image"] = ref
    if job.get("text"):
        state["text"] = job["text"]
    qs = {}
    for q in job["questions"]:
        kind = q["kind"]
        instr = q["instructions"]
        if ref:
            instr = instr.replace("the image", "the image `image`") if "`image`" not in instr else instr
        if kind == "yesno":
            jq = {"type": "noul", "instructions": instr}
            if q.get("criteria"):
                jq["criteria"] = q["criteria"]
        elif kind == "choice":
            jq = {"type": "choice", "instructions": instr,
                  "criteria": {o["option"]: o.get("description") for o in q["options"]}}
        elif kind == "score":
            jq = {"type": "score", "instructions": instr, "criteria": list(q["levels"])}
        elif kind == "box":
            if not ref:
                raise ValueError("box needs an image")
            jq = {"type": "box", "instructions": instr if "`image`" in instr else f"{instr} in `image`"}
        else:
            raise ValueError(f"Jev has no {kind} question")
        qs[q["id"]] = jq
    body = {"model": JEV_MODEL, "state": state if len(state) > 1 or not job.get("text") else job["text"] if not ref else state,
            "questions": qs}
    if assets:
        body["assets"] = assets
    headers = {"Authorization": f"Bearer {jev_key()}"}
    if ref or any(q["kind"] == "box" for q in job["questions"]):
        headers["TypeSafe-Danger-Zone-Images"] = "true"
    return body, headers


def jev_ask(job):
    body, headers = jev_request(job)
    resp, ms = post(JEV_URL, body, headers)
    toks = resp.get("usage", {}).get("input_tokens")
    out = []
    n = len(job["questions"])
    for q in job["questions"]:
        a = resp["answers"].get(q["id"], {})
        r = {"kind": q["kind"], "answer": None, "p": None, "confidence": None, "box": None, "probabilities": None}
        if a.get("type") == "noul":
            p = a["noul"]
            r["p"] = p
            r["answer"] = None if UNSURE[0] < p < UNSURE[1] else ("yes" if p >= 0.5 else "no")
        elif a.get("type") == "choice":
            r.update(answer=a["choice"], confidence=a.get("confidence"), probabilities=a.get("probabilities"))
        elif a.get("type") == "score":
            r.update(answer=a["score"], confidence=a.get("confidence"))
        elif a.get("type") == "box":
            b = a["box"]
            r.update(answer="box", confidence=a.get("confidence"),
                     box=[b["left"], b["top"], b["right"] - b["left"], b["bottom"] - b["top"]])
        r.update(model=resp.get("model"), latency_ms=round(ms, 1), input_tokens=round(toks / n) if toks else None)
        out.append(r)
    return out


def sage_ask(job):
    content = job.get("text") or ""
    if job.get("image"):
        raw, fmt = prepared_image(job["image"], "sage")
        content = {"kind": "image", "media": f"data:image/{fmt};base64," + base64.b64encode(raw).decode()}
        if job.get("text"):
            content["text"] = job["text"]
    qs = []
    for q in job["questions"]:
        kind = q["kind"]
        if kind == "box":
            raise ValueError("Sage has no box question")
        sq = {"id": q["id"], "kind": {"score": "scale"}.get(kind, kind), "instructions": q["instructions"]}
        if kind == "yesno" and q.get("criteria"):
            c = q["criteria"]
            sq["instructions"] += f" Yes means: {c.get('true')}. No means: {c.get('false')}."
        if kind == "choice":
            sq["options"] = [{"option": o["option"], **({"description": o["description"]} if o.get("description") else {})}
                             for o in q["options"]]
        if kind == "score":
            sq["levels"] = [{"level": i, "description": d} for i, d in enumerate(q["levels"])]
        qs.append(sq)
    body = {"requests": [{"content": content, "questions": qs}]}
    resp, ms = post(SAGE_URL + "/decide/batch", body, {"Authorization": f"Bearer {sage_key()}"})
    usage = resp.get("meta", {}).get("usage", {})
    # Sage bills images separately from the rendered text; billed_input_tokens is the total
    toks = usage.get("billed_input_tokens") or ((usage.get("input_tokens") or 0) + (usage.get("image_tokens") or 0)) or None
    out = []
    for q, a in zip(job["questions"], resp["results"][0]["answers"]):
        r = {"kind": q["kind"], "answer": None, "p": None, "confidence": None, "box": None, "probabilities": None}
        if not a.get("ok"):
            r["error"] = str(a.get("error"))[:300]
        else:
            res = a["result"]["result"]
            if q["kind"] == "yesno":
                r.update(answer=res.get("answer"), p=res.get("probability"))
            elif q["kind"] == "choice":
                r.update(answer=res.get("chosen"), confidence=res.get("probability"),
                         probabilities={x["option"]: x["probability"] for x in res.get("probabilities", [])})
            elif q["kind"] == "score":
                r.update(answer=res.get("expectation"), confidence=res.get("confidence"))
        r.update(model=resp.get("meta", {}).get("model"), latency_ms=round(ms, 1),
                 input_tokens=round(toks / len(job["questions"])) if toks else None)
        out.append(r)
    return out


ASK = {"jev": jev_ask, "sage": sage_ask}


def cache_key(provider, job):
    j = {k: job.get(k) for k in ("text", "questions")}
    if job.get("image"):
        j["image"] = hashlib.sha256(open(job["image"], "rb").read()).hexdigest()
    model = JEV_MODEL if provider == "jev" else "sage"
    return hashlib.sha256(json.dumps([provider, model, j], sort_keys=True).encode()).hexdigest()


def ask(job, provider, use_cache=True):
    """Answers for every question of one job from one provider (a list of normalized dicts)."""
    os.makedirs(CACHE, exist_ok=True)
    ck = os.path.join(CACHE, cache_key(provider, job) + ".json")
    if use_cache and os.path.exists(ck):
        rows = json.load(open(ck))
        for r in rows:  # the same content can come from another job (identical frames): label it as this one
            r["cached"] = True
            r["key"] = job.get("key")
        return rows
    qs = job["questions"]
    if provider == "sage" and any(q["kind"] == "box" for q in qs):
        qs = [q for q in qs if q["kind"] != "box"]
    try:
        rows = ASK[provider]({**job, "questions": qs}) if qs else []
        err = None
    except Exception as e:  # noqa: BLE001 - every failure becomes an error row, never a crash of the batch
        rows, err = [], str(e)[:500]
    out = []
    for q in job["questions"]:
        r = next((x for x, qq in zip(rows, qs) if qq["id"] == q["id"]), None)
        base = {"key": job.get("key"), "id": q["id"], "provider": provider, "cached": False}
        if r is None:
            base.update(kind=q["kind"], answer=None, error=err or f"{provider} cannot answer {q['kind']}")
        else:
            base.update(r)
        out.append(base)
    if err is None:
        json.dump(out, open(ck, "w"))
    return out


def run_jobs(jobs, providers, jobs_n=8, use_cache=True):
    tasks = [(j, p) for j in jobs for p in providers]
    with cf.ThreadPoolExecutor(jobs_n) as ex:
        futs = [ex.submit(ask, j, p, use_cache) for j, p in tasks]
        for f in futs:
            yield from f.result()


# ---------- CLI ----------

def parse_q(spec, kind):
    qid, _, text = spec.partition(":")
    return {"id": qid, "kind": kind, "instructions": text or qid}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--selftest", action="store_true")
    sub = ap.add_subparsers(dest="cmd")
    a1 = sub.add_parser("ask", help="one job from flags")
    a1.add_argument("--text")
    a1.add_argument("--image")
    a1.add_argument("--yesno", action="append", default=[], metavar="ID:QUESTION")
    a1.add_argument("--choice", metavar="ID:QUESTION")
    a1.add_argument("--option", action="append", default=[], metavar="NAME[=DESCRIPTION]")
    a1.add_argument("--score", metavar="ID:QUESTION")
    a1.add_argument("--level", action="append", default=[], metavar="DESCRIPTION")
    a1.add_argument("--box", action="append", default=[], metavar="ID:WHAT")
    a1.add_argument("--provider", default="auto", choices=["auto", "jev", "sage", "both"])
    a1.add_argument("--no-cache", action="store_true")
    a2 = sub.add_parser("run", help="a JSONL file of jobs, answers as JSONL")
    a2.add_argument("jobs")
    a2.add_argument("-o", "--out")
    a2.add_argument("--provider", default="auto", choices=["auto", "jev", "sage", "both"])
    a2.add_argument("--jobs", dest="parallel", type=int, default=8, help="parallel requests")
    a2.add_argument("--no-cache", action="store_true")
    a = ap.parse_args()

    if a.selftest:
        return selftest()
    if not a.cmd:
        ap.print_help()
        return 2
    have = available()
    if not have:
        print(json.dumps({"error": "no decision-model key (TYPESAFE_API_KEY or SAGE_API_KEY)"}))
        return 3
    prov = {"auto": have[:1], "both": have}.get(a.provider, [a.provider])
    if a.cmd == "ask":
        qs = [parse_q(s, "yesno") for s in a.yesno] + [parse_q(s, "box") for s in a.box]
        if a.choice:
            q = parse_q(a.choice, "choice")
            q["options"] = [{"option": o.split("=", 1)[0], "description": (o.split("=", 1) + [None])[1]} for o in a.option]
            qs.append(q)
        if a.score:
            q = parse_q(a.score, "score")
            q["levels"] = a.level
            qs.append(q)
        job = {"key": "ask", "text": a.text, "image": a.image, "questions": qs}
        for r in run_jobs([job], prov, use_cache=not a.no_cache):
            print(json.dumps(r))
        return 0
    jobs = [json.loads(l) for l in open(a.jobs) if l.strip()]
    out = open(a.out, "w") if a.out else sys.stdout
    t0 = time.time()
    n = 0
    for r in run_jobs(jobs, prov, a.parallel, not a.no_cache):
        out.write(json.dumps(r) + "\n")
        n += 1
    if a.out:
        out.close()
        print(json.dumps({"answers": n, "jobs": len(jobs), "providers": prov, "wall_s": round(time.time() - t0, 1)}))
    return 0


def selftest():
    """Live check of each provider that has a key: one text yes/no; Jev also an image yes/no + box."""
    have = available()
    print(json.dumps({"providers_with_keys": have}))
    if not have:
        return 3
    ok = True
    tjob = {"key": "self-text", "text": "Email draft promises guaranteed 40% returns and calls the product risk-free.",
            "questions": [{"id": "review", "kind": "yesno", "instructions": "Does this copy need compliance review before send?"}]}
    img = os.path.join(tempfile.mkdtemp(), "self.png")
    subprocess.run(["sips", "-s", "format", "png", "-z", "400", "600", "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/AlertStopIcon.icns",
                    "--out", img], capture_output=True)
    ijob = {"key": "self-image", "image": img,
            "questions": [{"id": "has_icon", "kind": "yesno", "instructions": "Is there a warning or stop icon in the image?"},
                          {"id": "where", "kind": "box", "instructions": "the warning or stop icon"}]}
    for p in have:
        for job in (tjob, ijob if os.path.exists(img) else None):
            if not job:
                continue
            for r in ask(job, p, use_cache=False):
                good = r.get("error") is None or (p == "sage" and r["kind"] == "box")
                ok &= good
                print(json.dumps({k: r.get(k) for k in ("provider", "key", "id", "answer", "p", "box", "latency_ms", "input_tokens", "error")}))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
