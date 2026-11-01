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

## Music bed

`--music bed.mp3` on the pipeline copies it to `W/music/` and sets:

```yaml
output: {height: 1080, fps: 30, chapter_cards: insert, card_ms: 1800,
         music: {src: "music/bed.mp3", gain_db: -18, duck_db: -12, attack_ms: 60, release_ms: 450, fade_ms: 1500}}
```

It loops under the whole video, dips `duck_db` under his voice, and fades in and out. Lower
`gain_db` (-22) for a talk-heavy tutorial; never use a commercial track without a license (Content
ID). Edit the numbers in `edit.yaml` and `--from-stage render`.

## Loudness

The pipeline renders with `--loudnorm`: -14 LUFS integrated, true peak -1.5 dBTP, YouTube's
reference. System audio (sound from the screen recording) mixes under the mic at 0.6; change with
`--render-args "--system-gain 0.3"` (0 mutes it).

## Resolution and codec

1080p30 H.264 on the hardware encoder by default. `--height 2160` for 4K (about 5x realtime instead
of 15x), `--codec hevc` for smaller files. Do not use `x264` unless the hardware encoder fails: it
competes with the decoders for the CPU.

## Captions

Cutroom writes `<video>.srt` next to the video for YouTube's caption upload. Burned-in captions:
`output: {captions: true}` in `edit.yaml` (off by default for long-form).
