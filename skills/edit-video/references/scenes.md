# Scenes: custom animations, drawn by the engine

A `scene` beat is a small keyframed animation made of layers. Cutroom draws it natively and
frame-exact (a few ms per frame), so there is no render queue, no Chrome, no Remotion: write the
YAML, check stills, done. Use it whenever a template is not enough.

> The field names follow the engine contract in `docs/PLAN.md` §A. If the engine's own
> `engine/docs/SCENE.md` says otherwise, the engine wins: `cutroom check` names any unknown field.

## Shape

```yaml
- at: "<his phrase>"          # any anchor: w123, "a phrase", s:754.2, o:61.5
  until: w1290                # or hold: 6s
  kind: scene
  place: stage                # stage = the panel slot (default) | frame = full frame (camera tile stays on top)
  bg: none                    # none | card | canvas
  layers: [ ... ]
```

Coordinates are normalized to the scene's box (0..1; the panel for `stage`, the frame for `frame`).
Colors: `accent`, `ink`, `muted`, `card`, `canvas`, `green`, `red`, `gold`, or hex.

| Layer | Fields |
|---|---|
| `text` | `text` (`*word*` = accent), `x`, `y`, `size` (px at 1080p), `weight` (regular/semibold/bold), `align` (left/center/right), `color` |
| `box` | `x`, `y`, `w`, `h`, `radius`, `fill`, `stroke`, `label` |
| `circle` | `x`, `y`, `r`, `fill`, `stroke` |
| `line` | `points: [[x, y], ...]`, `color`, `width` |
| `arrow` | `from: [x, y]`, `to: [x, y]`, `color`, `width` |
| `image` | `src` (relative to the work dir), `x`, `y`, `w`, `h` |
| `code` | `text`, `x`, `y`, `w`, `size` (types on) |
| `counter` | `from`, `to`, `prefix`, `x`, `y`, `size` (counts up) |
| `highlight` | `x`, `y`, `w`, `h`, `color` (a marker sweep behind a rect) |

Animation, per layer (all optional):
- `in: {at: <anchor or "+0.4s" from the beat start>, from: up|down|left|right|fade|scale|pop, draw: true, ms: 400}`
  (`draw: true` strokes a line, arrow or box outline on)
- `out: {at: ..., ms: 300}`
- `keys: [{t: "+2s", x: …, y: …, scale: 1.08, opacity: 1, rotate: 0}, ...]` keyframes
- `ease: out | in_out | back | linear`
- `id:` to refer to a layer in notes; layers draw in list order (later on top).

**Rules:** build on his words (each layer enters when he says it); the key number or name is the
biggest thing; one idea per scene; no layer static for more than ~4 s (a `keys` push, a second
highlight); body text ≥ 36 px; headlines < 7 words; no em dashes; only facts he says or a proof file
supports; on `place: frame` keep content out of the camera tile (x ≥ 0.78, y 0.2-0.8 by default).

## Recipes

### Diagram build (how a pipeline flows)
```yaml
- at: "you record once"
  until: w2290
  kind: scene
  layers:
    - {type: text, text: "One recording, *every* format", x: 0.5, y: 0.12, size: 56, weight: bold, align: center}
    - {type: box, x: 0.04, y: 0.40, w: 0.26, h: 0.22, radius: 20, fill: card, stroke: muted, label: Record, in: {at: "record", from: left}}
    - {type: arrow, from: [0.31, 0.51], to: [0.38, 0.51], color: accent, width: 6, in: {at: "then", draw: true}}
    - {type: box, x: 0.38, y: 0.40, w: 0.26, h: 0.22, radius: 20, fill: card, stroke: accent, label: Claude edits, in: {at: "Claude", from: up}}
    - {type: arrow, from: [0.65, 0.51], to: [0.72, 0.51], color: accent, width: 6, in: {at: "out comes", draw: true}}
    - {type: box, x: 0.72, y: 0.40, w: 0.24, h: 0.22, radius: 20, fill: card, stroke: muted, label: Video + reels, in: {at: "reels", from: right}}
```

### Stat reveal (he says a number)
```yaml
- at: "ten thousand dollars"
  hold: 5s
  kind: scene
  layers:
    - {type: counter, from: 0, to: 10075, prefix: "$", x: 0.5, y: 0.45, size: 150, in: {from: scale, ms: 900}}
    - {type: text, text: "paid for *one* video", x: 0.5, y: 0.66, size: 44, align: center, color: muted, in: {at: "+0.9s", from: up}}
    - {type: highlight, x: 0.22, y: 0.36, w: 0.56, h: 0.2, color: accent, in: {at: "+1.2s"}}
```

### Before / after
```yaml
- at: "before"
  until: w3120
  kind: scene
  layers:
    - {type: text, text: Before, x: 0.25, y: 0.14, size: 40, weight: bold, align: center, color: muted}
    - {type: text, text: After, x: 0.75, y: 0.14, size: 40, weight: bold, align: center, color: accent, in: {at: "after"}}
    - {type: image, src: proof/before.png, x: 0.03, y: 0.22, w: 0.44, h: 0.6, in: {from: left}}
    - {type: image, src: proof/after.png, x: 0.53, y: 0.22, w: 0.44, h: 0.6, in: {at: "after", from: right}, keys: [{t: "+3s", scale: 1.04}]}
    - {type: line, points: [[0.5, 0.2], [0.5, 0.84]], color: muted, width: 2}
```

### Step list that builds (better than bullets when each step needs a picture)
```yaml
- at: "three things"
  until: w4410
  kind: scene
  layers:
    - {type: text, text: "Three things to set up", x: 0.06, y: 0.12, size: 52, weight: bold}
    - {type: circle, x: 0.09, y: 0.34, r: 0.035, fill: accent, in: {at: "first", from: pop}}
    - {type: text, text: "Install the CLI", x: 0.15, y: 0.34, size: 40, in: {at: "first", from: left}}
    - {type: circle, x: 0.09, y: 0.52, r: 0.035, fill: accent, in: {at: "second", from: pop}}
    - {type: text, text: "Connect your accounts", x: 0.15, y: 0.52, size: 40, in: {at: "second", from: left}}
    - {type: circle, x: 0.09, y: 0.70, r: 0.035, fill: accent, in: {at: "third", from: pop}}
    - {type: text, text: "Run *one* command", x: 0.15, y: 0.70, size: 40, in: {at: "third", from: left}}
```

### A prompt typing on (he reads a prompt aloud)
```yaml
- at: "so I just told it"
  until: w5230
  kind: scene
  layers:
    - {type: box, x: 0.06, y: 0.2, w: 0.88, h: 0.56, radius: 18, fill: card, stroke: muted}
    - {type: code, text: "Edit my video: cut the retakes, add proof for every claim, render it.", x: 0.1, y: 0.3, w: 0.8, size: 38}
    - {type: highlight, x: 0.1, y: 0.42, w: 0.38, h: 0.07, in: {at: "proof"}}
```

### Proof image with a push and a highlight (more motion than the `image` template)
```yaml
- at: "on their pricing page"
  until: w6020
  kind: scene
  layers:
    - {type: image, src: proof/launchpoint-rates.png, x: 0.05, y: 0.08, w: 0.9, h: 0.76, in: {from: scale}, keys: [{t: "+4s", scale: 1.06}]}
    - {type: highlight, x: 0.42, y: 0.47, w: 0.2, h: 0.06, color: accent, in: {at: "ten thousand"}}
    - {type: text, text: "launchpoint.com · captured Oct 31, 2026", x: 0.5, y: 0.92, size: 24, align: center, color: muted}
```

### Full-frame moment (`place: frame`)
Use for a chapter-opening hero or a moment that needs the width. Keep content left of the camera tile.
```yaml
- at: "here's the whole system"
  hold: 7s
  kind: scene
  place: frame
  bg: canvas
  layers:
    - {type: text, text: "The *whole* system", x: 0.38, y: 0.12, size: 64, weight: bold, align: center}
    - {type: image, src: proof/system-map.png, x: 0.04, y: 0.22, w: 0.68, h: 0.68, in: {from: up}, keys: [{t: "+5s", scale: 1.05}]}
```

## Checking a scene

```bash
C check W                                         # errors name the file and field
C still W --at "o:<start + 0.5>" -o /tmp/s1.png    # and at each layer's entrance + 0.2 s, and the end
```

Look for overlaps, text running out of its box or under the camera tile, anything too small, and
anything he did not say. Fix, re-check, done.

## The Remotion escape hatch

Only for a moment cutroom cannot draw (3D, physics, a complex illustration), at most 2 per video:
a job in `~/.borumi-agent/animation-kit` (read its `GUIDE.md` and `styles/velour/SKILL.md`), rendered
**one at a time**: `node ~/.borumi-agent/plugin/scripts/job.mjs render <id>` then `job.mjs wait <id>`
(the kit's render lock already serializes renders; never start several). h264 for an opaque clip,
ProRes 4444 when it needs transparency. Copy the result to `W/clips/` and drop it in:

```yaml
- {at: w800, until: w860, kind: video, src: clips/hero.mov, fit: frame, alpha: true, loop: false}
```
