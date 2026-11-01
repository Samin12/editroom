# `scene` beats: custom animations in YAML

A `scene` is a bespoke animation built from layers: text, boxes, circles, lines, arrows,
images, code cards, counters and highlights. Each layer has its own entrance, exit and
keyframes, all anchored to the words he says. The engine draws them natively (tiny-skia),
so a scene costs about 0.3–2 ms a frame at 1080p, and an agent writes it in 20–60 lines.
A hand-made Remotion scene takes about 1,000 lines.

Use a template (`title`, `bullets`, `number`, `image`, `code`, `quote`) when one says it. Use a
`scene` when the idea needs a structure the templates don't have:
- a diagram (A talks to B through C)
- a before/after, or a stat sitting next to its proof
- a timeline or a process that builds step by step
- a prompt typing on under a headline

## Placement and geometry

```yaml
- at: "connect Claude to Poppy"     # any anchor; lands 130 ms (4 frames) before the word
  until: "the board."               # or hold: 6s (default 6 s)
  kind: scene
  place: stage                      # stage (default) | frame
  bg: none                          # none (default) | card | canvas
  layers: [...]
```

- **`place: stage`** draws in the panel slot on the raised panel surface, like every other
  stage beat. The camera tile stays on the right.
- **`place: frame`** draws over the whole frame, on the dot-grid canvas. The camera tile and
  the step badge stay on top. In the default panel layout the tile covers x 0.80–0.98 and
  y 0.21–0.79 of the frame, so keep layers at x < 0.78. `check` warns about layers under it.
- **`bg: card`** adds an inset card, and layer geometry becomes relative to that card.
  **`bg: canvas`** repaints the canvas first (frame scenes).
- **Coordinates:** `x`, `y`, `w`, `h` are 0..1 of the scene box (the panel, the frame or the
  card), measured from its top-left. `r` (circles) is a fraction of the box *height*.
- **Sizes:** `size`, `width` and `radius` are units where the box is 1920 wide, like the
  templates. In the panel, 1 unit is about 0.77 px at 1080p.

## Layers

Every layer has a `type`. Any layer can also take:
- `id`, so arrows can attach to it
- `opacity` (0..1)
- `in`, `out`, `keys` and `ease` (see Timing)

Unknown fields are load errors. `cutroom check` says which fields a type needs and which it
ignores.

| type | anchor (`x`, `y`) | needs | optional |
|---|---|---|---|
| `text` | depends on `align` (left edge / centre / right edge); `y` is the top | `text`, `x`, `y` | `size` (56), `weight` (regular/semibold/bold/mono; bold from 72), `align` (left), `w` (max width), `lines` (3), `color` (ink), `style` (marker/underline the `*accent*` words) |
| `box` | top-left | `x`, `y`, `w`, `h` | `fill` (card2; `none` for an outline), `stroke`, `width` (4), `radius` (24), `label`, `sub`, `size` (label), `color` (label), `shadow` |
| `circle` | centre | `x`, `y`, `r` | `fill`, `stroke`, `width`, `label`, `size`, `color`, `shadow` |
| `line` | first point | `points: [[x,y],...]`, or `from` + `to` | `color` (muted), `width` (4), `dash` |
| `arrow` | `from` | `from`, `to`: `[x, y]` or a box/circle/image `id` | `color` (accent), `width` (6), `curve` (-0.5..0.5), `dash` |
| `image` | top-left of its frame | `src`, `x`, `y`, `w`, `h` | `fit` (contain/cover), `radius` (20), `source` (pill text), `ring: [x,y,w,h]` (image-normalized; ring + spotlight that follows push-ins), `shadow` |
| `code` | top-left | `text`, `x`, `y` | `w` (0.8), `title`, `size` (32). Types on at 28 chars/s |
| `counter` | like text | `to` (number), `x`, `y` | `from` (0), `prefix`, `suffix`, `decimals` (auto), `size` (160), `align` (center), `color`. Counts over 1 s; commas from 1,000 |
| `highlight` | top-left | `x`, `y`, `w`, `h` | `style` (marker/underline), `color` (accent), `radius`. List it *before* the layer it sits behind |

- `*word*` in text, labels and subs is drawn in the accent colour. Punctuation outside the
  stars is fine: `*one line*.`
- On an accent-coloured box, accent words switch to the canvas colour so they stay readable.
- **Colours:** `accent`, `ink`, `ink_soft`, `muted`, `faint`, `card`, `card2`, `surface`,
  `canvas`, `line`, `accent_soft`, `green`, `red`, `gold`, `white`, `black`, or `#RRGGBB`.
  They follow the edit's `style:` theme.
- **Arrows given ids** start and end at the edges of those layers (14 units off), at their
  resting positions.

## Timing

- **`in: {at, from, ms, draw}`:** `at` is when the layer enters.
  - Default: the beat start, staggered 90 ms per layer.
  - `from`: `up` (default: it rises), `down`, `left`, `right`, `fade`, `scale`, `pop` (overshoot), `none`.
  - `draw: true` draws strokes on (outlined boxes, circles). Lines, arrows and highlights always draw on.
  - `ms` defaults: 350, 420 for `pop`, 600 for draw-ons.
- **`out: {at, to, ms}`** leaves before the beat ends: `fade`, `up`, `down`, `left`, `right`,
  `scale`, `pop`, `none`. Every layer already fades with the beat's last 250 ms.
- **`keys: [{t, ms, x, y, scale, opacity, rotate}]`:** from `t`, the layer moves to these
  values over `ms` (default 500).
  - `x`/`y` move its anchor.
  - `scale` scales it about its centre.
  - `rotate` turns shapes only (boxes, circles, lines, arrows); text, images, code and counters ignore it.
- **`ease`:** `out` (default), `in_out`, `back`, `linear`.
- **Times** (`in.at`, `out.at`, `keys[].t`):
  - an anchor (`w1302`, `"the board"`, `w1302+0.3s`, `s:`, `o:`)
  - `"+0.4s"` / `"+400ms"` after the beat starts
  - a bare number of seconds after the beat starts

  Phrases match the words said *during the beat* first. Word anchors land 130 ms early, like
  beats. A time outside the beat window is clamped to it, and `check` warns about it.

## Examples

### 1. Diagram that builds on the words

```yaml
- at: "how Claude talks to Poppy"
  until: "in seconds."
  kind: scene
  layers:
    - {type: text, text: "How *Claude* talks to Poppy", x: 0.07, y: 0.08, size: 88, weight: bold}
    - {type: text, text: "One MCP server. No glue code.", x: 0.07, y: 0.205, size: 40, weight: regular, color: muted, in: {at: "+0.3s"}}
    - {id: claude, type: box, x: 0.06, y: 0.4, w: 0.25, h: 0.24, label: Claude, sub: your agent, in: {at: "Claude", from: up}}
    - {id: mcp, type: box, x: 0.375, y: 0.4, w: 0.25, h: 0.24, label: "*MCP*", sub: one URL, fill: accent_soft, stroke: accent, width: 3, in: {at: "MCP", from: pop}}
    - {id: poppy, type: box, x: 0.69, y: 0.4, w: 0.25, h: 0.24, label: Poppy, sub: your data, in: {at: "Poppy", from: up}}
    - {type: arrow, from: claude, to: mcp, in: {at: "talks"}}
    - {type: arrow, from: mcp, to: poppy, in: {at: "asks"}}
    - {type: arrow, from: [0.815, 0.68], to: [0.185, 0.68], curve: -0.1, color: muted, width: 4, in: {at: "answers", ms: 900}}
    - {type: text, text: "answers come back in *seconds*", x: 0.5, y: 0.8, size: 40, align: center, color: ink_soft, in: {at: "answers"}}
```

### 2. A stat next to its proof

```yaml
- at: "ten thousand dollars"
  hold: 6s
  kind: scene
  layers:
    - {type: image, src: proof/launchpoint-rates.png, x: 0.05, y: 0.12, w: 0.54, h: 0.66, source: launchpoint.app,
       ring: [0.045, 0.205, 0.18, 0.1], in: {from: scale, ms: 500}, keys: [{t: "+0.5s", ms: 4500, scale: 1.05}], ease: in_out}
    - {type: text, text: "PAID FOR ONE VIDEO", x: 0.645, y: 0.25, size: 30, weight: bold, color: accent, in: {at: "+0.4s"}}
    - {type: counter, to: 10075, prefix: "$", x: 0.64, y: 0.31, size: 132, align: left, in: {at: "ten thousand"}}
    - {type: highlight, x: 0.645, y: 0.455, w: 0.2, h: 0.012, style: underline, in: {at: "+1.6s"}}
    - {type: text, text: "LaunchPoint's *campaign* rate for a single AI video", x: 0.645, y: 0.51, w: 0.31, size: 38, weight: regular, color: ink_soft, in: {at: "+1s"}}
```

### 3. A prompt typing on under a marked headline

```yaml
- at: "paste one line"
  hold: 7s
  kind: scene
  layers:
    - {type: text, text: "Paste *one line*. That's it.", x: 0.07, y: 0.08, size: 84, weight: bold, style: marker}
    - {type: code, x: 0.07, y: 0.29, w: 0.86, title: Terminal, size: 34, in: {at: "+0.3s"},
       text: "$ claude mcp add poppy --transport http https://mcp.getpoppy.ai\n# then ask:\n> Find the three best hooks in my last 20 videos."}
    - {type: box, x: 0.6, y: 0.78, w: 0.33, h: 0.13, fill: accent, label: "Works in *any* agent", size: 36, color: white, radius: 60, in: {at: "any agent", from: pop}}
```

### 4. A full-frame timeline

```yaml
- at: "three steps"
  hold: 7s
  kind: scene
  place: frame
  layers:
    - {type: text, text: "The *3-step* system", x: 0.05, y: 0.09, size: 96, weight: bold}
    - {type: line, points: [[0.12, 0.5], [0.39, 0.5]], color: line, width: 8, in: {at: "film", ms: 600}}
    - {type: line, points: [[0.39, 0.5], [0.66, 0.5]], color: line, width: 8, in: {at: "agent", ms: 600}}
    - {type: circle, x: 0.12, y: 0.5, r: 0.075, fill: accent, label: "1", size: 64, color: white, in: {at: "idea", from: pop}}
    - {type: circle, x: 0.39, y: 0.5, r: 0.075, fill: card2, stroke: accent, width: 4, label: "2", size: 64, in: {at: "film", from: pop}}
    - {type: circle, x: 0.66, y: 0.5, r: 0.075, fill: card2, stroke: accent, width: 4, label: "3", size: 64, in: {at: "agent", from: pop}}
    - {type: text, text: "Find the idea", x: 0.12, y: 0.63, size: 44, align: center, in: {at: "idea"}}
    - {type: text, text: "Film it once", x: 0.39, y: 0.63, size: 44, align: center, in: {at: "film"}}
    - {type: text, text: "Let the *agent* edit", x: 0.66, y: 0.63, size: 44, align: center, in: {at: "agent"}}
```

These four (with word anchors replaced by `wN`) render in the gfx gallery:
`cargo run -p cutroom-gfx --release --example gallery` (`examples/scenes/*.yaml`; set
`CUTROOM_GALLERY_MP4=1` for MP4s).

## Tips for agents

**Layout**
- **Grid:** side margins 0.06–0.07. A title band at y 0.06–0.22, content at y 0.3–0.75, a
  footer line at y 0.78–0.9.
- Three columns of boxes: x 0.06 / 0.375 / 0.69 at w 0.25. Two columns: x 0.05 / 0.64.
- **Sizes:** headlines 80–100 units, body 36–48, labels 28 and up. Under 28 doesn't read on
  a phone (`check` warns).
- Aim for 3–9 layers. If a scene needs more, it's two scenes.

**Timing**
- **Land on the word.** Anchor each layer's `in.at` to the word that introduces it. Word
  anchors already land about 4 frames (130 ms) early, which is the 2–4 frame lead viewers
  read as "on time".
- **Hold results 3–6 s.** Size `until`/`hold` so the finished picture stays up while he talks
  about it.
- **Nothing static over about 4 s.** Add the next layer on a later word, a slow push-in
  (`keys: [{t: "+0.5s", ms: 4000, scale: 1.05}]` on an image), or a key that moves the focus.

**Content**
- **Proof first.** When he states a number from a page, show the page as an `image` layer
  with `ring` on the number, plus a `counter` beside it. Never invent numbers he didn't say.
- **Write less text.** He says the words; the scene shows the structure. Labels: 1–3 words.
  Headlines under about 7 words. No em dashes.

**Check it**
- Run `cutroom check work/`; it must be clean.
- Run `cutroom still work/ --at <anchor>+2s -o s.png` at the settled moment, and look at it.
- Run `cutroom sheet work/ --beats` for the whole video.

## Limits

- `rotate` works on shapes only.
- Arrows attach to where their boxes rest, not where keys move them.
- `code` layers don't scale.
- While a layer is scaling, its text is drawn from a pre-rendered sprite, so it can look
  slightly soft. It is pixel-sharp once settled.
- `place: frame` follows the panel layout. A `layout: camera` stretch has no stage, so frame
  scenes don't show during it.
