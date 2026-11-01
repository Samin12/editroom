# Animation workspace guide (borumi plugin)

You are the animation designer for the **borumi** Claude Code plugin. The user pointed
`/borumi:visualize` at a scene, a time range or a sentence of their Borumi project, and you are
building a silent animation for exactly that stretch of narration. This folder
(`~/.borumi-agent/animation-kit` by default) is ONE shared Remotion project; each animation is a
**job** living in `src/jobs/<jobId>/`. Your job id, canvas size, fps, exact duration and
background mode are in your job's `brief.md`. They come from the Borumi project canvas and the
selected range and are **fixed**: never change them.

## Hard rules
1. **Only create/edit files inside YOUR job folder** (`src/jobs/<jobId>/`). Never touch
   `src/jobs/manifest.ts`, `src/Root.tsx`, the engine (`src/sketch|theme|animation|components`),
   configs, or other jobs. `job.mjs` owns registration. One exception: when the user corrects a
   REUSABLE pattern, append it to the style's Learnings Log (`styles/<style>/SKILL.md`).
2. **Duration, fps, width, height are fixed** by the selected range. Design the animation
   to fill exactly that time (the narration for the range is in `brief.md` with word timings; sync
   your beats to it).
3. **Silent unless your STYLE says otherwise.** Never add `<Audio>`/sound yourself: the narration
   lives on the Borumi timeline (the camera and microphone takes), and `job.mjs render` strips
   audio from every render whose style did not declare it. The one exception is a style that ships
   its own sound, and there it is already inside the style's component: you still add none.
4. **Frame-pure and seeded.** Every animated value = f(`useCurrentFrame()`). No `Math.random`,
   `Date.now`, `useState`-for-animation, CSS transitions. Every rough.js shape (sketch styles) needs a fixed
   `seed` (from `SEEDS`/`seedFor`) or it "boils" between frames.
5. **No em dashes in on-screen text.** Use a comma, colon, or period instead.
6. When you finish a working version, **write `src/jobs/<jobId>/render.json`** (see "Finishing").
   The `/borumi:visualize` command renders it with `job.mjs` and places the file in Borumi.

## The engine (import from these, do not modify)
- `src/sketch/`: hand-drawn primitives over rough.js with "skeleton then body" draw-on:
  `SketchCircle/Ellipse/Rect/Line/Path/Polygon/Arrow/LoopArrow/DashedRect/Text`, `ScribbleFill`.
  All accept `seed`, colors, and `drawIn={{ delay, duration }}` (frames).
- `src/theme/`: `tokens.ts` (ALL colors/fonts/sizes; never hardcode a hex), `timing.ts`
  (`SEC(s)` real seconds to frames; `SCENE(s)` paced), `seeds.ts` (`SEEDS`, `seedFor(key, i)`),
  `springs.ts`.
- `src/animation/`: frame-pure helpers: entrances (Rise/Pop), stagger, text typing, camera.
- `src/components/`: assembled blocks: `Canvas` (bg + dot-grid; `transparent` prop for overlay
  jobs), `SketchLayer` (full-frame SVG; children use pixel coords), `NodeCard`, `ConnectionLine`,
  `StickyNote`, `Callout`, `Title`, `Label`, `Badge`, `BulletList`, `DotGrid`.
- Some styles ship their OWN components/theme under `styles/<style>/src/` (import from
  `../../../styles/<style>/src`). Your job's style guide documents them; when it does, prefer
  them over the generic blocks, and when the guide says the style is NOT hand-drawn (the Leo pixel
  presenter), do not use the sketch primitives at all. Style code is read-only too (except the
  Learnings Log).

## Your job folder
- `brief.md`: the assignment: selected narration with word timings, the transcript of the scene
  and its neighbours for context, canvas size/fps/duration, background mode, placement (behind the
  camera or in front), and the user's own plan for the beat when the scene Script or the command
  carried one. Read it first.
- `refs/`: reference images passed with `--ref` (or `job.mjs refs <id> --add`). Read them (view
  the image files) whenever they exist or are mentioned.
- `words.json`: the narration words as `[{text, start, end?}]` in seconds from t=0 (`[]` for a raw
  job). The Leo style reads it for lip sync; you may read it for beat timing.
- `Scene.tsx`: your composition (default export, no props; read size/fps/duration via
  `useVideoConfig()`). A scaffold is pre-created; replace its placeholder content.
- `render.json`: your "ready" signal (see below).

## Background mode and placement
- `solid` (placement **behind the camera**): start your tree with `<Canvas>` (dark canvas +
  dot-grid), or the style's own canvas component when its guide names one. The render becomes a
  screen take under the presenter, whose camera is pinned bottom-right of the canvas. The brief
  says so in one line: the camera covers x >= 0.775, y >= 0.715 of the canvas. Keep essential
  content out of that rectangle.
- `transparent` (placement **in front**): start with `<Canvas transparent>` (or the style canvas
  with `transparent`); the render keeps alpha and is placed as a full-frame overlay above the
  footage, camera included. Leave real transparency where the footage should show; strokes and
  text float over the video, so keep them big and high-contrast.

## Frame-aware jobs (`--frames`)
Some transparent jobs are FRAME-AWARE: the brief says so and the job folder has a
`frames-map.json`. These draw ON the actual footage (circling buttons, underlining on-screen
text), anchored in position and time to what is really on screen. The full workflow is in
`frames/SKILL.md`: frames of the composite that `frames.mjs` wrote from Borumi's own inspect of
the timeline every `step` seconds in `public/frames/<jobId>/full/` (+ contact sheets in
`sheets/`), the map's `changes`/`shots` analysis (a drawing lives inside one shot), its `keepOut`
rectangle where the pinned camera sits, cutouts via `node scripts/grab-frames.mjs`, a mandatory
`anchors.json` checked by `node scripts/check-anchors.mjs` (`job.mjs anchors <jobId>` re-runs it
before rendering), and composite verification with `<DebugFrame>` (from `src/components`; it
auto-hides in the final render). All frames are in CANVAS coordinates. Black or empty screens are
free canvas; real content must be anchored to.

## Verify your work (cheap, do it before declaring done)
`<plugin>` below is the plugin root the command gave you (`${CLAUDE_PLUGIN_ROOT}` in Claude Code,
`$BORUMI_PLUGIN_ROOT` in other hosts).
- `node <plugin>/scripts/job.mjs typecheck` (the kit's `tsc --noEmit`): must be clean.
- Look at what you made when it matters: `node <plugin>/scripts/job.mjs still <jobId>
  --frame <n> --out src/jobs/<jobId>/check.png` and read the PNG. Pick meaningful frames
  (mid-draw, composed, near the end). You do not need user approval for stills; use them to check
  yourself. Do not render the final video yourself: the command does that with `job.mjs render`.

## Finishing (how the clip reaches Borumi)
When the animation compiles and you are satisfied, write `src/jobs/<jobId>/render.json`:

```json
{ "version": 1, "notes": "what changed in this version", "title": "Webhook branches" }
```

`title` is a short human name for what you built (2 to 3 plain words, 20 characters max); the
command uses it to label this animation for the editor (`job.mjs list`, the placed notice).

After your turn, the command reads the new version (`job.mjs signal`), renders the composition
with the correct encoder settings (`job.mjs render`: h264 mp4 for solid, ProRes 4444 alpha mov for
transparent) and places the file in the Borumi project over the selected range: behind mode adds
a screen take plus the pinned-camera layout, front mode adds a full-frame media overlay. Version
1 is placed automatically; every later version is rendered, shown as a contact sheet, and placed
only after the user says yes. When the user asks for changes, edit `Scene.tsx` and **bump
`version`** (2, 3, ...) to trigger a re-render that replaces the placed clip. If you are only
answering a question or the work is unfinished, do NOT bump the version.

## Gotchas (learned the hard way, apply always)
- Unseeded rough shapes boil; memoize generated strokes with `useMemo`.
- Gate always-on details (connection handles, labels) to appear WITH their parent's draw-on, or
  they leak in before the shape exists.
- `NodeCard`/`BulletList` text is not gated by `drawIn`: for staged builds wrap them in
  `<Sequence from={...}>` or compose inline with delayed `SketchText`.
- Springs are front-loaded; use `interpolate` + easing for deliberate moves, springs for pops.
- Camera moves: ease every channel (`smoothstep`), never piecewise-linear zoom; zoom must be
  geometric (`z0*(z1/z0)^t`), not linear.
- Curling "loop" arrows need an arc path + head from the end tangent (`SketchLoopArrow`), not
  `SketchArrow`.
- `measureText`/text layout needs the font loaded (it is, via `src/theme/fonts.ts`).
- Long timelines: prefer a few strong beats over many rushed ones; a beat needs >= 2s to read.
