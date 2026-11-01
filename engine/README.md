# cutroom

**An agentic video editor engine.** Give it a raw recording (a Borumi project, or camera + screen
files) and it cuts the retakes, adds chapters and motion graphics, blurs secrets on screen and
renders the finished video, for any length of video. The deterministic work (timing, cuts,
layout, graphics, rendering, checks) runs in Rust; the model only makes the editorial calls, as a
few lines of YAML anchored to the words he says.

```
cutroom ingest borumi "My video.bmprojbundle" work/   # read-only; raw takes -> work/source.json
cutroom agent work/                                  # Claude: cut review, chapters, visuals per chapter (parallel)
cutroom privacy work/                                # OCR the screen for secrets -> blur beats
cutroom check work/                                  # lint everything without rendering
cutroom sheet work/ --beats -o sheet.png             # every visual in one image
cutroom render work/ -o final.mp4                    # + final.chapters.txt + final.srt
```

## Measured on a real project

Samin's 4-hour raw course recording ("Grow Your Business with YouTube + Claude", 49 scenes,
39,766 words, 5K HEVC screen recordings + camera + mic), on an M5 Ultra:

| step | before (Oct 2026 workflow) | cutroom |
|---|---|---|
| read the project | Borumi MCP reads in chunks | **0.4 s** (`ingest`) |
| find retakes, fillers, silences | per-scene LLM judges over two days | **20 ms** (`cut`), 0 tokens; 202 ambiguous items left for review |
| editorial passes (cut review + chapters + visuals for the whole video) | 186 hand-written Remotion scenes: ~1.4M tokens of TSX written + ~0.8M tokens of briefs read, 274 renders | **22 calls, 270k input + 25k output tokens, $1.36, 78 s** for 188 cut decisions, 20 chapters, 145 visuals |
| tokens an agent writes per visual | ~7,500 (a React scene) | **~27** (one YAML line) |
| render the whole video | ~80 min per full Borumi export (4K) | **12 min 11 s** at 1080p (15.3x realtime); ~33 min at 4K (5.6x, measured on a 5-min slice) |
| find secrets on screen | several agents, hours, one window still shipped raw | **26 s** for a 15-min section (1 email found, stored redacted); render blocked until it is blurred |
| visuals drifting after retake cuts | days of re-sync | impossible: visuals are anchored to words |
| checking the result | frame-by-frame checks against a model of the editor's renderer | `check`: the same model draws and lints |

The visuals are not like-for-like: the old scenes were bespoke animations, cutroom's are 15
templates. The point is that the templates cover what most of those scenes did (headlines,
lists, numbers, screenshots with highlights, rings on the screen, callouts, chapter cards) at a
tiny fraction of the cost, and bespoke work is still possible (render it elsewhere, drop it in as
a `video` beat).

**Cut accuracy** against the cuts the expensive workflow actually made (`crates/cut/EVAL.md`):
automatic cuts are 94.8% precise (97.5% counting identical-copy swaps), and 93% of the time they
remove was also removed by the original edit.

### Render speed

Full pipeline (decode 5K HEVC screen + camera, composite, graphics, encode H.264 on VideoToolbox,
soundtrack), 1080p30:

| clip | output | wall | speed |
|---|---|---|---|
| 3 min of an agent-edited section (9 workers) | 3:00 | 9.8 s | **18x realtime** (551 fps) |
| the whole agent-edited course (145 visuals, 20 chapter cards, 2,333 cuts) | 3:05:50 | 12 min 11 s | **15.3x realtime** (458 fps) |
| 5 min at 2160p | 5:00 | 53.8 s | **5.6x realtime** (167 fps) |

Every frame count matched the plan exactly (334,508 frames for the whole course), with the
soundtrack, `chapters.txt` (YouTube description chapters) and an `.srt` written alongside.

Decoding the 5K screen recordings is the bottleneck (about one CPU-second per second of footage).
Decode, compositing and encode run as a pipeline per worker; the output is split into GOP-aligned
chunks rendered in parallel and joined without re-encoding.

## How it works

See [docs/DESIGN.md](docs/DESIGN.md). In short:

1. **One source timeline.** `ingest` flattens the raw takes (camera, screen, mic, system audio,
   transcript words) onto one timeline. The Borumi bundle is only read (the SQLite database is
   copied with the backup API first).
2. **Everything anchors to words.** Cuts remove word ranges; chapters and visuals start at words
   (`w123`, or a phrase he says). A time map turns that into output time after every cut, so
   re-cutting never desyncs a visual.
3. **One composition model.** `compose::segments` says what each output instant shows (layout,
   screen clip, camera clip, stage visual, chapter card). The renderer draws exactly that and
   `check` lints exactly that.
4. **Native motion graphics.** 15 frame-pure templates drawn with tiny-skia and Inter
   (0.1 to 2.6 ms per 1080p frame), in Samin's look: dark dot-grid canvas, screen in the left
   panel, camera tile on the right, step badge above it.
5. **Agents do only judgement.** `cutroom agent` (or Claude Code with `skill/SKILL.md`) reads a
   compact transcript (`w123 one sentence per line`, tagged `[talk]` or `[screen]`) and writes
   YAML. Each chapter is a separate call, in parallel; every answer is validated by `check` and
   repaired once if needed.

## The edit spec

```yaml
# work/edit.yaml
source: source.json
include: [cuts.yaml, beats/*.yaml]          # cuts from `cut`, beats from agents (one file per chapter)
output: {height: 1080, fps: 30, chapter_cards: insert, card_ms: 1800}
layout: {default: panel}
chapters:
  - {at: "the first thing you need", title: Find the idea, short: Idea}
beats:
  - {at: "connect Claude to Poppy", until: w1250, kind: title, kicker: STEP 2, text: "Connect *Claude* + Poppy"}
  - {at: w1302, kind: number, value: "$10,075", label: paid for one video}
  - {at: w1340, kind: image, src: proof/rates.png, highlight: [0.1, 0.2, 0.4, 0.1], source: launchpoint.com}
  - {at: w1400, kind: ring, rect: [0.62, 0.08, 0.2, 0.06]}
  - {at: s:2014.5, until: s:2031.0, kind: blur, rects: [[0.05, 0.3, 0.4, 0.05]]}
```

Anchors: `w123` (word id from `cutroom brief`), `"a phrase"` (`#2` for the second match),
`s:754.2` (source seconds, survives cuts), `o:61.5` (output seconds), `+0.5s` offsets.

| kind | what | kind | what |
|---|---|---|---|
| `title` | headline built word by word | `ring` | circle something on screen |
| `bullets` | list, items appear as he says them | `highlight` | marker/underline on screen text |
| `number` | big stat that counts up | `arrow` | point at something on screen |
| `image` | proof screenshot, push-in, highlight | `callout` | short pill |
| `video` | real footage / b-roll in the panel | `lower` | lower-third |
| `code` | prompt or command typing on | `blur` | hide private info (mosaic) |
| `quote` | quote card | `zoom` | push into part of the screen |
| | | `layout` | panel / screen / camera for a stretch |

`*word*` in title, bullets and quote text is drawn in the accent colour.

## Privacy

`cutroom privacy` samples the screen recordings, OCRs changed frames with tesseract, and finds
emails, API keys and tokens (OpenAI/Anthropic, GitHub, Google, Slack, AWS, Stripe, JWTs, bearer
tokens), phone numbers and card numbers. Findings are stored redacted (`sk-ab1...`) and turned
into blur beats anchored to source time. `check` reports any finding that would be visible
unblurred in the output as an error, and `render` refuses to run until it is fixed (`--force` to
override).

## Install

Requirements: Rust 1.85+, ffmpeg/ffprobe on PATH; tesseract for `privacy`; `claude` (Claude Code
CLI) for `agent`; whisper-cli for transcribing plain files.

```
cargo install --path crates/cli
cp -r skill ~/.claude/skills/cutroom      # optional: drive it from Claude Code
```

## Crates

| crate | |
|---|---|
| `cutroom-core` | source model, edit spec, anchors, time map, compose, checks |
| `cutroom-ingest` | Borumi bundles (raw takes or the current timeline), plain files + whisper |
| `cutroom-cut` | retake / false-start / filler / silence detection, review queue, ground-truth eval |
| `cutroom-privacy` | OCR secret scanner, blur beat proposals |
| `cutroom-gfx` | motion graphics (tiny-skia, Inter, JetBrains Mono) |
| `cutroom-render` | decoders, compositor, parallel encode, soundtrack, chapters, captions |
| `cutroom` | the CLI, including `agent` |

## Limits (honest)

- The headless agent cannot see the screen, so it only places visuals that need no coordinates
  (titles, lists, numbers, code, quotes, callouts). Rings, highlights and zooms need a look at a
  `still` first (Claude Code with the skill can do that).
- Visuals are templates. Bespoke animation still has to be made elsewhere and dropped in as a
  `video` beat.
- About 30% of what a human editor cuts is editorial (dropping content that is never re-said);
  the detector leaves those to the model or the human.
- Output goes straight to a video file. Nothing is written back into Borumi.

## License

MIT. Fonts: Inter and JetBrains Mono under the SIL Open Font License (see `crates/gfx/assets/fonts`).
