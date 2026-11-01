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

**Studio mode = that exact method (same look), with the cutroom engine as the speed helper.**
`draft` mode (the cutroom render) stays as a quick preview only.

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
  borumi/         Borumi glue (python): MCP client, duplicate project, canvas, base layouts,
                  apply cuts, chapter cards, badges, screen blurs, place render (behind/overlay)
                  with checks, export
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
Never write to the source project. Studio duplicates it to "<name> - editroom" and writes only
there, in checked transactions (begin -> dry-run check -> commit), last-to-first.

### Helpers the cutroom engine provides (workstream: helpers)
All read-only on Borumi, all fast:
1. `cutroom borumi-cuts <work>`: the final cuts (detector + editorial agent, already built) as
   Borumi trim ranges per scene in the DUPLICATE's project ms, ready for `trim_timeline`
   (edges snapped to quiet, 120 ms air, nothing under 300 ms). JSON.
2. `cutroom timing <work> --from <ms> --to <ms> [--fps 30]`: the kept words of a stretch with
   timeline ms and frame numbers in the slot's own clock (`at(ms)=round((ms-slot0)*fps/1000)`),
   so brief writers never measure. Plus `--json`.
3. `cutroom privacy` -> `privacy.json` already exists; add `cutroom borumi-blurs <work>`: the
   findings as Borumi screen_blur segments (timeline ms + normalized rect on the screen track).
4. `cutroom boxes <png> --phrase "..." [--all]`: exact glyph boxes of a phrase in a proof PNG
   (tesseract TSV, word-joined), normalized and px. Brief writers and designers use it instead of
   hand measuring.
5. `cutroom sheet-video <mp4|mov> --at t1,t2.. | --every N | --beats <job.json>`: frame contact
   sheets of a render (alpha shown over the dot grid), and `cutroom check-render <render>
   --slot <job.json>`: frame count, empty panel > 1 s, black dips, static holds > 6 s, alpha
   outside the panel (x > 0.775 or into the camera tile), beat lead vs word timings. JSON +
   markdown, so checkers only judge honesty, legibility and look.

### Placement (workstream: borumi)
`studio/borumi/place.py <work> <slot>`: behind jobs as a take on screen_2 with the panel layout;
overlays as the alpha .mov on screen_2 above screen_1; both under camera tile and badge. Dry-run
frame montage + PSNR vs render + tile/badge checks (tolerant of +-1 ms and dark shirts), then
commit. Every passing render gets placed (no T8b gaps).

### Orchestration (workstream: studio)
The skill drives Workflow scripts. Order, with parallelism:
1. duplicate -> canvas + base layouts -> cuts (cutroom agent, applied in Borumi), in parallel with
   proof research (claims from the transcript) and privacy (cutroom privacy -> blurs)
2. sections (from the chapters pass) -> chapter cards + badges
3. section briefs, all sections at once (writer -> critic -> revise), using `cutroom timing`
   and `cutroom boxes`
4. design, all sections pipelined (designer -> render via queue -> `check-render` + sheets ->
   checker -> numbered fixes -> revise, max 3 rounds), then one continuity pass
5. place every passing render, captions (.srt from words), audio (Auphonic master when
   configured), export from Borumi, final QA (ffmpeg detectors + 3 reviewers), report.
Model routing: briefs and design Opus; checkers Sonnet (medium); capture Sonnet; continuity Opus.
