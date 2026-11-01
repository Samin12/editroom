# Studio helpers: films rendered elsewhere, composed by cutroom

In studio mode each section of the video (the intro, each chapter) is a film designed and
rendered elsewhere (Remotion), 1920x1080 at 30 fps. cutroom cuts the recording, places the films
frame-exact, draws the camera tile, badge, chapter cards and blurs on top, and renders.

Order: cut and lock the cuts, then `slots`, then design each film from `timing`, render it at
exactly `frames`, check it with `check-render`, paste its beat line, `check`, `render`.

## `cutroom slots <work> [--json] [--src "studio/films/{id}.mp4"]`

The sections in OUTPUT time. Slot `intro` runs from 0 to the first chapter; slot `chNN` runs from
the end of chapter NN's card (or its start, if it has no card) to the next chapter. Boundaries are
snapped to frames.

```json
{
  "fps": 30.0, "width": 1920, "height": 1080, "duration_ms": 930066.67, "frames": 27902,
  "cuts_hash": "4bc5b74d3d8c6e67",
  "panel_rect": [19, 19, 1474, 1042], "camera_rect": [1529, 230, 354, 620], "badge_rect": [1529, 70, 354, 138],
  "canvas": {"bg": "#121212", "dot": "#2C2C2C", "pitch_px": 40.0, "dot_radius_px": 1.6, "drift_px_s": 2.5, "phase_step_px": 0.125},
  "slots": [{
    "id": "ch01", "n": 1, "title": "Ask For HTML", "group": "The Easy Tips",
    "start_ms": 36566.67, "end_ms": 101733.33, "frame0": 1097, "frames": 1955,
    "first_word": "w135", "last_word": "w387", "words": 214,
    "layout": [{"f0": 0, "f1": 1951, "mode": "panel", "screen": true, "camera": true},
               {"f0": 1951, "f1": 1955, "mode": "camera", "screen": false, "camera": true}],
    "beat": {"at": "o:36.5666657", "lead": 0, "hold": "65166.6667ms", "kind": "video",
             "src": "studio/films/ch01.mp4", "fit": "frame", "alpha": false, "fade": false}
  }]
}
```

- `frames` is the exact frame count the film must have.
- Films keep `camera_rect` and `badge_rect` clear: cutroom draws the camera and badge there.
- `layout` says where the raw recording has a screen capture (`screen: true`), in slot frames.
  Where it does, the film can show the capture itself, or a transparent overlay can go over it
  instead.
- `cuts_hash` changes when the cuts, cards or frame rate change. A film made for another hash no
  longer fits; re-run `slots` and re-time it.
- `beat` places the film. `at` is a microsecond before the first frame, so float rounding can
  never drop frame 0, and `hold` is exactly `frames`. Paste it into a beats file as one line.

## `cutroom timing <work> --slot ch01 [--words] [--json]`  (or `--from <anchor> --to <anchor>`)

The kept words, with output ms and frame numbers in the slot's own clock,
`f = round((ms - slot0_ms) * fps / 1000)`. Frame 0 is the film's first frame.

```json
{"slot": "ch01", "slot0_ms": 36566.67, "fps": 30.0, "frames": 1955,
 "words": [{"id": 135, "text": "So", "ms": 36700.1, "end_ms": 36760.2, "f": 4, "f_end": 6, "view": "panel", "screen": true}],
 "sentences": [{"from": "w135", "to": "w144", "f": 4, "f_end": 68, "text": "So let's start with the easiest tip on the list."}]}
```

A beat lands 2 to 4 frames before its word: `f_beat = word.f - 3`.

## `cutroom boxes <png> --phrase "..." [--all] [--pad 6] [--json]`

Where a phrase sits in a proof screenshot, measured on the glyphs with tesseract. Case,
punctuation, spaces and line wraps are ignored. Light and dark pages both work. When nothing
matches exactly, it returns the closest match with an edit distance of 1 per 8 characters.

```json
{"image": ".../proof/karpathy-vibe-coding.png", "width": 1520, "height": 1250,
 "phrase": "SuperWhisper so I barely even touch the keyboard",
 "matches": [{"text": "SuperWhisper so | barely even touch the keyboard.", "exact": false, "distance": 1, "conf": 93.1,
   "px": [53, 384, 1325, 102], "norm": [0.0349, 0.3072, 0.8717, 0.0816],
   "lines": [{"px": [1028, 384, 350, 41], "norm": [...]}, {"px": [53, 444, 616, 42], "norm": [...]}]}]}
```

`norm` is what an `image` beat's `highlight` takes. `lines` gives one box per wrapped line, for
marker highlights.

## `cutroom check-render <film> [--work <dir> --slot chNN] [--job job.json] [--json] [--md out.md]`

Decodes every frame at 1/8 size, which takes about a second per minute of film. It reports:

| kind | when | sev |
|---|---|---|
| `frame_count` | frames differ from the slot's (more than 2 is an error) | warn/error |
| `fps` | the film's rate is not the edit's | error |
| `empty_panel` | the panel shows only the canvas for over 1 s | warn |
| `black` | 2 or more black frames | error |
| `static` | nothing moves for over 6 s (measured against the start of the hold, so slow push-ins count as motion) | warn |
| `camera_rect` / `badge_rect` | content where cutroom draws the camera tile or badge | error |
| `alpha_cover` | a transparent overlay covers over 60% of the frame | warn |
| `beat_lead` | with `--job`: a beat is not 1 to 6 frames before its word | warn |

`job.json` beats look like `{"beats": [{"f": 120, "word": "w812"}]}`. The keys `frame`, `w` and
`at` also work. The exit code is 1 when there are errors. The JSON has `frames`,
`expected_frames`, `fps`, `alpha`, `alpha_cover_mean` and `alpha_cover_max`, plus `findings`
(`sev`, `kind`, `f0`, `f1`, `msg`) and `ok`.

## `cutroom sheet-video <film> --at 1.5,f300,1:02 | --every 2 [--cols 4] [--width 480] -o sheet.png`

A contact sheet of a render. Each tile is labelled with its time and frame. Transparent overlays
are shown over the canvas.

## `cutroom canvas --at 12.5 [--work <dir>] [--json] [-o c.png]`

The dot-grid canvas at an output time. See [CANVAS.md](CANVAS.md) for the exact formula.

## Edit spec fields studio mode uses

- `video` beat with `fit: frame` and `alpha: false` is an opaque film. It fills the picture
  under the camera tile, so a camera-only stretch keeps the panel layout under it instead of
  switching to the full camera.
- `fade: false` cuts in and out on the frame, with no 180 ms fade. `check` and `render` error
  when such a film is more than 2 frames off its window.
- `chapters[].group: "The Easy Tips"` groups consecutive chapters:
  - The badge reads `TIPS 1–5 · 1 / 3` over the group title, with one bar per group.
  - Before chapter 1 it reads `INTRO` over `output.intro`.
  - Cards go on group starts only (`output.card_on: auto | chapter | group`).
- `output.card_style: rea` gives the white grid card with a black title building word by word,
  as in the tips edit. Use it with `card_ms: 1500`.

Seeking is frame-exact. A film reopened mid-way, for example at a parallel chunk boundary, is
sought a quarter frame early, because ffmpeg's `-ss` is printed to 0.1 ms and could otherwise round
past the frame and drop it. `crates/render/tests/films.rs` renders a barcode film across chunk
boundaries and checks every frame.
