# Look, layout, music and loudness

## The default look (Samin's)

Dark canvas `#121212` with a slow-drifting 40 px dot grid; the screen recording or the visual in the
left panel; his camera tile on the right; the step badge above the camera; terracotta accent
`#E2774E`; Inter for text. Chapter cards are inserted between sections (1.8 s) and the badge
follows them. This came from his approved proof-first edits; change it only when he asks.

## Changing the look: `style:` in `W/edit.yaml`

```yaml
style: {accent: "#3B82F6", canvas: "#0B0B0F", dot: "#2A2A33", ink: "#F5F5F5", muted: "#A1A1AA", card: "#18181B", dot_pitch: "48", drift: "2"}
```

Keys: `accent`, `canvas` (or `bg`), `dot`, `ink`, `muted`, `card`, `dot_pitch`, `drift`. Other shades
derive from these. "Use my brand colors and a light background" = set these and re-render; nothing
else changes because the beats are content, not styling.

## Layout

```yaml
layout:
  default: panel                       # panel | screen | camera
  panel: [0.01, 0.0178, 0.7676, 0.9644]
  camera: [0.7966, 0.2126, 0.1843, 0.5747]
```

Per stretch: a `layout` beat, e.g. `{at: w900, until: w1100, kind: layout, mode: camera}` for a long
talking-head stretch with nothing on screen, or `mode: screen` for a full-width demo.

## Chapter kicker

The chapter card and the badge read `output.kicker` + the chapter number: "Tip" shows "TIP 3 OF 10",
"Step" (the default) "STEP 3 OF 10", and `""` hides the kicker and the big number. The agent's
chapters pass sets it from how he numbers his points, with `output.intro` (the 0:00 line of
chapters.txt), in `beats/00-chapters.yaml`; a value in `edit.yaml` wins over it.

## Music bed

`--music bed.mp3` on the pipeline copies it to `W/music/` and sets (engine reference: `engine/docs/AUDIO.md`):

```yaml
output: {height: 1080, fps: 30, chapter_cards: insert, card_ms: 1800,
         music: {src: "music/bed.mp3", gain_db: -18, duck_db: 12, fade_ms: 1500}}
```

| field | default | what |
|---|---|---|
| `src` | required | audio file, relative to the work dir (`check` errors when it is missing) |
| `gain_db` | -18 | level of the bed against the file's own level |
| `duck_db` | 12 | how many dB **quieter** the bed gets while he talks (a positive number) |
| `attack_ms` | 60 | how fast it ducks when speech starts |
| `release_ms` | 450 | how fast it comes back after speech stops (after a 250 ms hold, so it does not pump between words) |
| `fade_ms` | 1500 | fade in at the start and out at the end of the video |
| `threshold_db` | -45 | narration level (dBFS short-term peak) that counts as speech |

It loops to the length of the video, keeps playing under the (silent) chapter cards, and is mixed
before `--loudnorm`, so the final -14 LUFS includes it. A talk-heavy tutorial: `gain_db: -22`.
Preview without it: `--render-args "--no-music"`. Never use a commercial track without a license
(Content ID). Edit the numbers in `edit.yaml`, then `--from-stage render`.

## Narration source

The voice track is the **mic** clips; else the camera's own audio; else the system audio. System
audio (sound from the screen recording) mixes under the narration at 0.6
(`--render-args "--system-gain 0.3"`; 0 mutes it). Every cut edge gets a 5 ms fade, so cuts never click.

## Loudness

The pipeline renders with `--loudnorm`: -14 LUFS integrated, true peak -1.5 dBTP, YouTube's
reference.

## Resolution and codec

1080p30 H.264 on the hardware encoder by default. `--height 2160` for 4K (about 5x realtime instead
of 15x), `--codec hevc` for smaller files. Do not use `x264` unless the hardware encoder fails: it
competes with the decoders for the CPU.

## Footage, b-roll and pre-rendered clips: the `video` beat

```yaml
- {at: w800, until: w860, kind: video, src: clips/broll.mp4, fit: panel}
- {at: w900, until: w940, kind: video, src: clips/lower-third.mov, fit: frame, alpha: true, loop: false}
```

`fit: card` (default: on a card in the panel, loops), `panel` (fills the panel slot), `frame` (the
whole frame under the camera tile; a transparent `.mov` overlays the footage). `alpha` defaults to
what the file has; `loop` defaults to true for `card`, false otherwise (the last frame holds).
Clips are muted. `from: 12.5` starts 12.5 s into the clip.

## Captions

Cutroom writes `<video>.srt` next to the video for YouTube's caption upload. Burned-in captions:
`output: {captions: true}` in `edit.yaml` (off by default for long-form).
