# Animation styles: self-contained packages

A **style** is everything that defines one visual language for the animation designer: a design
guide it follows, plus (optionally) components and a theme built for that look. Each style is
ONE folder in here. Adding a style = dropping a folder in, removing it = deleting the folder.
Nothing else registers it: `kit.mjs styles` discovers styles by scanning this directory, and
`/borumi:visualize --style <id>` validates the id against that list.

## Package layout

```
styles/<id>/
  style.json    REQUIRED  identity: { "id", "name", "description", "default"?, "audio"? }
  SKILL.md      REQUIRED  the designer's guide + a "Learnings log" section it appends to
  src/          optional  style-specific React components / theme (TypeScript, compiled with
                          the kit; import the engine via ../../../src/...)
```

- `style.json`: `id` must equal the folder name; `name`/`description` feed `kit.mjs styles` and
  the command's help text; at most one style ships `"default": true`. `"audio": true` declares
  that the style's scenes make their own sound: renders are MUTED and stripped of audio unless a
  style asks for it (the flag becomes `job.audio` at creation, and `job.mjs render` reads it).
  No shipped style declares it.
- `SKILL.md`: the designer reads it at the start of every design turn for the chosen style, so
  keep it focused: the aesthetic, the non-negotiables, the components to use, look-and-feel
  defaults. End with a **Learnings log** section: the designer appends user-taught rules there,
  and the runtime workspace copy of this file is NEVER overwritten by kit updates (the log
  survives). Bump the `<!-- guide-version: N -->` comment at the top when you change the shipped
  text, or the workspace copy keeps the old wording.
- `src/`: export everything from `src/index.ts`. Job scenes import it as
  `../../../styles/<id>/src`. Build on the shared engine (`src/sketch`, `src/theme`,
  `src/animation`, `src/components`) instead of duplicating it; a theme overlay that spreads the
  engine `tokens` and overrides colors (see `n8n/src/theme.ts`) keeps the package tiny. Style
  code is type-checked with the kit (`job.mjs typecheck`), so a style must compile.

Keep packages code-only: no npm dependencies of their own (the kit's package.json is shared) and
no static assets that need `public/`; inline SVG paths instead (see `n8n/src/n8nLogoPaths.ts`).
A style may ship its own fonts, but only INLINED as data: URIs in `src/fontdata/*.ts` (add the
`.woff2` under `styles/<id>/fonts/` and an entry to `scripts/inline-fonts.mjs`), loaded through
the engine's `loadInlineFont`. A render must never make a font request.

## Custom styles (user-made)

Drop a package folder into the runtime workspace's `styles/` directory
(`~/.borumi-agent/animation-kit/styles/<id>/` by default; `BORUMI_AGENT_HOME` moves the
workspace) and `kit.mjs styles` lists it next to the shipped styles. Kit updates never touch
folders they did not ship. A shipped style with the same id wins over a workspace copy, so give a
custom style its own id.

## Shipped styles

Three styles ship with the plugin. Upstream OpenCutAgent also carried `n8n-brand`, `n8n-game` and
`n8n-ui` packages; they are not in this repository. If you have them, they install as custom
styles in the workspace.

- `excalidraw/`: hand-drawn dark whiteboard, violet accent (the default). Skill-only package:
  it styles the shared engine components directly.
- `n8n/` ("n8n sketch"): the same hand-drawn engine in n8n's older brand look: pink `#EA4B71`
  accent, sketchy square workflow nodes + connectors with data pulses, the AI-builder chat. Ships
  components under `src/`. Note: the n8n name, logo geometry (`n8nLogoPaths.ts`), and brand
  palette belong to n8n; the designer only renders the logo when explicitly asked.
- `leo/` ("Leo (pixel presenter)"): a 16-bit pixel-art likeness of the upstream author (square
  jaw, black curls, thin wide frames, stubble, black tee; a `hair="cap"` variant) who talks along
  with the narration. NOT hand-drawn. The 56x64 sprite is authored as a char grid in
  `src/sprite.ts` (one letter per pixel, `src/palette.ts` is the only place a colour lives) and
  rendered as merged SVG rects at an integer scale, so it is crisp at any canvas size with zero
  assets. Lip sync is word-timed (`src/lipsync.ts`: a viseme per syllable from its vowel, landed
  on each word's onset; `job.mjs create` writes every job's `words.json` for it), idle life is
  seeded (`src/motion.ts`: blinks, breath, talking bob, glances). Components: `Leo`, `LeoCorner`
  (stream cam placement, the default), `PixelPanel`. A personal likeness, kept as the worked
  example of a sprite style; swap the char grid to make it someone else.
