---
name: edit-video
description: "Edit a raw recording into a finished YouTube video, end to end and fast: cut retakes and dead air, chapters, real proof screenshots, motion graphics and custom animations, privacy blur, music, render and QA. Works on a Borumi project (by name) or camera/screen files. Use when Samin says: get this edited, edit this video, edit my Borumi project, cut and visualize, make this video, finish this recording, tighten and add visuals, edit <project name>."
---

# edit-video: raw recording in, finished video out

The cutroom engine (Rust, in this plugin's `engine/`) does every deterministic step: reading the
project, cutting, layout, graphics, privacy, rendering at ~15x realtime, checks. You make the
editorial calls and run the parallel helpers. Never hand-compute milliseconds, never write
animation code when a beat or `scene` will do, never touch the Borumi project (it is only read).

**Do the whole thing without stopping.** Every gate below is yours to handle; ask Samin only when
the project can't be identified. Never upload, publish or post anything. Never render with `--force`.

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
- Music: `--music <file>` (looped under the video, 12 dB quieter while he talks; `references/looks-and-audio.md`).
- Values that may stay visible on screen (his own email in a demo): `--allow samin@aianswer.us`.

## 2. Run the pipeline, and handle every gate yourself

```bash
bash "$ER/scripts/pipeline.sh" "<project>" [--music bed.mp3] [--timeline] [--scenes 3-5] [--allow x]
```

Its first line is `editroom: <name> -> <W>`: remember `W`. Every later run is just
`bash "$ER/scripts/pipeline.sh" --work "$W"` (it resumes in the right place) or with `--from-stage <stage>`.

Stages: ingest → setup → analyze (privacy OCR **and** the editorial agent at once: editorial cuts,
chapters, visuals per chapter, screen marks, all in parallel) → proofs → drop → check → sheet →
render → qa → report. A 4-hour course reaches the first gate in a few minutes; run long videos in
the background and wait for them. Timings go to `W/editroom-log.json`.

**Loop until it exits 0. Never stop to ask Samin at a gate.** Each gate allows 2 rounds; the
pipeline counts them (`round N` in the message) and moves on by itself after that.

| exit | meaning | what you do, automatically |
|---|---|---|
| 0 | finished | section 6 |
| 10 | proofs pending | **Gate 10** below, then `--work "$W"` (continues at drop) |
| 11 | check errors | **Gate 11** below, then `--work "$W"` (continues at check) |
| 12 | contact sheets ready | **Gate 12** below, then `--work "$W" --from-stage check` if you changed beats, else `--work "$W"` (continues at render) |
| 1 | a stage failed | read the `FAILED <stage>` line and `W/logs/<stage>.log`, fix per `references/troubleshooting.md`, rerun `--from-stage <stage>`. Failing twice the same way: stop and tell Samin what broke (an error, not a question) |

### Gate 10: proofs pending

`W/proofs-pending.json` lists image beats waiting for proof: `claim`, `find`, `src` (where the PNG
goes), `line` (talk or screen), `file` + `index` (the beat), `out` (its time). Spawn **one
`editroom:proof-capturer` per request, all in a single message** (parallel, Sonnet; batches of 12
when there are more than ~25). Give each: the request JSON, `W`, and
`node "$ER/scripts/capture_proof.mjs"`. Each writes only the PNG at `src`, its sidecar, and
`W/proof/<slug>.source.json` (verdict, URL, highlight, source pill, optional corrected `caption`, flag).

When all have returned, rerun. Round 2 (only when a capture failed for a transient reason: a
timeout, a crashed browser) is `--from-stage proofs`, which re-lists what is still missing. After
round 2 the pipeline drops the rest by itself: `drop` writes each capture's highlight, source pill
and corrected caption onto its beat, then turns every request still without a PNG into a text beat
(title, quote or callout with its claim) or removes it.

### Gate 11: check has errors

Read `W/logs/check.txt`. Each error names a beats file and the beat (its anchor or time and kind).
- Rounds 1 and 2: fix them in `W/beats/*.yaml`: an anchor on a cut or unknown phrase → the nearest
  kept word id (`C brief W --from wA --to wB` shows the kept words); overlapping stage beats → end the
  first one earlier (`until`) or drop the weaker one; a scene layer missing a field → add it
  (`references/scenes.md`); a missing file → remove the beat. Never delete or weaken a blur beat:
  an unblurred privacy finding gets a blur beat, not a `--force`.
- Round 3 and later (the message says so): **remove** every beat an error names. Move its line into
  `W/agent/removed-by-check.yaml` so the report lists it, then rerun.

### Gate 12: review the contact sheets

`W/review/sheet-NN.png` (about 20 frames each, 4 per row) with `sheet-NN.md` listing every frame's
time, kind, text and beats file. Up to 3 sheets: Read them yourself. More: spawn one
`editroom:visual-reviewer` per sheet in one message (parallel, Sonnet) and collect their numbered fixes.
Look for: text cut off or overlapping, a stage visual covering what he is demonstrating, an empty
panel, a ring or zoom off target, a headline stronger than his words, anything private, his
community's price, a misspelled name. When a thumbnail is too small to judge:
`C still W --at o:<seconds> -o /tmp/x.png`. Apply every **blocking** fix and the clear minor ones
to `W/beats/*.yaml`, then rerun with `--from-stage check` (changed) or plain (nothing to change).
After 2 rounds the pipeline renders without stopping.

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
- Privacy: `cutroom privacy` always runs (keys, tokens, passwords, emails, phones, cards, plus
  `W/privacy-rules.yaml`: `patterns` for his community price, Skool about pages, order numbers,
  addresses; `allow` for values that may show; `skip` for built-in kinds to leave out). Every
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

When the pipeline exits 0, read `W/report.md` and `W/qa.md`. Fix any QA **blocker/major** first
(`--work "$W" --from-stage render`; a re-render takes minutes). Then reply, short and speakable:

- The video path (the Borumi project is untouched), raw length → final length, render time.
- The YouTube chapters block, ready to paste (it is in the report).
- **Cuts he can undo:** `W/review.md` lists every cut the agent made, with its time in the video and
  in the raw recording, the words removed and why. The agent can occasionally cut something he meant
  to keep (an instruction, an aside). To restore one: delete that line (`by: agent`) from
  `W/cuts.yaml` and rerun with `--from-stage check` (not `analyze`, which regenerates the agent's
  cuts). There is no keep list; changing `by:` does not restore the words.
- On-screen corrections: names the transcript misspelled that the captions now spell from the source.
- Flags: accuracy (claims the proof did not support), privacy (third parties in his footage), legal
  (third-party clips or music), each with a timestamp and a suggested fix. Claims are never
  silently changed. Beats removed at the check gate, if any.
- What's next: only his actions (watch it, decide on flags, upload).

Open the video for him (`open "<video>"`) unless he said not to. Never upload or publish it.

## References

| File | Read when |
|---|---|
| `references/editorial-rules.md` | fixing cuts or beats by hand; judging retakes; chapter and card rules |
| `references/proof.md` | what counts as proof, honesty rules, capture recipes (pages, X posts, GitHub numbers) |
| `references/scenes.md` | writing a `scene` beat (custom animation) |
| `references/qa.md` | QA findings and fixes, the final watch-through |
| `references/looks-and-audio.md` | theme, layout, music bed, loudness, captions |
| `references/troubleshooting.md` | a stage failed, the engine is old, a gate loops |
