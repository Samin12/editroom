#!/usr/bin/env python3
"""Apply captured proof to its image beats: the highlight rect and the source pill.

  apply_proofs.py <work dir>

Each proof-capturer writes <work>/proof/<slug>.source.json:
  {"src": "proof/x.png", "claim": "...", "url": "...", "verdict": "supported|rounded|partly|unsupported|not_found",
   "highlight": [x, y, w, h] | null, "source_pill": "domain · captured Oct 31, 2026", "flag": "..." | null}

For every capture whose PNG exists and whose verdict is not unsupported/not_found, the image beat
with that `src` in <work>/beats/*.yaml or edit.yaml gets `highlight:` and `source:` when it has
none yet. Flow-style (`- {at: ..., kind: image, src: ...}`) and block-style beats both work.
Prints one line per change; never touches other beats.
"""
import glob
import json
import os
import re
import sys

OK = {"supported", "rounded", "partly"}


def fmt_rect(r):
    return "[" + ", ".join(f"{float(v):.4f}".rstrip("0").rstrip(".") for v in r) + "]"


def patch_text(txt, src, fields):
    """Insert missing fields into the beat whose src is `src`. Returns (new_text, changed_keys)."""
    lines = txt.split("\n")
    src_re = re.compile(r"(^|[\s{,])src:\s*['\"]?" + re.escape(src) + r"['\"]?\s*(?=[,}\n]|$)")
    for i, line in enumerate(lines):
        if not src_re.search(line):
            continue
        if "{" in line and line.rstrip().endswith("}"):  # flow mapping on one line
            body = line.rstrip()
            add = [f"{k}: {v}" for k, v in fields.items() if not re.search(rf"[{{,\s]{k}:", body)]
            if not add:
                return txt, []
            lines[i] = body[:-1].rstrip() + ", " + ", ".join(add) + "}"
            return "\n".join(lines), [a.split(":")[0] for a in add]
        # block mapping: siblings share the src line's indentation (after a possible "- ")
        m = re.match(r"^(\s*)(-\s+)?", line)
        indent = " " * (len(m.group(0)))
        j = i + 1
        block = [line]
        while j < len(lines) and (lines[j].startswith(indent) and lines[j].strip() and not lines[j].lstrip().startswith("- ")):
            block.append(lines[j])
            j += 1
        have = "\n".join(block)
        add = [f"{indent}{k}: {v}" for k, v in fields.items() if not re.search(rf"(^|\n)\s*(-\s+)?{k}:", have)]
        if not add:
            return txt, []
        lines[i + 1:i + 1] = add
        return "\n".join(lines), [a.strip().split(":")[0] for a in add]
    return txt, None


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    w = os.path.abspath(sys.argv[1])
    files = sorted(glob.glob(os.path.join(w, "beats", "*.yaml"))) + [os.path.join(w, "edit.yaml")]
    caps = sorted(glob.glob(os.path.join(w, "proof", "*.source.json")))
    applied = skipped = 0
    for c in caps:
        try:
            s = json.load(open(c))
        except ValueError:
            print(f"skip {os.path.basename(c)}: not JSON")
            continue
        src = s.get("src")
        if not src or s.get("verdict") not in OK or not os.path.exists(os.path.join(w, src)):
            skipped += 1
            continue
        fields = {}
        if s.get("highlight") and len(s["highlight"]) == 4:
            fields["highlight"] = fmt_rect(s["highlight"])
        if s.get("source_pill"):
            fields["source"] = json.dumps(s["source_pill"], ensure_ascii=False)
        if not fields:
            continue
        for f in files:
            if not os.path.exists(f):
                continue
            txt = open(f).read()
            new, keys = patch_text(txt, src, fields)
            if keys is None:
                continue
            if keys:
                open(f, "w").write(new)
                print(f"{os.path.relpath(f, w)}: {src} + {', '.join(keys)}")
                applied += 1
            break
    print(f"apply_proofs: {applied} beats updated, {skipped} captures not usable, {len(caps)} total")


if __name__ == "__main__":
    main()
