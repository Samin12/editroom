# Benchmarks

How the numbers in the top-level README were produced (M5 Ultra, 36 cores).

```
# a Borumi project, read-only
cutroom ingest borumi "<course>.bmprojbundle" work/
cutroom cut work/                         # timing printed
cutroom agent work/ --model sonnet --jobs 12   # prints calls, tokens, cost, wall time; work/agent/usage.json
cutroom stats work/                       # spec size and tokens per beat
cutroom render work/ -o out.mp4           # prints fps / x realtime; out.report.json
cargo run --release -p cutroom-cut --example eval_borumi   # cut precision/recall vs the project's timeline
cargo run --release -p cutroom-gfx --example bench          # ms per frame per graphic
```

`baseline.py` measures the previous workflow from its own artifacts (Remotion job folders).
