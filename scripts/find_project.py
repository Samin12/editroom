#!/usr/bin/env python3
"""List Borumi projects and fuzzy-match one by name.

  find_project.py                 # every project, newest first
  find_project.py "tips raw"      # best matches for a name (exit 1 when nothing matches)
  find_project.py "tips" --json   # machine-readable

Read-only: each project database is copied into memory with SQLite's backup API before it is
read, so a project Borumi has open is never locked or touched.
"""
import argparse
import datetime as dt
import difflib
import json
import os
import sqlite3
import sys

ROOT = os.path.expanduser(os.environ.get("BORUMI_PROJECTS", "~/Borumi Projects"))


def read_bundle(path):
    db = os.path.join(path, "project.bmproj")
    info = {"path": path, "bundle": os.path.basename(path), "name": os.path.basename(path)[: -len(".bmprojbundle")],
            "modified": dt.datetime.fromtimestamp(os.path.getmtime(db if os.path.exists(db) else path)).isoformat(timespec="minutes")}
    if not os.path.exists(db):
        info["error"] = "no project.bmproj"
        return info
    try:
        src = sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=2)
        mem = sqlite3.connect(":memory:")
        src.backup(mem)
        src.close()
        row = mem.execute("select name from projects limit 1").fetchone()
        if row and row[0]:
            info["name"] = row[0]
        info["scenes"] = mem.execute("select count(*) from scenes").fetchone()[0]
        takes = mem.execute("select started_at, ended_at from takes where ended_at is not null").fetchall()
        info["takes"] = len(takes)
        info["raw_minutes"] = round(sum(max(0.0, (e or 0) - (s or 0)) for s, e in takes) / 60, 1)
        mem.close()
    except sqlite3.Error as e:
        info["error"] = str(e)
    return info


def score(query, info):
    q = query.lower().strip()
    names = [info["name"].lower(), info["bundle"].lower()]
    best = 0.0
    for n in names:
        if n == q:
            return 1.0
        s = difflib.SequenceMatcher(None, q, n).ratio()
        if q in n:
            s = max(s, 0.75 + 0.25 * len(q) / max(len(n), 1))
        best = max(best, s)
    return best


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("query", nargs="?")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--root", default=ROOT)
    ap.add_argument("-n", type=int, default=5, help="matches to show")
    a = ap.parse_args()

    if not os.path.isdir(a.root):
        sys.exit(f"no Borumi projects folder at {a.root}")
    bundles = [os.path.join(a.root, d) for d in os.listdir(a.root) if d.endswith(".bmprojbundle")]
    infos = [read_bundle(b) for b in bundles]

    if a.query:
        for i in infos:
            i["score"] = round(score(a.query, i), 3)
        infos = [i for i in infos if i["score"] >= 0.5]
        infos.sort(key=lambda i: i["modified"], reverse=True)
        infos.sort(key=lambda i: i["score"], reverse=True)
        infos = infos[: a.n]
    else:
        infos.sort(key=lambda i: i["modified"], reverse=True)

    if a.json:
        print(json.dumps(infos, indent=1))
    else:
        for i in infos:
            extra = f"  score {i['score']:.2f}" if "score" in i else ""
            stats = f"{i.get('raw_minutes', '?')} min recorded, {i.get('scenes', '?')} scenes" if "error" not in i else i["error"]
            print(f"{i['name']:<45} {i['modified']}  {stats}{extra}\n    {i['path']}")
    if a.query and not infos:
        sys.exit(1)


if __name__ == "__main__":
    main()
