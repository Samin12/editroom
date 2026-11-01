# Studio mode: files, commands, resuming

> **The rule (Samin, Nov 1): never compromise how the edits look and feel; optimize speed and cost.**
> Decision models (TypeSafe Jev, Levanto Sage) only pre-screen, rank and suggest. They never remove a check:
> the checker's full pass, the proof verifiers and the Opus editorial pass all stay. A model replaces a judgment
> only if it matches or beats it on labelled data with recall at least as high, reported as a proposal, never as a
> default. Unsure answers (null, low confidence, an error) always go to the agent; nothing is auto-passed.
> Measurements: `tools/jev/BENCHMARKS.md`.

## Work dir (W = `~/Borumi Projects/<name> editroom/`)
| Path | Written by | What |
|---|---|---|
| `source.json`, `edit.yaml`, `cuts.yaml`, `review.md` | pipeline (ingest, analyze `--cuts-only`) | the recording, the edit spec, the cuts and why |
| `beats/privacy.yaml`, `privacy.json` | pipeline (analyze) | blur beats for secrets and `privacy-rules.yaml` patterns |
| `beats/00-chapters.yaml` | `studio.py resolve` | chapters, `output: {kicker, intro, card_style}`, badge groups |
| `beats/studio-films.yaml` | `studio.py wire` | one `video` beat per passing film/overlay (frame-exact) |
| `beats-draft/<stamp>/` | `studio.py init` | a draft run's visual beats, moved aside (never deleted) |
| `studio/state.json` | studio.py | title, creator, today, the cut lock (map hash) |
| `studio/designer-brief.md` | `init` + the section planner | the law for every film (from `studio/templates/designer-brief.md`) |
| `studio/proof/` | research-proof.js | `groups.json`, `NN-<group>/` (captures, `sources.json`, `VERIFIED.json`), `proof-index.json`, `README.md` |
| `studio/sections-plan.json` | the section planner | chapters + slots in word ids, modes, groups, the "This video" block |
| `studio/sections.json` | `studio.py resolve` | slots in OUTPUT time: slot0, end, frames, clock, paths, `sig` |
| `studio/timing/<KEY>.json` | `studio.py resolve` | every kept word: output ms, frame `f` in the slot's clock, on-screen source; sentences; screen-media mapping |
| `studio/briefs/<KEY>.md` | brief writer | the beat-by-beat brief (+ `.critic.json`, `.revised.json`, `.sig`, `<KEY>-assets/`) |
| `studio/jobs/<KEY>/` | resolve + designer | `job.json` (slot0, frames, mode, codec), `Scene.tsx`, renders, `design-report.json`, `checks/` (`mech-v<N>.json`, `check-v<N>.json`, sheets), `sig.txt` |
| `studio/receipts/` | continuity, final QA | `continuity.json`, `final-qa-<joins|flow|watch>.json` |
| `studio/timings.json` | `studio.py log` | stage start/end times |

## studio.py
| Command | Does |
|---|---|
| `init <W> --title T [--direction D]` | studio folders, moves draft beats aside, fills the designer brief template |
| `resolve <W>` | plan -> chapters file -> `cutroom plan` -> slots snapped to output frames (floor), contiguous between chapter cards; timing files; job.json per slot; the cut lock |
| `verify <W>` | exit 3 when cuts/chapters moved under the slots; lists the stale keys |
| `status <W> [--json]` | per slot: brief (missing/draft/final), render version, check, STALE-BRIEF / STALE-RENDER |
| `args <stage> <W>` | the Workflow args JSON: `research`, `plan`, `briefs [--chain-design]`, `design [--only K1,K2] [--max-rounds N]`, `continuity`, `final-qa [--video]`. Issuing briefs/design stamps the slot signature on them (`--no-stamp` for dry runs) |
| `wire <W> [--allow-failing]` | passing renders -> `beats/studio-films.yaml`, then `cutroom check`; exit 2 = slots missing a passing render |
| `timing <W> --slot KEY` or `--from wA --to wB [--json]` | a slot's (or any range's) words with output ms and frames |
| `boxes <png> --phrase "..." [--all] [--json]` | glyph boxes of a phrase in a capture (cutroom `boxes` when the engine has it, else tesseract; one segment per line; notes punctuation glued to the phrase) |
| `sheet-video <video> --every S` or `--at t1,..` | a contact sheet (alpha shown over the canvas) + a frame list |
| `check-render <W> <KEY> [--render P]` | mechanical checks: frame count, dips to black, static holds over 6 s, anything inside the camera tile or badge area (BEHIND), alpha outside the panel and coverage (FRONT), beat lead vs word frames |
| `log <W> <stage> start|end` / `report <W>` | the timing log / a markdown summary for the report |
| `jev-privacy <W> [--video P] [--every S]` | decision-model privacy sweep of a render (Jev, Sage fallback): candidate stretches with boxes, `studio/receipts/privacy-sweep/candidates.jpg`, suggested blur beats in `studio/receipts/privacy-candidates.json` (never in `beats/`) |
| `jev-frames <W> <KEY> [--render P]` | decision-model pre-screen of a BEHIND film: `flagged` (clipped/overlap/empty/too small), `unsure`, `weakest` (polish) -> `checks/jev-frames-v<N>.json` |
| `polish <video> [--every S]` | polish score per sampled frame, weakest first (a ranking for reviewers, never a gate) |
| `rank-crops --claim "..." a.png b.png ...` | a starting order among captures for one claim |

## Staleness (why a stage reruns)
Every slot has a signature (slot0, frames, its words' frames, mode). Issuing a brief or a design task records the
signature next to it. If cuts or chapters change, `resolve` gives the slot a new signature: its brief and render
become STALE, `args briefs`/`args design` re-issue them, and `wire` refuses the old render.

## Composition contract (what cutroom does with the films)
- BEHIND film: 1920x1080 opaque h264, canvas included, camera tile and badge area left clear; wired as
  `{at: "o:<slot0 s>", hold: <frames/30>, lead: 0, kind: video, fit: frame, loop: false}`.
- FRONT overlay: 1920x1080 ProRes 4444 with alpha, same beat plus `alpha: true`, drawn over his screen recording.
- cutroom draws the camera tile and badge on top, the chapter cards between slots, blur beats on the screen
  recording, the audio, captions and chapters.
- Known engine gap (until the helpers branch lands): in a stretch with NO screen recording (talking head only,
  e.g. a hook), cutroom switches to the full-frame camera layout and the camera covers a BEHIND film. Check one
  still inside every such slot after wiring (`cutroom still W --at o:<s>`).

## Decision models: where they plug in (all optional, all additive)
- **Checker** (each round): `jev-frames` first; open its `flagged`, then `unsure`, then `weakest` frames FIRST, then do
  the full 13-point pass on every beat as before. A clean pre-screen is never a pass.
- **Continuity / final QA**: `polish <render>` to start with the weakest frames; every reviewer still watches it all.
- **Privacy, after the first full render**: `jev-privacy W`. Look at `candidates.jpg`; for each real detail copy its
  suggested blur into `beats/review-privacy.yaml` (re-anchor to `s:` source time), then re-render. It adds to the OCR
  regex (it caught a URL with ids, a partial email and a bank name the regex missed); it replaces nothing.
- **Brief writer**: `rank-crops` when a claim has several captures; the writer still picks the one with his exact words.
- **Not used** (benchmarked, not good enough): proof verification (Jev flags everything, Sage misses weaker claims),
  highlight boxes (Jev's box is too loose; `boxes` stays), retake judgment (below the Opus pass's recall).
