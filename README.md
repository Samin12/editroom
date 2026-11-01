# editroom

**Say "get this edited" and a raw recording comes back as a finished video.**

editroom is one Claude Code plugin that edits a raw recording (a Borumi project, or camera +
screen files) into a finished YouTube video: retakes and dead air cut, chapters, real proof
screenshots for every claim, motion graphics and custom animations, privacy blur, a music bed, a
render at ~15x realtime, and a QA report. The heavy lifting is the
[cutroom](https://github.com/AI-Answer/cutroom) Rust engine (vendored in `engine/`); Claude only
makes the editorial calls, in parallel.

It merges Samin's earlier editing skills into one: cutroom (the engine), proof-first-video-editor
and proof-first-video-pipeline (proof and honesty rules, QA), edit-borumi-video and
opencutagent-borumi (the cut rulebook, the treatment), cut-video (retake detection, laugh protection).

## Install

```bash
# in Claude Code
/plugin marketplace add Samin12/editroom
/plugin install editroom@editroom
```

Then build the engine once (the pipeline also rebuilds it whenever `engine/` changes):

```bash
bash ~/.claude/plugins/marketplaces/editroom/scripts/install.sh    # or: bash scripts/install.sh from a clone
```

It checks ffmpeg, tesseract, the `claude` CLI, node and Chrome, builds the engine into
`~/.editroom/bin/cutroom` (it never replaces `~/.cargo/bin/cutroom`, which Creator Club uses) and
adds an `editroom` command next to it.

Without the marketplace: `git clone git@github.com:Samin12/editroom.git && claude --plugin-dir editroom`.

## Use

From any project, in Claude Code:

> get this edited: tips raw

> /editroom:edit "Youtube course raw" music ~/Music/bed.mp3

> edit ~/Desktop/cam.mp4 and ~/Desktop/screen.mov, call it "launch video"

Or straight from a terminal (the same pipeline, minus the parallel proof and review agents):

```bash
~/.editroom/bin/editroom find tips                 # list / fuzzy-find Borumi projects
~/.editroom/bin/editroom run "tips raw" --auto-proof --no-review
```

Output lands in `~/Borumi Projects/<name> editroom/out/<name> - editroom vN.mp4`, with the YouTube
chapters, captions (`.srt`) and `report.md` (what was cut, proof captured, flags, QA, timings).
The Borumi project is only ever read.

## What runs, and what runs in parallel

```
ingest (0.4 s)
  -> privacy OCR  ||  editorial agent: cut review + editorial cuts, chapters, visuals per chapter (12 at once), screen pass
  -> proof capture: one agent per claim, all at once (gate 10)
  -> apply proof, drop what could not be proven
  -> check (gate 11 on errors) -> contact sheets, reviewed in parallel (gate 12)
  -> render (every core, hardware encode) -> QA (video and audio passes at once) -> report
```

Measured on Samin's M5 Ultra (from `engine/README.md` and the plugin's own test run):

| step | old workflow | editroom |
|---|---|---|
| read a 4-hour Borumi project | chunked MCP reads | 0.4 s |
| find retakes, fillers, silences | per-scene LLM judges over two days | 20 ms + one review call |
| chapters + 145 visuals for a 4-hour course | 186 hand-written Remotion scenes, ~2.2M tokens | 22 calls, 78 s, $1.36 |
| render the whole 3:06 course at 1080p | ~80 min Borumi export | 12 min 11 s (15.3x realtime) |
| render an 11-minute edit + QA | n/a | 75 s + 40 s |
| **tips raw, end to end** (20:41 raw → 15:44 final, 64 visuals, 6 real proofs) | an evening of agents, 12 renders at once maxing the Mac | analyze 75 s ($1.23) + proof capture ~2 min (parallel) + render 68 s + QA 17 s |
| secrets on screen | several agents, hours | 26 s per 15-min section, render blocked until blurred |

## Custom animations

Templates cover titles, lists, numbers, proof screenshots, code, quotes, rings, highlights,
arrows, callouts, lower thirds, blur, zoom and layouts. For anything more there is the **`scene`**
beat: a keyframed animation of text, boxes, arrows, counters, images and highlights, timed to his
words and drawn natively by the engine (no Remotion, no Chrome, no render queue):

```yaml
- at: "you record once"
  until: w2290
  kind: scene
  layers:
    - {type: box, x: 0.04, y: 0.40, w: 0.26, h: 0.22, label: Record, in: {at: "record", from: left}}
    - {type: arrow, from: [0.31, 0.51], to: [0.38, 0.51], color: accent, in: {at: "then", draw: true}}
    - {type: box, x: 0.38, y: 0.40, w: 0.26, h: 0.22, label: Claude edits, in: {at: "Claude", from: up}}
```

Ask for one in plain words ("animate the part where I explain the pipeline"); the `scene-designer`
agent writes it and checks its stills. Recipes: `skills/edit-video/references/scenes.md`. For the
rare moment the engine cannot draw, the skill falls back to one Remotion render at a time, dropped
in as a full-frame `video` beat with transparency.

## Layout of this repo

```
.claude-plugin/       plugin.json, marketplace.json
skills/edit-video/    SKILL.md (the one entry point) + references/
agents/               proof-capturer, visual-reviewer, scene-designer
commands/edit.md      /editroom:edit
scripts/              install.sh, pipeline.sh/.py, find_project.py, capture_proof.mjs, apply_proofs.py, qa.py
templates/            privacy-rules.yaml, style.md (copied into each work dir)
engine/               cutroom (Rust), vendored from AI-Answer/cutroom with git subtree
docs/PLAN.md          the engine contracts editroom builds on
```

## Updating the engine

The engine is a `git subtree` of [AI-Answer/cutroom](https://github.com/AI-Answer/cutroom):

```bash
git subtree pull --prefix=engine https://github.com/AI-Answer/cutroom main --squash   # bring upstream in
git subtree push --prefix=engine https://github.com/AI-Answer/cutroom editroom-changes # send changes back
```

Then `bash scripts/install.sh` (or just run the pipeline: it rebuilds when `engine/` changed).

## Other agents (Codex, Grok, Hermes)

`AGENTS.md` points them at the same skill. Without Claude Code subagents, run the pipeline with
`--auto-proof` (headless proof capture with `claude -p`) and `--no-review`.
