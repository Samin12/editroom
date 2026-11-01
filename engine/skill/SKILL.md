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
cutroom agent work/               # headless first pass: cut review + chapters + visuals per chapter (claude -p, parallel)
cutroom cut work/ [--accept 1,3,7-9]   # retakes, false starts, fillers, silences -> cuts.yaml (+ review.md)
cutroom privacy work/ [--allow me@x.com]   # OCR secrets on screen -> beats/privacy.yaml (blur beats)
cutroom brief work/ --from w0 --to w2000    # kept transcript: "w123 one sentence per line"
# ... write chapters + beats (below) ...
cutroom check work/               # must print 0 errors
cutroom sheet work/ --beats -o /tmp/sheet.png   # look at it (one image = every beat)
cutroom render work/ -o final.mp4 # + final.chapters.txt + final.srt
```

## 1. Cuts
`cutroom cut` writes confident cuts to `cuts.yaml` and only the ambiguous ones to `review.md`
(each with context and word ids). Decide each item, then `cutroom cut work/ --accept 1,3,7-9`.
Other cuts go in `edit.yaml` under `cuts:` (cuts.yaml is regenerated):
`- {from: w812, to: w830, why: retake}` (inclusive word range; the LAST attempt is the keeper).
Never cut inside a word or leave a half sentence; `cutroom brief` after cutting shows what survives.

## 2. Chapters (edit.yaml)
```yaml
chapters:
  - {at: "the first thing you need", title: Find the idea, short: Idea}
```
Cards are inserted (1.8 s, 2 s max), the step badge follows automatically, and
`final.chapters.txt` is ready for the YouTube description.

## 3. Beats: the whole vocabulary
Every beat: `at:` (anchor), optional `until:` (anchor, ends after that word) or `hold: 3s`.
Anchors: `w123` (from brief), `"a phrase he says"` (`#2` = 2nd match), `s:754.2` (source s), `o:61.5` (output s), `+0.5s` offsets.

| kind | fields | use |
|---|---|---|
| title | text, kicker?, sub? | headline that builds word by word in the panel |
| bullets | items: [text or {text, at}], title? | steps/lists; items with `at` appear as he says them |
| number | value, label? | a stat he says ("$10,075", "4.76M"); counts up |
| image | src, caption?, highlight: [x,y,w,h]?, source? | PROOF: a real screenshot/page, push-in, highlight in image coords |
| video | src, from?, caption? | real footage / b-roll in the panel |
| code | text, title? | a prompt or command typing on |
| quote | text, by? | a quote / tweet line |
| ring | rect | circle something on the screen recording |
| highlight | rect, underline? | marker on screen text |
| arrow | from: [x,y], to: [x,y] | point at something on screen |
| callout | text, pos: [x,y]? | short pill (pos = frame coords) |
| lower | title, sub? | lower-third |
| blur | rects: [[x,y,w,h]...] | hide private info on screen |
| zoom | rect | push into a region of the screen |
| layout | mode: panel/screen/camera | change layout for a stretch |

`*word*` in title/bullets/quote text draws that word in the accent colour.
`rect`/`from`/`to` are SCREEN-normalized (0..1 of the screen recording) so they stay on target
through zooms and layout changes. Find them on a `cutroom still` of that moment.

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
