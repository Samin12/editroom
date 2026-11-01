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


# between two words of a name: spaces, a line break, or a whole cue break ("\n\n12\n00:00:01,000 --> 00:00:02,000\n")
SEP = r"(\s+(?:\d+\s*\n\d\d:\d\d:\d\d,\d{3} --> \d\d:\d\d:\d\d,\d{3}\s*\n)?)"


def fix(text, names):
    counts = {}
    for wrong in sorted(names, key=len, reverse=True):
        ww, rw = wrong.split(), names[wrong].split()
        pat = re.compile(r"(?<![\w-])" + SEP.join(f"({re.escape(x)})" for x in ww) + r"(?![\w-])", re.I)

        def rep(m):
            if len(ww) == 1:
                return names[wrong]
            seps = [m.group(2 * i + 2) for i in range(len(ww) - 1)]
            if len(rw) == len(ww):  # word for word, so a name split over two cues stays split the same way
                return "".join(rw[i] + (seps[i] if i < len(seps) else "") for i in range(len(rw)))
            if all("-->" not in x for x in seps):
                return names[wrong]
            return m.group(0)  # different word count across a cue break: leave it

        text, n = pat.subn(rep, text)
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
