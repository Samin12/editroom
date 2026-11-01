# editroom studio mode: the tips-edit method, made fast

Samin's verdict on the cutroom-rendered edit (v2 of "tips raw"): not good enough. The bar is
"tips - edited.mp4" (Drive, md5 e2143037…, = `~/Borumi Projects/tips - finished products/1 YouTube
long-form/10 Opus 5.5 tips - final.mp4`). It was made on Oct 15–16 in the Borumi project "tips"
(id 913b…) by the proof-first method:

- real proof for every claim (90 claims, 453 captures, verified)
- one long Remotion "section film" per section (intro, each tip, CTA), plus a few transparent
  overlays on screen demos. Velour style, designed from detailed beat-by-beat section briefs,
  checked by an independent checker, revised, then one continuity pass.
- Borumi does the rest: dot-grid canvas, base layouts, chapter cards, step badges, screen blurs,
  placement (behind the camera tile, or alpha overlays), export.

It took 13.6 h wall time, about 37 agent-hours and about $817. The slow parts were mechanical:
section briefs (946 agent-min, mostly measuring highlight boxes by hand), the design loop
(963 agent-min), research and proof run more than once, whisper run 3 times, placement patches,
and 5 h blocked on usage limits. It also left gaps: T8b never placed, no captions, the mastered
audio never placed, final QA never run.

**Studio mode = that exact method (same look), with the cutroom engine as the speed helper AND
the compositor.** Samin (Nov 1): Borumi does not have to be the editing surface; editing in our own
engine is fine if faster, then import the result into Borumi. So: cutroom cuts, composes and
renders; Borumi is read (ingest) and gets the finished video imported at the end. No timeline
writes, no placement patches. `draft` mode (cutroom templates only) stays as a quick preview.

### Composition (decided)
- Section films (one per section: intro, each tip, CTA) are 1920x1080 full-frame OPAQUE Remotion
  renders that include the dot-grid canvas and leave the camera tile and badge area clear. They go
  in as `video` beats `fit: frame` spanning the section's output time, frame-exact, no loop.
- Demo overlays (callouts on screen recordings) are 1920x1080 TRANSPARENT ProRes 4444 renders,
  `video` beats `fit: frame, alpha: true`, over the screen recording, under camera tile and badge.
- cutroom draws the camera tile, badge (with group labels like "TIPS 1-5 · 1 / 3 · The Easy
  Tips"), chapter cards, privacy blurs (blur beats apply over section films too when needed),
  audio (+ loudnorm), captions (.srt), chapters.txt.
- Cuts are final before briefs; slots are locked by a cuts hash, so a changed cut invalidates
  the renders it touches.
- End: `cutroom render` -> QA -> import the final mp4 into a new Borumi project
  "<name> - editroom" (Borumi MCP `import_media` + add to timeline), never touching the source.

## Repo layout (new)

```
studio/
  kit/            Remotion animation kit (vendored from ~/.borumi-agent/animation-kit: velour style,
                  proof.tsx with BrowserCard/Highlight/SourcePill/CodeChip/TweetCard/TipOpener,
                  components, theme) + references/ (the best jobs as designer references:
                  Honcho anim-mvagv60a hook, anim-mvagv66m proof cards; tips T1 anim-mvas6emb,
                  T4 anim-mvas6eqq, T6 anim-mvas6evb, T7b anim-mvas6eyk), NOTICE (OpenCutAgent).
  jobs/           job runner (job.mjs port) with a GLOBAL RENDER QUEUE (default 3 slots,
                  concurrency 8 each; behind = h264 crf 14 .mp4, overlay = ProRes 4444 .mov)
  borumi/         Borumi glue (python): MCP client, import the finished video (and optionally the
                  section films as media) into a new Borumi project
  workflows/      Workflow scripts: research-proof, section-briefs (write -> critic -> revise),
                  design (designer -> checker -> revise, pipelined per section), continuity,
                  final-qa
  templates/      designer-brief.md (generalized from tips), section-brief template
tools/tweetshot/  logged-out X post capture with phrase boxes
```

## Contracts between workstreams

### Work dir
`~/Borumi Projects/<name> editroom/` (same as draft mode). Studio files under `studio/`:
`studio/sections.json`, `studio/briefs/<slot>.md`, `studio/jobs/<slot>/` (Scene.tsx, job.json,
render outputs), `studio/proof/` (captures + proof-index.json), `studio/receipts/`.

### Borumi safety
Never write to the source project. The only Borumi write is creating/filling a NEW project
"<name> - editroom" with the finished video.

### Helpers the cutroom engine provides (workstream: helpers)
1. `cutroom slots <work>`: the sections (from chapters) as slots in OUTPUT time: id, title,
   start/end ms, frame count at 30 fps, first/last word ids, the screen/camera layout inside, and
   a cuts hash. `--json`. Designers render exactly `frames` frames.
2. `cutroom timing <work> --slot <id> | --from <anchor> --to <anchor>`: kept words with output
   ms and frame numbers in the slot's own clock (`f = round((ms - slot0) * 30 / 1000)`), plus
   which source (screen/camera) is on screen. `--json`. Brief writers never measure.
3. `cutroom boxes <png> --phrase "..." [--all]`: exact glyph boxes of a phrase in a proof PNG
   (tesseract TSV, word-joined), normalized and px.
4. `cutroom sheet-video <mp4|mov> --at t1,t2.. | --every N`: contact sheets of a render (alpha
   shown over the dot grid); `cutroom check-render <render> --slot <id> --work <work>`: frame count
   vs slot, empty panel > 1 s, black dips, static holds > 6 s, content inside the camera tile or
   badge rect, alpha coverage, beat lead vs word timings. JSON + markdown.
5. Frame-exact `video` beats for section films: `fit: frame` with `from` = 0 at the slot start,
   no loop, no fades when `fade: false`, decoder seek exact; verify a rendered film with frame
   numbers burned in lands frame-for-frame.
6. Badge groups: chapters can carry `group` ("The Easy Tips") and the badge shows
   "<KICKER> a-b · g / G" + group title like the tips edit (look at its frames); chapter card style
   `output.card_style: rea` (white grid card, black title) besides the current card.
7. Canvas phase: export the dot-grid drift formula so Remotion films can match cutroom's canvas
   exactly (same pitch, color, drift, phase origin = output time 0).

### Import (workstream: borumi)
`studio/borumi/import.py <work> [--films]`: create a new Borumi project "<name> - editroom"
(or the closest the MCP allows) and import the final mp4 onto its timeline; optionally the
section films as media. Find the exact MCP calls (`get_guides` first). Never touch other projects.

### Orchestration (workstream: studio)
The skill drives Workflow scripts. Order, with parallelism:
1. ingest -> cuts (cutroom agent: detector + editorial) + chapters/sections, in parallel with
   proof research (claims from the transcript) and privacy (cutroom privacy -> blur beats)
2. lock cuts -> `cutroom slots`
3. section briefs, all sections at once (writer -> critic -> revise), using `cutroom timing`
   and `cutroom boxes`
4. design, all sections pipelined (designer -> render via queue -> `check-render` + sheets ->
   checker -> numbered fixes -> revise, max 3 rounds), then one continuity pass
5. wire every passing render in as video beats, `cutroom render` (captions .srt, chapters,
   loudnorm), final QA (ffmpeg detectors + 3 reviewers watching sheets), import into Borumi,
   report.
Model routing: briefs and design Opus; checkers Sonnet (medium); capture Sonnet; continuity Opus.
