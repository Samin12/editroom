<!-- Section brief template (from the T1 brief of "10 Opus 5.5 tips", the best one of that edit). A brief writer
fills one per slot into <W>/studio/briefs/<KEY>.md. The designer follows it beat by beat; the checker checks the
render against it. Keep every number exact: they come from tools, not from looking. Delete these comments. -->

<SECTION NAME> (<m:ss.s> to <m:ss.s>), <BEHIND|FRONT>, KEY <KEY>, SLOT0 <slot0 output ms>, frames <N> (slot <a>-<b> output ms).

<One short paragraph: what is on screen in his footage during this slot (from the timing file's `source` column and
a still), and what this film does about it: replaces a static screen with a full film (BEHIND), or adds callouts
over a live demo (FRONT). For FRONT: the recording's source size and the source px -> final px mapping.>

How to read this brief:
- Times are OUTPUT seconds from `<W>/studio/timing/<KEY>.json`.
- Frames are job frames, `at(ms) = round((ms - <SLOT0>) * 30 / 1000) - 3`, so each one lands 3 frames before its word.
  Footage seams and clicks are raw frames with no lead, because they sync to footage, not to a word.
- Proof paths are relative to `<W>/studio/proof/`. Extra crops made for this slot are in
  `<W>/studio/briefs/<KEY>-assets/` (say how each was cut and that it is pixel-identical to its source).

BOX RULES (read before placing any highlight):
- Every box is [x0,y0,x1,y1] on the drawn glyphs of the PNG named in that beat, from `studio.py boxes` (or `cutroom
  boxes`). Multi-line phrases are listed as one segment per line.
- TweetCard `highlights` take SOURCE px directly. BrowserCard children use image-area px: give those already scaled
  for the width stated.
- Use the CLEAN PNGs (`*-crop.png`, `*.png`) with your own `Highlight`, never baked `-highlight` PNGs (two boxes).
- ONE live mark per card: every highlight gets an `out`, so the old mark leaves as the next one lands.

SHARED LOOK (match <neighbouring keys>):
- <card geometry shared with neighbours: e.g. tweet cards at scale 0.85, w 1292, x 314, viewport 900 stage px>
- <the row under each card: label chip left, SourcePill right>
- <locked labels for people and products in this slot>
- The opener is the shared `TipOpener` (kicker + name >= 96 px), then a compact header docked top-left.

MOTION NOTES:
- <component behaviours the designer must work around, e.g. built-in push running to the end of the job; Reveal's
  default 18-frame entrance; exit durations; MediaFrame push={[1,1]} on real footage>

- <out s> (f<frame>) "<his exact words>": <WHAT SHOWS, in caps first>. <proof file + its pixel size>. <geometry: x, y, w
  (scale), view/crop>. <entrance and motion>. <row under it: pill/label>.
  <out s> (f<frame>) "<word>": <highlight kind> on <phrase>, glyph box [x0,y0,x1,y1] (two segments if it wraps). `out` f<n>.
  HONEST NOTE (<proof id>, <status>): <what the screen may and may not say>. See HONEST NOTES <n>.

- <next beat ... one bullet per beat in narration order; beats every 2.5 to 6 s; never static > ~4 s; every claim he
  makes in the slot has a real proof where one exists; concepts get a simple diagram built on his words>

- <out s> (f<frame>) "<closing words>": THE CLOSE. <how the film hands off to the next section or card: the last
  frames hold calm; nothing new in the final ~10 frames>

HONEST NOTES
1. <proof id> (<✓|~|✗|–>). He says "<...>", the source says "<...>". Pack honest line: "<...>". What the screen shows.

DO NOT SHOW:
- <every do_not_show from the pack entries used here; private regions of his footage with their source px boxes;
  wording that would overstate; baked highlight PNGs; logged-out overlays>

OPEN ISSUES:
- <anything you could not source or measure, mismatched file sizes, assumptions the designer must verify on a still>
