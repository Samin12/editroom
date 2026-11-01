# QA: what is checked, and how to fix what it finds

Three layers, cheapest first. The pipeline runs the first two; the third is your contact-sheet review.

## 1. The engine (`cutroom check`, before render)

Errors block the render: an anchor on a cut word or not found, overlapping stage beats in one slot,
a missing file, an empty panel, an unblurred privacy finding, a pending proof. Fix the beat named in
`W/logs/check.txt`; never delete a blur beat and never render with `--force`.

## 2. The file (`scripts/qa.py`, after render)

| Finding | Severity | Usual cause | Fix |
|---|---|---|---|
| duration differs from the plan | blocker | a chunk failed to render | `--from-stage render`; read `logs/render.log` |
| no audio stream | blocker | no narration source | check `source.json` clips (mic, camera audio) |
| black ≥ 0.4 s | major | a missing clip, a cut landing on nothing | `C still W --at o:<t>`; fix the beat or cut |
| camera tile frozen ≥ 2 s | major | a stalled or missing camera clip | check the camera clip at that source time |
| whole frame unchanged ≥ 20 s | minor | a static screen while he talks | a `zoom`/`ring` on what he says, or `layout: camera` for the stretch |
| integrated loudness off -14 by > 1.5 LU / true peak > -1 | minor | rendered without `--loudnorm` | the pipeline always passes it; re-render |
| loudness jump > 4 LU between 10 s windows | minor | system audio or a different take | `--render-args "--system-gain 0.4"` or check that source |
| dead air ≥ 2 s | minor | a pause the cut kept | add a cut in `edit.yaml` (`cuts: - {from: wA, to: wB, why: dead air}`) unless it shows an action |

`W/qa.json` has every finding with its output time; `W/qa.md` is the summary in the report.

## 3. Eyes (gate 12, and the final spot check)

- The contact sheets: `W/review/sheet-NN.png` with `sheet-NN.md` listing each frame's time, kind,
  text and beats file. What to look for is in `agents/visual-reviewer.md`.
- After render, spot-check the finished file around anything you changed:
  `ffmpeg -ss <t> -i "<video>" -frames:v 1 /tmp/f.png` and Read it.
- Audio leftovers the transcript cannot show (a doubled phrase, a clipped word at a join): listen to
  ±3 s around suspicious joins, or transcribe the finished file and diff it against `C brief W`
  (remember whisper collapses stutters and invents words at joins: check each side of a join alone).

## Fix without breaking sync

Everything is anchored to words, so cutting more never knocks a visual out of sync. To remove a
beat: delete its line. To move it: change its `at`. To shorten: `until` or `hold`. Then
`--from-stage check`.
