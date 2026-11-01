# editroom studio kit

The Remotion kit section films and demo overlays are designed in. It is the velour kit behind
Samin's best edits (the tips and Honcho videos), ported from the Borumi agent setup, with one
change: the canvas now lines up with cutroom's, because cutroom composes the final video.

- **Section film** (`mode: behind`): one per section (intro, each tip, CTA). Full-frame 1920x1080,
  OPAQUE: cutroom's dot-grid canvas plus the left panel, with all content in the panel. cutroom
  places it as a `video` beat with `fit: frame` and draws the camera tile and the step badge on
  top, so nothing may sit right of the panel (`LAYOUT.keepClear`, x >= 1500).
- **Demo overlay** (`mode: front`): a TRANSPARENT ProRes 4444 render over his screen recording
  (callouts, rings, proof cards beside the demo). Final-frame pixels, inside the panel only.

## Where things live

| | |
|---|---|
| `studio/kit/` | the template (this folder): engine, styles, fonts, `references/` |
| `~/.editroom/kit/` | the runtime workspace (`EDITROOM_KIT_HOME`): a synced copy with `node_modules`. Never edit it; `job.mjs kit ensure` re-syncs it. |
| `<work>/studio/jobs/<slot>/` | a job: the designer edits files HERE |
| `studio/jobs/job.mjs` | the job runner (create, typecheck, stills, render, sheets) |
| `tools/tweetshot/` | logged-out X post captures with exact phrase boxes |

## A job

```
<work>/studio/jobs/T4/
  job.json      runner-owned (don't edit): id, slot, mode, slot0, frames, renders...
  brief.md      the binding brief (from the brief writer)
  words.json    his words in this slot, with output ms and slot frames (cutroom timing --json)
  Scene.tsx     default export, no props; plus any local components (Handoff.tsx, ...)
  render.json   {"version": N, "notes": "...", "title": "2-3 words"} = "v N is ready to render"
  assets/       files the scene shows: A("file.png") = staticFile("jobs/<id>/file.png")
  out/          renders, logs, status, stills, sheets, history (runner-owned)
```

`job.json` fields the other workstreams rely on:

| field | meaning |
|---|---|
| `id` | Remotion composition id (`<slot-slug>-<6 hex>`) |
| `slot` | the slot name from `cutroom slots` (e.g. `T4`) |
| `mode` | `behind` (opaque section film) or `front` (transparent overlay) |
| `background` | `solid` or `transparent` (follows mode) |
| `slot0` | the slot's start in OUTPUT ms (the canvas phase origin) |
| `frames` | exact frame count (= `durationInFrames`); every render is checked against it |
| `slotEnd` | `slot0 + frames * 1000 / 30` |
| `fps`, `width`, `height` | 30, 1920, 1080 |
| `canvas` | optional overrides of cutroom's canvas (`{bg, dot, pitch, radius, driftPxPerSec}`) when the edit's `style:` changes it |
| `assetDirs` | extra folders mirrored into the kit's `public/` (`--proof-dir` adds the edit's proof folder as `proof/<hash>`; the scaffold's `P()` points there) |
| `cutsHash` | the cuts the slot was made for (a changed cut invalidates the render) |
| `renders[]` | `{version, file, path, frames, codec, bytes, renderMs, queuedMs, elapsedMs, notes, title}`; `lastRenderedVersion` |
| `drafts[]` | the same for `--draft` renders |

**Frame of an output time:** `f = round((ms - slot0) * 30 / 1000)`. `words.json` has every word's
frame already; land beats 2 to 4 frames BEFORE the word (`at(ms) - 3`).

**Renders:** `out/<id>-v<N>.mp4` (h264 crf 14, behind), `out/<id>-v<N>.mov` (ProRes 4444
yuva444p, front), `out/<id>-v<N>-draft.mp4` (960x540 h264, any mode, alpha dropped). Status:
`out/render-status.json` (`render-status-draft.json` for drafts).

## Commands

```bash
J="node <plugin>/studio/jobs/job.mjs"
$J kit ensure                                   # sync the workspace, npm ci when the lock changed, browser
$J create --dir "<work>/studio/jobs/T4" --slot T4 --slot0 248520 --frames 1816 \
   [--mode behind|front] [--title "Agent Handoffs"] [--words words.json] [--brief brief.md] \
   [--proof-dir "<work>/studio/proof"] [--cuts-hash h] [--work "<work>"]
$J typecheck <job>                              # this job only; must be clean
$J stills <job> --frames 0,240,900 --sheet      # one bundle, one browser: ~2 s + ~0.2 s per frame
$J still <job> --frame 900 --out /tmp/f900.png
$J render <job> --draft                         # 960x540, fast; returns at once
$J render <job>                                 # final; needs render.json version > last rendered
$J wait <job> [--draft] --timeout 540           # poll until done (renders queue)
$J sheet <job> [--count 12 | --at 3.5,10 | --every 5]   # frames of the latest render, tiled
$J status <job> | show <job> | signal <job> | list --work <work> | queue | probe <file>
$J reference tips-t4-agent-handoffs --draft     # render a reference job to see it move
```

**Queue:** every render on the Mac takes one of `EDITROOM_RENDER_SLOTS` (default 3) slots in
`~/.editroom/render-queue/`, at `EDITROOM_RENDER_CONCURRENCY` (default 8) Chrome tabs each, so a
design workflow can fire every section at once without starving the machine. A dead holder's slot
is reclaimed. Stills, sheets and typechecks never queue. Each render has a 10-minute stall
watchdog and one retry on transient renderer failures, then is checked frame-exact.

**Measured (M5 Ultra):** T4 reference (60.5 s, 1816 frames): draft 15.6 s, final h264 24.3 s;
T7b overlay (101 s, 3038 frames, ProRes 4444): 39 s; 4 stills from one bundle: 2.0 s bundle +
0.7 s; with one slot, a second render waited 16 s and then ran.

## Components (import from `../../../styles/velour/src`)

Coordinates are stage pixels at 1920x1080 (behind jobs are scaled into the panel by
`VelourCanvas`; front jobs use final-frame pixels). `delay`/`out` are frames.

| component | props |
|---|---|
| `VelourCanvas` | `tone: "cream" \| "dark"` (use dark), `transparent` (front jobs), `panel` (default true) |
| `TipOpener` | `n, name, delay, out, x=80, y=60, dockAt, dockScale=0.5, dockY=28`: the shared "TIP N" + name opener. Always use it, never build your own. |
| `TweetCard` | `src, srcW, srcH` (the tweetshot PNG and its px size), `x, y, w`, `view: [y0, y1]`, `scroll: {from, to, toY}`, `push`, `pushTo`, `highlights: [{box, at, kind}]` (boxes in source PNG px, straight from tweetshot's json), `delay, out` |
| `BrowserCard` | `url, src, x, y, w, h, delay, out, push: [from, to], fit, objectPosition` + children (put `Highlight`s inside) |
| `Highlight` | `x, y, w, h, delay, dur, kind: "ring" \| "box" \| "underline" \| "marker", color, stroke, pad, out` |
| `SourcePill` | `domain, captured, delay, out` ("x.com/karpathy · captured Oct 31, 2026") |
| `CodeChip` | `text, delay, size, out` |
| `Headline` | `text, delay, stagger, size, weight, color, accent: [words], accentColor, align, maxWidth, out` |
| `Kicker` | `text, delay, color, size, rule` ("WHY IT WORKS", "THE TIP") |
| `Reveal`, `Pop`, `FadeOut` | entrance/exit wrappers: `delay, dur, y, x, scale, blur, out, outDur` |
| `CountUp` | `to, from, delay, dur, format: compact \| money \| int \| plain, digits, prefix, suffix` (start at 0 or a sourced value) |
| `Checklist` | `items: [{text, at}], x, y, size, gap, tone, out` |
| `NumberCard` | `n, title, body, tone, x, y, w, h, delay, out, media` |
| `MediaFrame`, `MediaFill`, `Phone` | real footage, muted, at its own aspect |
| `ArrowTo` | `x1, y1, x2, y2, bend, delay, dur, color, width, out` |
| `ChatBubble` | `text, x, y, w, delay, cps, size, from: user \| claude, out` |
| `Pill`, `Stamp`, `StepTracker`, `ChapterSlate`, `MiniTracker`, `CourseRoadmap` | see `styles/velour/src/components.tsx` |

Helpers: `C` (palette; never hard-code hexes), `prog(frame, start, dur, ease)`, `mix(t, a, b)`,
`geoZoom(t, z0, z1)`, `EASE_OUT / EASE_IN_OUT / EASE_IN`, `compact`, `money`. Canvas and layout:
`CUTROOM_CANVAS`, `LAYOUT` (`panel`, `camera`, `badge`, `keepClear`), `canvasFor(id)`, `driftAt(ms)`.

## Rules (from GUIDE.md and the tips designer brief; non-negotiable)

- Only edit your own job folder. Never change `frames`, fps or size; never add `<Audio>`.
- Frame-pure: everything is a function of `useCurrentFrame()`; no `Math.random`, `Date`, CSS
  transitions; seeded rough shapes only.
- Proof first, animation second. A claim gets a REAL capture (tweet via tweetshot, a page in a
  `BrowserCard` with the real domain) with ONE highlight on the exact phrase, keyed to the word
  he says, and a source pill. A concept gets a clean diagram that builds on his words.
- Movement: every proof beat enters (rise + de-blur <= 12 frames), then a slow geometric push
  (<= 4%) toward the highlight. Beats change every 2.5 to 6 s; nothing static for more than ~4 s;
  the panel is never empty for more than a beat and never shows an empty shell; exit before
  entrance; no two texts in one spot; crossfades never dip to black.
- Legibility: body >= 36 px and labels >= 28 px in stage pixels (about 22 px in the final frame
  is the floor). Crop screenshots so their own text clears that bar.
- Honesty: never show a number, name, date or quote that is not in a real proof file or his own
  words; labels are never stronger than his words; no invented counters; his own framing ("10,000
  hours") is never styled as data. When narration and proof disagree, show the proof's true line
  and report it.
- Privacy: no readable third-party names, meeting titles, calendars, member lists, DMs or emails;
  never his paid community's price or checkout; never re-show his own email addresses.
- No em dashes on screen; headlines under ~7 words; header comment in Scene.tsx listing every
  beat (frame, keyed word, what shows, proof file).
- Check with `typecheck` and `stills` (every beat fully composed AND 2 to 3 frames into every
  transition), LOOK at them, then write `render.json` and render.

Start from the references (`references/README.md`): they are the bar.
