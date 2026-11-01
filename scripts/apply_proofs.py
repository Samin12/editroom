#!/usr/bin/env python3
"""Apply captured proof to its image beats: highlight, source pill and corrected caption.

  apply_proofs.py <work dir>

Each proof-capturer writes <work>/proof/<slug>.source.json:
  {"src": "proof/x.png", "claim": "...", "url": "...", "verdict": "supported|rounded|partly|unsupported|not_found",
   "highlight": [x, y, w, h] | null, "source_pill": "x.com/lingxi · captured Oct 31, 2026",
   "caption": "Grok Bot: an engineering intern for agents" | null,   # only when names/terms were misspelled
   "flag": "..." | null}

For every capture whose PNG exists and whose verdict is supported, rounded or partly, the image
beat with that `src` (in <work>/beats/*.yaml or edit.yaml) gets the capture's `highlight`,
`source` (the pill) and `caption` (when given). These OVERWRITE what the agent wrote: the capture
saw the real page, the agent only guessed ("x.com" vs "x.com/lingxi · captured Oct 31, 2026").

Beats come in three styles and each is rewritten in its own style:
  - JSON-object lines (what `cutroom agent` writes): - {"at":"w910","kind":"image","src":"proof/x.png",...}
  - YAML flow lines:                                  - {at: w910, kind: image, src: proof/x.png}
  - YAML block mappings:                              - at: w910\\n    kind: image\\n    src: proof/x.png
Prints one line per change and a summary; idempotent.
"""
import glob
import json
import os
import re
import sys

OK = {"supported", "rounded", "partly"}


# --- value formatting ------------------------------------------------------------------------

def yaml_value(v):
    if isinstance(v, list):
        return "[" + ", ".join(num(x) for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)  # a JSON string is a valid YAML double-quoted scalar


def num(x):
    return f"{float(x):.4f}".rstrip("0").rstrip(".")


# --- JSON-object lines -----------------------------------------------------------------------

JSON_LINE = re.compile(r"^(\s*-\s*)(\{\".*\})\s*$")


def patch_json_line(line, src, fields):
    m = JSON_LINE.match(line)
    if not m:
        return None
    try:
        obj = json.loads(m.group(2))
    except ValueError:
        return None
    if not isinstance(obj, dict) or obj.get("src") != src:
        return None
    keys = list(obj.keys())
    was_sorted = keys == sorted(keys)
    changed = [k for k, v in fields.items() if obj.get(k) != v]
    for k, v in fields.items():
        obj[k] = v
    if was_sorted:
        obj = dict(sorted(obj.items()))
    return m.group(1) + json.dumps(obj, ensure_ascii=False, separators=(",", ":")), changed


# --- YAML flow mappings on one line ----------------------------------------------------------

def top_level_items(s, start, end):
    """Top-level `key: value` items of the flow mapping s[start:end] (inside the braces).
    Returns [(key, key_start, value_start, value_end)] with absolute offsets."""
    items, depth, quote, seg = [], 0, None, start
    i = start
    bounds = []
    while i < end:
        c = s[i]
        if quote:
            if c == "\\" and quote == '"':
                i += 2
                continue
            if c == quote:
                quote = None
        elif c in "\"'":
            quote = c
        elif c in "[{":
            depth += 1
        elif c in "]}":
            depth -= 1
        elif c == "," and depth == 0:
            bounds.append((seg, i))
            seg = i + 1
        i += 1
    bounds.append((seg, end))
    for a, b in bounds:
        m = re.match(r"\s*(['\"]?)([\w-]+)\1\s*:\s*", s[a:b])
        if m:
            vs = a + m.end()
            ve = b
            while ve > vs and s[ve - 1].isspace():
                ve -= 1
            items.append((m.group(2), a, vs, ve))
    return items


def unquote(v):
    v = v.strip()
    if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
        return v[1:-1]
    return v


def patch_flow_line(line, src, fields):
    m = re.match(r"^(\s*-\s*)\{(.*)\}\s*$", line)
    if not m or m.group(2).lstrip().startswith('"'):
        return None
    open_at = len(m.group(1))
    close_at = line.rstrip().rfind("}")
    items = top_level_items(line, open_at + 1, close_at)
    vals = {k: unquote(line[vs:ve]) for k, _, vs, ve in items}
    if vals.get("src") != src:
        return None
    out = line
    changed = []
    # replace existing values right to left so offsets stay valid
    for k, _, vs, ve in sorted(items, key=lambda t: -t[2]):
        if k in fields:
            new = yaml_value(fields[k])
            if out[vs:ve] != new:
                changed.append(k)
            out = out[:vs] + new + out[ve:]
    missing = [k for k in fields if k not in vals]
    if missing:
        close_at = out.rstrip().rfind("}")
        add = ", ".join(f"{k}: {yaml_value(fields[k])}" for k in missing)
        out = out[:close_at].rstrip() + ", " + add + out[close_at:]
        changed += missing
    return out, changed


# --- YAML block mappings ---------------------------------------------------------------------

def patch_block(lines, i, src, fields):
    """lines[i] is a `src:` line of a block mapping. Edits lines in place; returns changed keys, or None."""
    m = re.match(r"^(\s*)(-\s+)?src:\s*(.*?)\s*$", lines[i])
    if not m or unquote(m.group(3)) != src:
        return None
    indent = " " * (len(m.group(1)) + len(m.group(2) or ""))  # the column of the beat's keys
    top = i
    if not m.group(2):  # walk up to the item's "- " line
        while True:
            top -= 1
            if top < 0 or not (lines[top].startswith(indent) or re.match(r"^\s*-\s+", lines[top])):
                return None
            mm = re.match(r"^\s*-\s+", lines[top])
            if mm and len(mm.group(0)) == len(indent):
                break
    bottom = top + 1
    while bottom < len(lines) and lines[bottom].startswith(indent) and lines[bottom].strip():
        bottom += 1
    changed = []
    for k, v in fields.items():
        new = yaml_value(v)
        for j in range(top, bottom):
            pre = r"^\s*-\s+" if j == top else "^" + re.escape(indent)
            mm = re.match("(" + pre + re.escape(k) + r":)[ \t]*(.*)$", lines[j])
            if mm:
                # a value on the following lines (serde_yaml writes lists as "- x" lines at the
                # key's own indent, or deeper): take those lines out and write the value inline
                cont = j + 1
                if not mm.group(2).strip():
                    while cont < bottom and (lines[cont].startswith(indent + "- ") or lines[cont].startswith(indent + " ")):
                        cont += 1
                cur = block_list(lines[j + 1:cont]) if cont > j + 1 else scalar(mm.group(2))
                if not same(cur, v):
                    lines[j:cont] = [f"{mm.group(1)} {new}"]
                    bottom -= cont - j - 1
                    changed.append(k)
                break
        else:
            lines.insert(bottom, f"{indent}{k}: {new}")
            bottom += 1
            changed.append(k)
    return changed


def scalar(raw):
    """A one-line YAML value as Python: a flow list of numbers, or an (un)quoted string."""
    raw = raw.strip()
    if raw.startswith("[") and raw.endswith("]"):
        try:
            return [float(x) for x in raw[1:-1].split(",") if x.strip()]
        except ValueError:
            return raw
    if raw.startswith('"'):
        try:
            return json.loads(raw)
        except ValueError:
            pass
    return unquote(raw)


def same(cur, v):
    if isinstance(v, list):
        return isinstance(cur, list) and len(cur) == len(v) and all(abs(float(a) - float(b)) < 1e-6 for a, b in zip(cur, v))
    return cur == v


def block_list(lines):
    """The numbers of a block sequence ("- 0.46" lines), to compare with the new value."""
    out = []
    for l in lines:
        try:
            out.append(float(l.strip().lstrip("-").strip()))
        except ValueError:
            return None
    return out


def patch_text(txt, src, fields):
    """Apply fields to the beat whose src is `src`. Returns (new_text, changed_keys) or (txt, None) when absent."""
    lines = txt.split("\n")
    for i, line in enumerate(lines):
        if src not in line:
            continue
        for fn in (patch_json_line, patch_flow_line):
            r = fn(line, src, fields)
            if r is not None:
                lines[i], changed = r
                return "\n".join(lines), changed
        if re.match(r"^\s*(-\s+)?src:", line):
            changed = patch_block(lines, i, src, fields)
            if changed is not None:
                return "\n".join(lines), changed
    return txt, None


def fields_for(s):
    f = {}
    if s.get("highlight") and len(s["highlight"]) == 4:
        f["highlight"] = [round(float(v), 4) for v in s["highlight"]]
    if s.get("source_pill"):
        f["source"] = s["source_pill"]
    if s.get("caption"):
        f["caption"] = s["caption"]
    return f


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    w = os.path.abspath(sys.argv[1])
    files = sorted(glob.glob(os.path.join(w, "beats", "*.yaml"))) + [os.path.join(w, "edit.yaml")]
    caps = sorted(glob.glob(os.path.join(w, "proof", "*.source.json")))
    updated = unusable = missing = 0
    for c in caps:
        try:
            s = json.load(open(c))
        except ValueError:
            print(f"skip {os.path.basename(c)}: not JSON")
            unusable += 1
            continue
        src = s.get("src")
        if not src or s.get("verdict") not in OK or not os.path.exists(os.path.join(w, src)):
            unusable += 1
            continue
        fields = fields_for(s)
        if not fields:
            continue
        for f in files:
            if not os.path.exists(f):
                continue
            txt = open(f).read()
            new, changed = patch_text(txt, src, fields)
            if changed is None:
                continue
            if changed:
                open(f, "w").write(new)
                note = f"  (caption corrected: {s['caption']!r})" if "caption" in changed else ""
                print(f"{os.path.relpath(f, w)}: {src}: {', '.join(changed)}{note}")
                updated += 1
            break
        else:
            missing += 1
            print(f"no beat uses {src} (capture kept, nothing to update)")
    print(f"apply_proofs: {updated} beats updated, {unusable} captures not usable, {missing} without a beat, {len(caps)} total")


if __name__ == "__main__":
    main()
