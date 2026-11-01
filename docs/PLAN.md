# editroom build plan

Goal: Samin says "get this edited" and a raw recording (a Borumi project or camera/screen files)
comes back as a finished video, as fast as possible. The Rust engine (`engine/`, vendored from
AI-Answer/cutroom) does every deterministic step. Claude only makes editorial calls, in parallel.

## Pipeline (what the skill runs)

1. `cutroom ingest borumi <bundle> work/` (read-only, under a second)
2. In parallel:
   - `cutroom privacy work/` (OCR secrets and custom patterns, always on)
   - `cutroom agent work/` (editorial cuts, cut review, chapters, visuals per chapter, screen pass)
3. Proof capture: one subagent per pending proof request (`cutroom proofs work/`), then
   `cutroom proofs work/ --drop-missing`.
4. `cutroom check work/` until clean; `cutroom sheet work/ --beats` for one visual review.
5. `cutroom render work/ -o out/final.mp4 --loudnorm` (never `--force`).
6. QA script (ffmpeg detectors) and a short report.

## Engine interfaces (contracts between the three engine workstreams)

Names below are the contract. If a name has to change, change it everywhere and say so in the
hand-back.

### A. `scene` beat: custom animation, native (workstream: scene)

```yaml
- at: "connect Claude to Poppy"      # any anchor
  until: w1290                       # or hold: 6s
  kind: scene
  place: stage                       # stage (the panel slot, default) | frame (full frame, under the camera tile)
  bg: none                           # none | card | canvas
  layers:
    - {id: head, type: text, text: "Claude *talks to* Poppy", x: 0.5, y: 0.14, size: 64, weight: bold, align: center}
    - {id: a, type: box, x: 0.08, y: 0.38, w: 0.32, h: 0.26, radius: 24, fill: card, stroke: accent, label: Claude,
       in: {at: "+0.4s", from: left}}
    - {id: b, type: box, x: 0.60, y: 0.38, w: 0.32, h: 0.26, label: Poppy, in: {at: "Poppy", from: right}}
    - {type: arrow, from: [0.40, 0.51], to: [0.60, 0.51], color: accent, width: 6, in: {at: "talks", draw: true}}
    - {type: counter, from: 0, to: 10075, prefix: "$", x: 0.5, y: 0.82, size: 72, in: {at: w1302}}
    - {type: image, src: proof/rates.png, x: 0.1, y: 0.2, w: 0.8, h: 0.6, in: {from: scale},
       keys: [{t: "+2s", scale: 1.08}]}
```

- Layer types: `text`, `box`, `circle`, `line`, `arrow`, `image`, `code`, `counter`, `highlight`
  (a marker sweep behind a rect).
- Coordinates are normalized to the scene's box (the panel slot for `stage`, the frame for `frame`).
- Colors: theme names (`accent`, `ink`, `muted`, `card`, `canvas`, `green`, `red`, `gold`) or hex.
- Animation per layer: `in` / `out` (`at` anchor or `+N s` relative to the beat start, `from:
  up|down|left|right|fade|scale|pop`, `draw: true` for strokes, `ms`), `keys` (keyframes `t`, `x`,
  `y`, `scale`, `opacity`, `rotate`), `ease: out|in_out|back|linear`. Frame-pure.
- `*word*` in text draws in the accent color, like the other templates.

### B. `video` beat, music bed, privacy rules (workstream: render)

```yaml
- {at: w800, until: w860, kind: video, src: clips/hero.mov, fit: frame, alpha: true, loop: false}
```

- `fit: card` (current look, default) | `panel` (fills the panel slot) | `frame` (full frame, under
  the camera tile). `alpha: true` composites transparency correctly (premultiply). `loop` default
  true for card, false for frame.

```yaml
output:
  music: {src: music/bed.mp3, gain_db: -18, duck_db: -12, attack_ms: 60, release_ms: 450, fade_ms: 1500}
```

- Music loops under the whole video, ducks under narration, fades in and out.
- With no mic clip, narration falls back to camera audio, then system audio.

Privacy custom patterns, in `work/privacy-rules.yaml` (also `--pattern name=regex`):

```yaml
patterns:
  - {name: community-price, regex: '\$\s?\d+(?:\.\d{2})?\s*/\s*(?:mo|month)'}
allow: [samin@aianswer.us]
```

### C. Agent upgrades (workstream: agent)

- Editorial cut pass (default on, `--no-editorial`, `--cut-model opus`): overlapping chunks,
  retakes / false starts / abandoned sentences / dead air the detector missed, keep the LAST good
  take, merged into `cuts.yaml` with a reason.
- Beats prompt covers every stage kind plus `scene` (1–3 hero moments per chapter) and `image`
  proof beats with a `proof` request:

```yaml
- {at: w1340, kind: image, src: proof/launchpoint-rates.png,
   proof: {claim: "LaunchPoint charges $10,075 per video", find: "launchpoint.com pricing page"}}
```

- `cutroom proofs <dir>` prints pending proof requests as JSON (image beats whose `src` is missing);
  `--drop-missing` turns each still-missing one into a `title`/`callout` with the claim text, or
  removes it. Lint treats a missing image that has a `proof` request as pending, not fatal, until
  `--drop-missing` runs; `render` still refuses while any are pending.
- Screen pass (default on, `--no-screen`): for `[screen]` stretches, render a still, let Claude
  look at it (Read tool) and place `ring` / `highlight` / `zoom` / `arrow` / `callout` beats.
- Beats that still fail after the repair call are dropped and logged to `agent/dropped.yaml`.
- Optional `work/style.md` is appended to the system prompt (creator voice and look rules).
