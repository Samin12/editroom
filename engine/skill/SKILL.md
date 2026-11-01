---
name: cutroom
description: Edit a whole talking-head + screen-recording video (any length) with the cutroom engine - ingest a raw Borumi project or plain files, auto-cut retakes/fillers/silences, add chapters and motion graphics as word-anchored YAML beats, check privacy and layout, render in parallel. Use when asked to edit, cut, add visuals/motion graphics to, or render a Borumi recording or a camera+screen video.
---

# cutroom: the agentic edit loop

cutroom does every deterministic part (timing, cuts, layout, graphics, rendering, checks) in Rust.
You only make editorial decisions, written as a few lines of YAML anchored to WORDS. Never compute
milliseconds, never write animation code, never touch a timeline by hand.

```
cutroom ingest borumi "<bundle>.bmprojbundle" work/   # raw takes (read-only; --scenes 3-5 for a part), or: ingest files work/ --camera c.mp4 --screen s.mov
cutroom agent work/               # headless first pass, parallel (claude -p): editorial cuts (opus) + chapters + visuals per chapter + screen marks
cutroom proofs work/ [--drop-missing]   # proof screenshots still to capture (JSON); --drop-missing turns the rest into text beats
cutroom cut work/ [--accept 1,3,7-9]   # retakes, false starts, fillers, silences -> cuts.yaml (+ review.md)
cutroom privacy work/ [--allow me@x.com] [--pattern name=regex]   # OCR secrets (+ work/privacy-rules.yaml) -> beats/privacy.yaml
cutroom brief work/ --from w0 --to w2000    # kept transcript: "w123 one sentence per line"
# ... write chapters + beats (below) ...
cutroom check work/               # must print 0 errors
cutroom sheet work/ --beats -o /tmp/sheet.png   # look at it (one image = every beat)
cutroom render work/ -o final.mp4 # + final.chapters.txt + final.srt
```

Studio mode (films rendered elsewhere, one per section): `cutroom slots work/ --json` (exact
frames + the beat line per slot), `cutroom timing work/ --slot ch03` (words in the film's frame
clock), `cutroom boxes proof/x.png --phrase ".."` (highlight rects), `cutroom check-render film.mp4
--work work/ --slot ch03`, `cutroom sheet-video film.mp4 --every 2 -o s.png`. Details:
docs/STUDIO.md and docs/CANVAS.md.

## 1. Cuts
`cutroom cut` writes confident cuts to `cuts.yaml` and only the ambiguous ones to `review.md`
(each with context and word ids). Decide each item, then `cutroom cut work/ --accept 1,3,7-9`.
`cutroom agent` does all of it: detector cuts, its unsure candidates judged, plus an editorial pass
(retakes the detector could not pair, section restarts, repeats), each line `by: agent` in
cuts.yaml with a `note`; review.md then summarizes every cut with output timestamps.
Your own cuts: add `- {from: w812, to: w830, why: retake, by: human}` to cuts.yaml (inclusive word
range; the LAST attempt is the keeper). `cut` and `agent` regenerate their own cuts and keep
`by: human` ones (`cut` also keeps `by: agent`).
Never cut inside a word or leave a half sentence; `cutroom brief` after cutting shows what survives.

## 2. Chapters (edit.yaml)
```yaml
chapters:
  - {at: "the first thing you need", title: Find the idea, short: Idea}
```
Cards are inserted (1.8 s, 2 s max), the step badge follows automatically, and
`final.chapters.txt` is ready for the YouTube description. Add `group: The Easy Tips` to group
consecutive chapters: the badge then reads "TIPS 1-5 · 1 / 3" over the group title, and cards go on
group starts only. `output.card_style: rea` gives the white grid card (with `card_ms: 1500`).

## 3. Beats: the whole vocabulary
Every beat: `at:` (anchor), optional `until:` (anchor, ends after that word) or `hold: 3s`.
Anchors: `w123` (from brief), `"a phrase he says"` (`#2` = 2nd match), `s:754.2` (source s), `o:61.5` (output s), `+0.5s` offsets.

| kind | fields | use |
|---|---|---|
| title | text, kicker?, sub? | headline that builds word by word in the panel |
| bullets | items: [text or {text, at}], title? | steps/lists; items with `at` appear as he says them |
| number | value, label? | a stat he says ("$10,075", "4.76M"); counts up |
| image | src, caption?, highlight: [x,y,w,h]?, source? | PROOF: a real screenshot/page, push-in, highlight in image coords |
| video | src, from?, caption?, fit?, alpha?, loop? | footage / b-roll / a pre-rendered animation. `fit: card` (default, on a card, loops), `panel` (fills the panel slot), `frame` (full frame under the camera tile; with a transparent .mov it overlays the footage). Clips that end hold their last frame unless `loop: true` |
| code | text, title? | a prompt or command typing on |
| quote | text, by? | a quote / tweet line |
| scene | layers: [...], place: stage/frame?, bg: none/card/canvas? | a CUSTOM animation: text, boxes, circles, lines, arrows (attach by id), images (with `ring`), code, counters, highlights, each entering on its own word. See docs/SCENE.md |
| ring | rect | circle something on the screen recording |
| highlight | rect, underline? | marker on screen text |
| arrow | from: [x,y], to: [x,y] | point at something on screen |
| callout | text, pos: [x,y]? | short pill (pos = frame coords) |
| lower | title, sub? | lower-third |
| blur | rects: [[x,y,w,h]...] | hide private info on screen |
| zoom | rect | push into a region of the screen |
| layout | mode: panel/screen/camera | change layout for a stretch |

`*word*` in title/bullets/quote/scene text draws that word in the accent colour.
`rect`/`from`/`to` are SCREEN-normalized (0..1 of the screen recording) so they stay on target
through zooms and layout changes. Find them on a `cutroom still` of that moment.

## Custom animations (`scene`)
When no template fits (a diagram, a stat beside its proof, a timeline, a before/after), write a
`scene`: 3-9 layers in the panel (`place: stage`) or over the whole frame (`place: frame`, keep
x < 0.78 for the camera tile). Geometry is 0..1 of the box; sizes are units of a 1920-wide box.
Anchor each layer's `in: {at: ...}` to the word that introduces it.
```yaml
- at: "how Claude talks to Poppy"
  until: "in seconds."
  kind: scene
  layers:
    - {type: text, text: "How *Claude* talks to Poppy", x: 0.07, y: 0.08, size: 88, weight: bold}
    - {id: a, type: box, x: 0.06, y: 0.4, w: 0.25, h: 0.24, label: Claude, sub: your agent, in: {at: "Claude"}}
    - {id: b, type: box, x: 0.69, y: 0.4, w: 0.25, h: 0.24, label: Poppy, in: {at: "Poppy", from: pop}}
    - {type: arrow, from: a, to: b, in: {at: "talks"}}
    - {type: counter, to: 10075, prefix: "$", x: 0.5, y: 0.72, size: 120, in: {at: "ten thousand"}}
```
Full field list, timing rules and four copy-paste examples: docs/SCENE.md.

## Music and privacy rules
- Music bed: `output: {music: {src: music/bed.mp3, gain_db: -18, duck_db: 12}}` in edit.yaml. It
  loops, ducks under his voice and fades at the ends (`render --no-music` to preview without it).
- Project privacy: `work/privacy-rules.yaml` with `patterns: [{name, regex}]`, `allow`, `skip`
  (e.g. the community price `\$\s?\d+(?:\.\d{2})?\s*/\s*(?:mo|month)`). Run `privacy` again after
  changing it; custom findings are blurred and enforced like keys and emails.

## Rules that keep quality high
- Proof over animation: when he claims something, show the real page (`image` with `highlight`).
- One idea per beat, at least 2 s on screen; `check` warns below 1.5 s.
- Stage beats (title..quote) replace the screen in the panel: use them on talking-head stretches,
  not while he is demonstrating something on screen.
- No em dashes in on-screen text. Headlines under ~7 words.
- `check` must be clean before render: anchors on cut words, empty panels, overlapping beats,
  missing files and every unblurred privacy finding are errors or warnings there.

## Long videos: parallel agents
Give each agent one chapter: it runs `cutroom brief work/ --from wA --to wB` and writes
`work/beats/chNN.yaml` with `scope: [wA, wB]` at the top (phrase anchors then only match inside
its chapter). Files in `beats/` are merged automatically. Then one pass of `check` + `sheet`.
