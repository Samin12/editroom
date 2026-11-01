# cutroom design

## What went wrong before (the YouTube course edit, Oct 2026)

| Problem | Cost | cutroom's answer |
|---|---|---|
| 186 motion graphics, each a hand-written React/Remotion scene (median 33 KB of TSX) | ~1.4M tokens of generated code, ~0.8M tokens of briefs, 274 renders | A fixed vocabulary of 15 beat templates rendered natively in Rust. An agent writes ~2 lines of YAML per visual. |
| Visuals timed in milliseconds on the uncut timeline; retakes cut later | Every placed visual drifted; days of re-sync (voice-clock resampling, re-placement) | Everything anchors to WORDS. Cuts change the time map; visuals follow their words automatically. |
| The checker modelled the editor's renderer and disagreed with it (empty-panel fallback) | Bugs shipped to the master; hours of frame-by-frame verification | One `compose::segments` model drives BOTH the renderer and `cutroom check`. What is checked is what is drawn. |
| Editor quirks (ripple trims repacking layers, layout merges, 1 ms slivers) | Hundreds of transactions with strict project-wide diffs | No editor in the loop: cutroom renders directly from the plan. The Borumi bundle is only read. |
| Secrets on screen found by hand | Several agents, one window still shipped raw | OCR scan proposes blur beats; `check` refuses to pass any unblurred finding. |
| Export of the full video | ~80 min per export, more after every fix | Parallel chunk render, joined without re-encoding. |

## Pipeline

```
Borumi bundle / files ──ingest──▶ source.json (takes, clips, words on one source timeline)
                                     │
            cut (retakes, fillers) ──┤──▶ cuts.yaml   ┐
        privacy (OCR secrets) ───────┤──▶ beats/privacy.yaml
   agent (chapters + beats YAML) ────┘──▶ edit.yaml, beats/*.yaml
                                     │
                         resolve ────▶ Plan: time map (frame-exact pieces + inserted cards),
                                     │       beats/chapters in output time, kept words
                         compose ────▶ segments: what each output instant shows
                                     ├──▶ check (lint)            ┐ same model
                                     └──▶ render (parallel)       ┘
                                            ffmpeg decode (crop+scale) → Rust compositor
                                            (canvas, panel, camera tile, stage beats, overlays,
                                             privacy mosaic, zoom, badge, cards, captions)
                                            → ffmpeg encode per chunk → concat (copy) + soundtrack
```

## Key decisions

- **Source timeline.** The raw recording is flattened: takes back to back, each with camera,
  screen, mic and system clips. Cuts are source ranges; inserts (chapter cards) add output time.
- **Frame-exact pieces.** Every kept piece is a whole number of output frames, so audio and
  video cut at the same instants and chunk boundaries never split a frame.
- **Word cuts keep breathing room.** A cut ends `min(gap/2, 120 ms)` from the kept neighbours:
  never a sliver of a removed word, never a clipped consonant.
- **Screen-normalized coordinates** for anything drawn on the screen recording (rings, blurs,
  zooms), mapped through the same transform as the pixels, so they stay on target in any layout.
- **Frame-pure graphics.** Every pixel is a function of (beat, output time): any frame can be
  rendered alone (stills, sheets, parallel chunks) and renders are deterministic.
- **Token economy.** Agents read `brief` (one sentence per line, prefixed by a word id), write
  YAML, and read terse `check` lines and one contact-sheet image instead of many frames.
- **Parallel by construction.** Cut detection and privacy scanning are pure functions of the
  source; beats for different chapters live in different files with phrase scopes; rendering
  splits into GOP-aligned chunks.
