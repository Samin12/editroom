---
name: edit-video
description: "Edit a raw recording into a finished YouTube video, end to end and fast: cut retakes and dead air, chapters, real proof screenshots, motion graphics and custom animations, privacy blur, music, render and QA. Works on a Borumi project (by name) or camera/screen files. Use when Samin says: get this edited, edit this video, edit my Borumi project, cut and visualize, make this video, finish this recording, tighten and add visuals, edit <project name>."
---

# edit-video: raw recording in, finished video out

The cutroom engine (Rust, in this plugin's `engine/`) does every deterministic step: reading the
project, cutting, layout, graphics, privacy, rendering at ~15x realtime, checks. You make the
editorial calls and run the parallel helpers. Never hand-compute milliseconds, never write
animation code when a beat or `scene` will do, never touch the Borumi project (it is only read).

**Defaults: do the whole thing without stopping.** Ask Samin only when the project can't be
identified. Never upload, publish or post anything. Never render with `--force`.

## 0. Paths

- `ER` = this plugin's root: two folders up from this skill's base directory (`<ER>/skills/edit-video`).
- `C` = `~/.editroom/bin/cutroom` (built by `bash "$ER/scripts/install.sh"`; the pipeline rebuilds it
  when the engine source changed). Run `bash "$ER/scripts/install.sh" --check` if anything is missing.
- Work dir `W` (default): `~/Borumi Projects/<name> editroom/`. Everything the edit produces lives
  there; the final video is `W/out/<name> - editroom vN.mp4`.

## 1. Identify the input

- A Borumi project name: `python3 -I "$ER/scripts/find_project.py" "<name>"`. Take the top match
  when its score is 1.0 or clearly ahead; if two are close (e.g. "tips raw" vs "tips raw - sagecut"),
  pick the one Samin named exactly, else ask with the two paths.
- Raw takes are the default (cutroom cuts them). If Samin says the project is already cut in Borumi
  ("use what's on the timeline"), add `--timeline`. A part only: `--scenes 3-5`.
- Plain files: `--files --camera cam.mp4 --screen screen.mov [--mic mic.wav] --name "<name>"`.
- Music: `--music <file>` (looped under the video, ducked under his voice).
- Values that may stay visible on screen (his own email in a demo): `--allow samin@aianswer.us`.

## 2. Run the pipeline

```bash
bash "$ER/scripts/pipeline.sh" "<project>" [--music bed.mp3] [--timeline] [--scenes 3-5]
```

Stages: ingest → setup → analyze (privacy OCR **and** the editorial agent at the same time: cut
review, editorial cuts, chapters, visuals per chapter in parallel, screen pass) → proofs → drop →
check → sheet → render → qa → report. Long videos: run it in the background and wait for it; a
4-hour course takes a few minutes before the first gate. It is resumable: rerunning continues after
the last finished stage, `--from-stage <stage>` redoes one. Timings go to `W/editroom-log.json`.

The pipeline stops at three gates. Handle each, then resume exactly as shown.

### Gate 10: proofs pending

`W/proofs-pending.json` lists image beats the agent asked for (a claim plus where to find it).
Spawn **one `editroom:proof-capturer` subagent per request, all in a single message** (they run in
parallel; Sonnet). Give each: the request JSON, `W`, the target path (`src` from the request) and
`node "$ER/scripts/capture_proof.mjs"`. Each writes only the PNG at `src` and one
`W/proof/<slug>.source.json` (verdict, URL, capture date, highlight rect, any flag).
When all have returned:

```bash
bash "$ER/scripts/pipeline.sh" "<project>" --from-stage drop
```

`drop` applies each capture's highlight and source pill to its beat, then turns every request that
has no capture (not found, or the source does not support the claim) into a text beat or removes it.
More than ~25 requests: batch them 12 at a time. No network or a rush: `--no-proof` skips capture.

### Gate 11: check has errors

Read `W/logs/check.txt`. Fix the beat files in `W/beats/` (anchors on cut words, overlaps, a
missing file, an unblurred privacy finding: never delete a blur beat). `C brief W --from wA --to wB`
shows the kept words around a problem. Then `--from-stage check`. After 3 rounds, remove the
beats that still fail and list them in the report.

### Gate 12: review the contact sheets

`W/review/sheet-NN.png`: one frame per beat and chapter card, about 20 per sheet. Up to 3 sheets:
Read them yourself. More: spawn one `editroom:visual-reviewer` per sheet in one message (parallel, Sonnet).
Look for: text cut off or overlapping, a visual covering what he is showing, an empty or near-empty
panel, a ring/zoom off target, a headline that overstates his words, anything private, a price of
his paid community. Apply the numbered fixes to `W/beats/*.yaml` (a frame you are unsure of:
`C still W --at <anchor> -o /tmp/x.png` and look). Then:

```bash
bash "$ER/scripts/pipeline.sh" "<project>" --from-stage check --no-review   # if you changed beats
bash "$ER/scripts/pipeline.sh" "<project>" --from-stage render              # if nothing changed
```

At most 2 review rounds. Then render.

## 3. Make it better than the templates (custom animation)

Cutroom's templates (title, bullets, number, image, video, code, quote, ring, highlight, arrow,
callout, lower, blur, zoom, layout) cover most beats. For a moment that deserves more, use a
**`scene` beat**: a custom keyframed animation (text, boxes, arrows, counters, images, highlights)
drawn natively by the engine, frame-exact, ~2 ms per frame. Recipes: `references/scenes.md`.

- The agent already writes 1 to 3 scenes per chapter for hero moments. Add more when Samin asks
  ("animate the pipeline part") or when a concept runs > 20 s on the talking head with only a title.
- Several scenes: spawn one `editroom:scene-designer` per moment in one message (parallel). Each writes
  `W/beats/scene-<slug>.yaml` with a `scope:` and checks its own stills. Then `--from-stage check`.
- **Escape hatch (≤ 2 per video, only for a truly special moment cutroom can't draw, e.g. a 3D or
  physics look):** a Remotion job in `~/.borumi-agent/animation-kit` (its `GUIDE.md`), rendered
  **one at a time** (h264, or ProRes 4444 when it needs transparency), then dropped in as
  `{at: …, until: …, kind: video, src: clips/<name>.mov, fit: frame, alpha: true, loop: false}`.
  Never render several Remotion jobs at once: that is what pinned the Mac at 100% on Oct 29.

## 4. Look, sound, privacy (all on by default)

- Look: `style:` in `W/edit.yaml` (accent, canvas, ink, card, dot pitch). Samin's dark dot-grid,
  panel-left, camera-right look is the default. `references/looks-and-audio.md`.
- Sound: rendered with `--loudnorm` (-14 LUFS, true peak -1). Music bed via `--music`.
- Privacy: `cutroom privacy` always runs (keys, tokens, emails, phones, cards, plus
  `W/privacy-rules.yaml`: his community price, Skool about pages, order numbers, addresses). Every
  finding is blurred, and render refuses while one is visible. Third-party names, DMs and
  calendars in his footage: flag them in the report (his footage, his call).
- Creator voice and rules the agent follows: `W/style.md` (from `templates/style.md`). Edit it per
  video if Samin gives direction ("keep it loose", "more visuals in part 2").

## 5. The rules that keep quality high

Condensed in `references/editorial-rules.md`; read it before fixing cuts or beats by hand. The core:
- Keep the last complete take of a restarted line; cut every other attempt. A restart re-says the
  same words; a next point moves forward. Be decisive: 25 to 35% of raw takes usually goes.
- Proof over animation. Never show a number, name or claim the source does not support. When he
  overstates, show the true source line and flag it. Label strength matches his words.
- Stage visuals on talking-head stretches; overlays (ring, zoom, highlight, arrow) while he is
  demonstrating. One idea at a time, ≥ 2 s each, nothing static > ~4 s, headlines < 7 words, no em dashes.
- Chapters: 2 to 5 words, Title Case, "Why This Matters" first after the intro, no "Level N" on cards.

## 6. Report to Samin

When the pipeline finishes, read `W/report.md` and `W/qa.md`. Fix any QA **blocker/major** (a
re-render takes minutes: `--from-stage render`). Then reply, short and speakable:

- The video path (and that the Borumi project is untouched), raw length → final length, render time.
- The YouTube chapters block (from `W/out/<video>.chapters.txt`), ready to paste.
- Flags: accuracy (claims the proof did not support), privacy (third parties in his footage), legal
  (third-party clips or music), each with a timestamp and a suggested fix. Claims are never
  silently changed.
- What's next: only his actions (watch it, decide on flags, upload).

Open the video for him (`open "<video>"`) unless he said not to.

## References

| File | Read when |
|---|---|
| `references/editorial-rules.md` | fixing cuts or beats by hand; judging retakes; chapter and card rules |
| `references/proof.md` | what counts as proof, honesty rules, capture recipes (pages, X posts, GitHub numbers) |
| `references/scenes.md` | writing a `scene` beat (custom animation) |
| `references/qa.md` | QA findings and fixes, the final watch-through |
| `references/looks-and-audio.md` | theme, layout, music bed, loudness, captions |
| `references/troubleshooting.md` | a stage failed, the engine is old, a gate loops |
