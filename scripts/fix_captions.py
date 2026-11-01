#!/usr/bin/env python3
"""Correct names in a rendered edit's captions: speech-to-text spellings -> the right names.

    fix_captions.py <video.srt> --work <work dir>

Merges the plugin's templates/caption-names.json with <work>/caption-names.json (the work dir's entries win),
replaces whole words case-insensitively (longest first, so "Cloud Code" wins over "Cloud"), keeps the original next
to it as <name>.raw.srt and prints what changed. Only the captions change: on-screen text and audio stay as they are.
"""
import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load_map(work):
    m = {}
    for p in [os.path.join(HERE, "..", "templates", "caption-names.json"), os.path.join(work, "caption-names.json")]:
        try:
            d = json.load(open(p))
        except (OSError, ValueError):
            continue
        m.update({k: v for k, v in d.items() if not k.startswith("_") and isinstance(v, str)})
    return m


def fix(text, names):
    counts = {}
    for wrong in sorted(names, key=len, reverse=True):
        pat = re.compile(r"(?<![\w-])" + r"\s+".join(map(re.escape, wrong.split())) + r"(?![\w-])", re.I)
        text, n = pat.subn(names[wrong], text)
        if n:
            counts[f"{wrong} -> {names[wrong]}"] = n
    return text, counts


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("srt")
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    if not os.path.exists(a.srt):
        sys.exit(f"no captions at {a.srt}")
    raw = a.srt[:-4] + ".raw.srt"
    src = open(raw).read() if os.path.exists(raw) else open(a.srt).read()
    if not os.path.exists(raw):
        open(raw, "w").write(src)
    out, counts = fix(src, load_map(os.path.abspath(a.work)))
    open(a.srt, "w").write(out)
    print(json.dumps({"srt": a.srt, "raw": raw, "fixed": counts}, indent=1))


if __name__ == "__main__":
    main()
