---
name: edit-video
description: "Edit a raw recording into a finished YouTube video at the quality of Samin's best edit (the \"10 Opus 5.5 tips\" video): real proof for every claim, one designed motion-graphics film per section, independent checks, privacy blur, chapters, captions, render, QA, imported into Borumi. Works on a Borumi project (by name) or camera/screen files. Use when Samin says: get this edited, edit this video, edit my Borumi project, cut and visualize, make this video, finish this recording, edit <project name>. A quick draft only when he asks for one."
---

# edit-video: raw recording in, finished video out

## Why this method
Samin's bar is "tips - edited.mp4" (Oct 16): real tweets and pages as proof with the exact words highlighted,
"WHY IT WORKS" / "THE TIP" kickers, checklists with icons, UI mockups, right-click callouts on his demos, the
"TIPS 1-5 · 1/3 The Easy Tips" badge. It came from: a verified proof pack (90 claims), a beat-by-beat brief per
section, ONE Remotion film per section built by a designer and passed by an independent checker, and a
continuity pass. It took 13.6 h because of mechanical work (measuring boxes by hand, re-runs, placement in
Borumi). Studio mode keeps that method and lets the cutroom engine do the mechanical parts: cuts, exact word
frames, glyph boxes, privacy, mechanical render checks, composition and a 15x-realtime render. A cutroom-only
edit (draft mode) is fast but was judged not good enough: use it only when he asks for a quick preview
(`references/draft-mode.md`).

**Hands-off.** Run everything below without stopping; ask Samin only when the project can't be identified.
Never upload, publish or post anything; never render with `--force`; never write to his Borumi project (only a
NEW project gets the finished video).

## 0. Paths and the timing log
- `ER` = this plugin's root (two folders up from this skill's base directory). `C` = `~/.editroom/bin/cutroom`
  (`bash "$ER/scripts/install.sh" --check`; the pipeline rebuilds it when `engine/` changed).
  `S` = `python3 "$ER/scripts/studio.py"`. Work dir `W` = `~/Borumi Projects/<name> editroom/`.
- Workflows live in `$ER/studio/workflows/`. Run each with the Workflow tool:
  `Workflow({scriptPath: "$ER/studio/workflows/<name>.js", args: <JSON printed by S args ...>})` (pass the
  parsed JSON object, not a string). Workflows run in the background; you get a notification when one ends.
- Log every stage: `S log "$W" <stage> start` before, `S log "$W" <stage> end` after. Remotion renders only go
  through `node "$ER/studio/jobs/job.mjs"` (its global queue renders a few at a time; never bypass it).

## 1. Identify the input and ingest
- Borumi project: `python3 -I "$ER/scripts/find_project.py" "<name>"`; take the top match when its score is 1.0
  or clearly ahead; if two are close, pick the one he named exactly, else ask with both paths.
- `bash "$ER/scripts/pipeline.sh" "<project>" --until setup` (add `--timeline` if he says the Borumi timeline is
  already cut, `--scenes 3-5` for a part, `--files --camera c.mp4 --screen s.mov --name "<name>"` for plain files,
  `--allow <value>` for things that may stay visible). The first line prints `W`.
- `S init "$W" --title "<video title>" [--direction "<his words about the look, if he gave any>"]`.

## 2. Cuts, privacy and proof, all at once
Start both, then wait for both:
- **Cuts + privacy** (Bash, background): `bash "$ER/scripts/pipeline.sh" --work "$W" --from-stage analyze --until analyze --agent-args=--cuts-only`.
  It runs the privacy OCR and the editorial cut agent (detector + Opus retake judge) in parallel (about a minute
  for 20 minutes of footage). `W/review.md` lists every cut.
- **Proof pack** (Workflow): `S args research "$W"` -> `research-proof.js`. A planner groups his claims, one
  capture agent + one independent verifier per group run in parallel (Sonnet), a completeness critic fills gaps,
  an assembler writes `W/studio/proof/proof-index.json`. It starts from the raw transcript, so it doesn't wait
  for the cuts. Rerunning skips verified groups.

## 3. Sections (needs the cuts)
`S args plan "$W"` -> `plan-sections.js`: the section planner writes chapters (kicker, badge groups, rea cards)
and slots (BEHIND film per talking stretch, FRONT overlay per live screen demo), runs `S resolve` (chapters into
`beats/00-chapters.yaml`, slots snapped to output frames, a timing file and a job folder per slot, the cut lock),
and an independent critic checks it. Then `S status "$W"`. From here the cuts are LOCKED: if anything changes
cuts.yaml or the chapters later, run `S verify "$W"` and `S resolve "$W"`; stale briefs and renders are redone.

## 4. Briefs and design, pipelined per slot (needs the proof pack)
`S args briefs "$W" --chain-design` -> `section-briefs.js`. For every slot at once: an Opus writer builds the
beat-by-beat brief (exact frames from the timing file, glyph boxes from `S boxes`, stills of his footage), an
independent critic checks it, a reviser applies the fixes; the moment a slot's brief is final, its design loop
starts: an Opus designer builds the Remotion film in the velour kit, renders through the queue, then a Sonnet
checker runs `S check-render` + `S sheet-video` and the 12-point check, and numbered fixes go back to the
designer (at most 3 rounds). Slots that `depends_on` another slot are not chained: after the workflow ends run
`S args design "$W"` -> `design.js` (it designs whatever has a final brief and no passing render).

When it ends: `S status "$W"`. For slots still failing, run `S args design "$W" --only K1,K2` -> `design.js` once
more. A slot that still fails after that keeps his plain footage there; say so in the report.

## 5. Continuity
`S args continuity "$W"` -> `continuity.js`: one Opus reviewer looks at every render in order, every join and
every chapter-card boundary; slots with blocking issues go back through `design.js` with numbered fixes.

## 6. Wire, render, check
- `S wire "$W"`: every passing film becomes a frame-exact `video` beat (`fit: frame`; FRONT overlays with alpha)
  in `W/beats/studio-films.yaml`; it refuses renders made for an older slot. Exit 2 lists missing or failing
  slots (go back to step 4's `--only` once, then accept the gap).
- `bash "$ER/scripts/pipeline.sh" --work "$W" --from-stage check --no-review`: check, render (`--loudnorm`, about
  15x realtime), the ffmpeg QA (`W/qa.md`) and the report. Exit 11 = check errors: read `W/logs/check.txt`, fix
  (a blur stays, a film beat with a wrong file gets re-wired), rerun with `--from-stage check`.

## 7. Final QA, Borumi, report
- Privacy sweep (optional, needs a Jev or Sage key): `S jev-privacy "$W"` on the render. Look at
  `W/studio/receipts/privacy-sweep/candidates.jpg`; copy each confirmed blur into `W/beats/review-privacy.yaml`
  (re-anchor to `s:`), then re-render. It adds to the OCR regex; never skip the reviewers because of it.
- `S args final-qa "$W"` -> `final-qa.js`: three read-only reviewers (cards and joins, narrative flow, a full
  watch-through against the proof pack and privacy rules). Fix every blocker and major through its owner:
  `design --only <KEY>` then `S wire`, a cut line in `cuts.yaml`, a blur beat, the chapters; then re-render with
  `pipeline.sh --work "$W" --from-stage render`. Minors go in the report.
- Borumi: `python3 "$ER/studio/borumi/import.py" "$W"` creates a NEW project "<name> - editroom" with the finished
  video (his source project is never touched). If that script is not installed yet, skip it and say so.
- `S report "$W"` (slots, renders, checks, continuity, final QA, stage times) plus `W/report.md` and `W/qa.md`.

Reply to Samin, short and speakable:
- The video path, raw length -> final length, wall time and where it went (from the stage times).
- The YouTube chapters block, ready to paste.
- Cuts he can undo: `W/review.md` lists every cut with its time and why; to restore one, delete its line in
  `W/cuts.yaml`, then `S verify`/`S resolve` and redo the stale slots (tell him which those would be).
- Flags: accuracy (claims the proof did not support, with the honest line), names corrected from the
  transcript, privacy (third parties in his footage), legal (third-party clips or music), slots without a film.
- What's next: only his actions (watch it, decide on flags, upload). Open the video (`open "<video>"`).

## Rules that keep the quality (the designer brief has the full list)
- Proof first: a claim gets the real page or post, with ONE highlight on the exact words he says, and a source
  pill. A concept gets a simple diagram that builds on his words. Nothing static over ~4 s, nothing empty.
- Honesty: never a number, name or quote the source doesn't support; when he overstates, show the true line and
  flag it. Correct spellings of names on screen, never the transcript's.
- Privacy: no third-party names, chats, calendars, emails; never his paid community's price; every finding
  blurred (render refuses otherwise).
- Cuts: keep the last complete take of a restarted line; be decisive (25 to 35% of raw usually goes).

## Failure handling
- A workflow returns `null` for an agent (it died): rerun the same `S args ...` + workflow; finished slots and
  verified groups are skipped (state is on disk: briefs, `design-report.json`, `checks/check-v<N>.json`).
- `S status` shows a rendered version with check `-` although the workflow reported it passed: the checker returned its
  verdict without writing `checks/check-v<N>.json`. Write that file from the workflow's journal result (the checker's
  JSON plus `key` and `version`) instead of re-running the slot.
- A stage fails twice the same way: stop and tell Samin what broke (an error, not a question).
- `references/troubleshooting.md` covers the engine, the pipeline and gates; `references/studio.md` covers the
  studio files and commands in detail.

## References
| File | Read when |
|---|---|
| `references/studio.md` | the studio work dir layout, every `studio.py` command, resuming a stage |
| `references/editorial-rules.md` | judging retakes, chapter and card rules |
| `references/proof.md` | what counts as proof, honesty rules, capture recipes |
| `references/qa.md` | QA findings and fixes |
| `references/looks-and-audio.md` | theme, layout, music bed, loudness, captions |
| `references/draft-mode.md` | Samin asks for a quick draft (cutroom templates only) |
| `references/scenes.md` | draft mode's native `scene` animations |
| `references/troubleshooting.md` | a stage failed, the engine is old, a gate loops |
