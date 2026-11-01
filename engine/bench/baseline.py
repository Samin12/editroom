#!/usr/bin/env python3
"""Token footprint of a Remotion-per-visual workflow, from its job folders.
usage: baseline.py <animations dir with */job.json>   (job.json has jobDir -> Scene.tsx, brief.md)"""
import glob, json, os, sys

d = sys.argv[1]
jobs = sc = br = renders = 0
secs = 0.0
for j in glob.glob(os.path.join(d, "*/job.json")):
    job = json.load(open(j))
    jd = job.get("jobDir")
    if not jd:
        continue
    jobs += 1
    renders += len(job.get("renders", []))
    secs += (job.get("endMs", 0) - job.get("startMs", 0)) / 1000
    for f in ("Scene.tsx", "brief.md"):
        p = os.path.join(jd, f)
        if os.path.exists(p):
            if f == "Scene.tsx":
                sc += os.path.getsize(p)
            else:
                br += os.path.getsize(p)
print(f"{jobs} visuals, {renders} renders, {secs/3600:.2f} h of animation")
print(f"written: {sc/1e6:.2f} MB of scene code (~{sc/4/1e6:.2f}M tokens, ~{sc/4/max(jobs,1):,.0f} per visual)")
print(f"read:    {br/1e6:.2f} MB of briefs (~{br/4/1e6:.2f}M tokens)")
