# Scenes: custom animations, drawn by the engine

A `scene` beat is a small animation made of layers (text, boxes, circles, lines, arrows, images,
code, counters, highlights), each entering on the word that introduces it. Cutroom draws it
natively (0.3–2 ms a frame), so there is no render queue, no Chrome, no Remotion: write 20–60 lines
of YAML, check two stills, done. The engine's own reference is `engine/docs/SCENE.md`; this page is
the short version plus recipes. `cutroom check` names any unknown or missing field.

Use a template (`title`, `bullets`, `number`, `image`, `code`, `quote`) when one says it. Use a
`scene` when the idea needs a structure: a diagram (A talks to B through C), a stat next to its
proof, before/after, a timeline or process that builds, a prompt typing under a headline.

## Shape

```yaml
- at: "connect Claude to Poppy"   # any anchor; lands 130 ms before the word
  until: "the board."             # or hold: 6s (default 6 s)
  kind: scene
  place: stage                    # stage (default, in the panel) | frame (whole frame, camera tile + badge stay on top)
  bg: none                        # none (default) | card (inset card; geometry becomes relative to it) | canvas (repaint, frame scenes)
  layers: [ ... ]                 # 3-9 layers; more = two scenes
```

- `x`, `y`, `w`, `h`: 0..1 of the scene box (panel, frame or card), from its top-left.
- `r` (circle radius): a fraction of the box **height**.
- `size`, `width`, `radius`: units of a 1920-wide box, like the templates (headlines 80–100, body
  36–48, labels ≥ 28; `check` warns under 28).
- `place: frame`: the camera tile covers x 0.80–0.98, y 0.21–0.79 of the frame, so keep layers at
  x < 0.78 (`check` warns). Frame scenes don't show during a `layout: camera` stretch.

## Layers

Every layer: `type`, plus optional `id` (arrows attach to it), `opacity`, `in`, `out`, `keys`, `ease`.

| type | anchor | needs | optional (default) |
|---|---|---|---|
| `text` | `align` edge; `y` = top | `text`, `x`, `y` | `size` (56), `weight` regular/semibold/bold/mono, `align` (left), `w` (max width), `lines` (3), `color` (ink), `style` marker/underline on `*accent*` words |
| `box` | top-left | `x`, `y`, `w`, `h` | `fill` (card2; `none` = outline), `stroke`, `width` (4), `radius` (24), `label`, `sub`, `size`, `color`, `shadow` |
| `circle` | centre | `x`, `y`, `r` | `fill`, `stroke`, `width`, `label`, `size`, `color`, `shadow` |
| `line` | first point | `points: [[x,y],...]` or `from` + `to` | `color` (muted), `width` (4), `dash` |
| `arrow` | `from` | `from`, `to`: `[x, y]` or a box/circle/image `id` | `color` (accent), `width` (6), `curve` (-0.5..0.5), `dash` |
| `image` | top-left | `src`, `x`, `y`, `w`, `h` | `fit` contain/cover, `radius` (20), `source` (pill text), `ring: [x,y,w,h]` (image-normalized ring + spotlight that follows push-ins), `shadow` |
| `code` | top-left | `text`, `x`, `y` | `w` (0.8), `title`, `size` (32); types on at 28 chars/s; does not scale |
| `counter` | like text | `to`, `x`, `y` | `from` (0), `prefix`, `suffix`, `decimals`, `size` (160), `align` (center), `color`; counts over 1 s, commas from 1,000 |
| `highlight` | top-left | `x`, `y`, `w`, `h` | `style` marker/underline, `color` (accent), `radius`. **List it before the layer it sits behind** |

Colours: `accent`, `ink`, `ink_soft`, `muted`, `faint`, `card`, `card2`, `surface`, `canvas`,
`line`, `accent_soft`, `green`, `red`, `gold`, `white`, `black`, or `#RRGGBB` (they follow `style:`).
`*word*` in text, labels and subs draws in the accent colour (on an accent box it flips to canvas).

## Timing

- `in: {at, from, ms, draw}`: `at` defaults to the beat start, staggered 90 ms per layer. `from`:
  `up` (default), `down`, `left`, `right`, `fade`, `scale`, `pop`, `none`. `draw: true` strokes
  outlines on; lines, arrows and highlights always draw on. `ms`: 350 (420 pop, 600 draw).
- `out: {at, to, ms}` leaves early (`to`: fade/up/down/left/right/scale/pop/none). Every layer
  already fades with the beat's last 250 ms.
- `keys: [{t, ms, x, y, scale, opacity, rotate}]`: from `t`, move to these values over `ms` (500).
  `rotate` turns shapes only.
- `ease`: `out` (default), `in_out`, `back`, `linear`.
- Times (`in.at`, `out.at`, `keys[].t`): an anchor (`w1302`, `"the board"`, `w1302+0.3s`, `s:`,
  `o:`), `"+0.4s"` / `"+400ms"` after the beat start, or a bare number of seconds. Phrases match
  words said during the beat first. A time outside the beat is clamped (`check` warns).

## Recipes

Copy the closest one, swap in his words. The four in `engine/docs/SCENE.md` (diagram, stat beside
proof, prompt typing, full-frame timeline) also render in the gfx gallery
(`engine/examples/scenes/*.yaml`).

### Diagram that builds on his words
```yaml
- at: "how Claude talks to Poppy"
  until: "in seconds."
  kind: scene
  layers:
    - {type: text, text: "How *Claude* talks to Poppy", x: 0.07, y: 0.08, size: 88, weight: bold}
    - {id: claude, type: box, x: 0.06, y: 0.4, w: 0.25, h: 0.24, label: Claude, sub: your agent, in: {at: "Claude"}}
    - {id: mcp, type: box, x: 0.375, y: 0.4, w: 0.25, h: 0.24, label: "*MCP*", sub: one URL, fill: accent_soft, stroke: accent, width: 3, in: {at: "MCP", from: pop}}
    - {id: poppy, type: box, x: 0.69, y: 0.4, w: 0.25, h: 0.24, label: Poppy, sub: your data, in: {at: "Poppy"}}
    - {type: arrow, from: claude, to: mcp, in: {at: "talks"}}
    - {type: arrow, from: mcp, to: poppy, in: {at: "asks"}}
```

### Stat next to its proof
```yaml
- at: "ten thousand dollars"
  hold: 6s
  kind: scene
  layers:
    - {type: image, src: proof/launchpoint-rates.png, x: 0.05, y: 0.12, w: 0.54, h: 0.66, source: "launchpoint.app · captured Oct 31, 2026",
       ring: [0.045, 0.205, 0.18, 0.1], in: {from: scale, ms: 500}, keys: [{t: "+0.5s", ms: 4500, scale: 1.05}], ease: in_out}
    - {type: text, text: "PAID FOR ONE VIDEO", x: 0.645, y: 0.25, size: 30, weight: bold, color: accent, in: {at: "+0.4s"}}
    - {type: counter, to: 10075, prefix: "$", x: 0.64, y: 0.31, size: 132, align: left, in: {at: "ten thousand"}}
    - {type: highlight, x: 0.645, y: 0.455, w: 0.2, h: 0.012, style: underline, in: {at: "+1.6s"}}
```
The image `ring` uses the capture's `highlights[].rect` from `capture_proof.mjs` (image-normalized).

### Before / after
```yaml
- at: "before"
  until: "after."
  kind: scene
  layers:
    - {type: text, text: Before, x: 0.27, y: 0.1, size: 48, weight: bold, align: center, color: muted}
    - {type: text, text: "*After*", x: 0.73, y: 0.1, size: 48, weight: bold, align: center, in: {at: "after"}}
    - {type: image, src: proof/before.png, x: 0.05, y: 0.22, w: 0.44, h: 0.62, fit: contain, in: {from: left}}
    - {type: line, from: [0.5, 0.2], to: [0.5, 0.86], color: line, width: 3}
    - {type: image, src: proof/after.png, x: 0.51, y: 0.22, w: 0.44, h: 0.62, fit: contain, in: {at: "after", from: right},
       keys: [{t: "after+0.5s", ms: 4000, scale: 1.04}]}
```

### Steps that build (a timeline)
```yaml
- at: "three steps"
  hold: 7s
  kind: scene
  layers:
    - {type: text, text: "The *3-step* system", x: 0.07, y: 0.08, size: 88, weight: bold}
    - {type: line, points: [[0.15, 0.5], [0.5, 0.5]], color: line, width: 8, in: {at: "film", ms: 600}}
    - {type: line, points: [[0.5, 0.5], [0.85, 0.5]], color: line, width: 8, in: {at: "agent", ms: 600}}
    - {type: circle, x: 0.15, y: 0.5, r: 0.075, fill: accent, label: "1", size: 64, color: white, in: {at: "idea", from: pop}}
    - {type: circle, x: 0.5, y: 0.5, r: 0.075, fill: card2, stroke: accent, width: 4, label: "2", size: 64, in: {at: "film", from: pop}}
    - {type: circle, x: 0.85, y: 0.5, r: 0.075, fill: card2, stroke: accent, width: 4, label: "3", size: 64, in: {at: "agent", from: pop}}
    - {type: text, text: "Find the idea", x: 0.15, y: 0.64, size: 44, align: center, in: {at: "idea"}}
    - {type: text, text: "Film it once", x: 0.5, y: 0.64, size: 44, align: center, in: {at: "film"}}
    - {type: text, text: "Let the *agent* edit", x: 0.85, y: 0.64, size: 44, align: center, in: {at: "agent"}}
```

### A prompt typing on under a marked headline
```yaml
- at: "paste one line"
  hold: 7s
  kind: scene
  layers:
    - {type: text, text: "Paste *one line*. That's it.", x: 0.07, y: 0.08, size: 84, weight: bold, style: marker}
    - {type: code, x: 0.07, y: 0.29, w: 0.86, title: Terminal, size: 34, in: {at: "+0.3s"},
       text: "$ claude mcp add poppy --transport http https://mcp.getpoppy.ai\n> Find the three best hooks in my last 20 videos."}
    - {type: box, x: 0.6, y: 0.78, w: 0.33, h: 0.13, fill: accent, label: "Works in *any* agent", size: 36, color: white, radius: 60, in: {at: "any agent", from: pop}}
```

### Full-frame hero (`place: frame`)
For a chapter opener or a moment that needs the width. Keep every layer at x < 0.78.
```yaml
- at: "here's the whole system"
  hold: 7s
  kind: scene
  place: frame
  bg: canvas
  layers:
    - {type: text, text: "The *whole* system", x: 0.05, y: 0.09, size: 96, weight: bold}
    - {type: image, src: proof/system-map.png, x: 0.05, y: 0.24, w: 0.7, h: 0.66, fit: contain, in: {from: up},
       keys: [{t: "+0.6s", ms: 5000, scale: 1.04}]}
```

## Rules

- Build on his words: each layer's `in.at` is the word that introduces it.
- The key number or name is the biggest thing. Labels 1–3 words, headlines < ~7 words, no em dashes.
- Hold the finished picture 3–6 s; nothing static > ~4 s (a later layer, a slow push via `keys`).
- Only facts he says or a proof file supports; a counter starts at 0 or a sourced value.

## Check it

```bash
C check W                                           # must be clean; warnings name size, timing and camera-tile problems
C still W --at "<the beat's at>+2s" -o /tmp/s.png    # the settled moment; also one per layer entrance
```

## The Remotion escape hatch

Only for a moment cutroom cannot draw (3D, physics, an illustrated character), at most 2 per
video: a job in `~/.borumi-agent/animation-kit` (its `GUIDE.md` and `styles/velour/SKILL.md`),
rendered **one at a time** (`node ~/.borumi-agent/plugin/scripts/job.mjs render <id>`, then
`job.mjs wait <id>`; the kit's lock serializes renders, never start several). h264 for an opaque
clip, ProRes 4444 for transparency. Copy it to `W/clips/` and drop it in:

```yaml
- {at: w800, until: w860, kind: video, src: clips/hero.mov, fit: frame, alpha: true, loop: false}
```

`fit`: `card` (default; on a card in the panel, loops), `panel` (fills the panel slot), `frame`
(the whole frame under the camera tile; with transparency it overlays the footage). `alpha`
defaults to what the file has. Clips that end hold their last frame unless `loop: true`.
