# Audio: narration, system audio, music bed

`cutroom render` builds the soundtrack once, from the same time map as the picture, in parallel
with the video chunks.

## Narration

The track that carries the voice is chosen in this order:

1. the **mic** clips (Borumi's mic recording, or `--mic` in `ingest files`);
2. else the **camera's own audio** (the first camera file that has an audio stream);
3. else the **system audio** clips.

With none of them, and no music bed, the video has no audio track.

Every kept stretch is decoded at its exact source position and joined with 5 ms fades at each
cut, so cuts never click. Inserted chapter cards are silent (the music bed keeps playing under
them). System audio is mixed under the narration at `--system-gain` (default 0.6; 0 mutes it).

## Music bed

```yaml
output:
  music: {src: music/bed.mp3, gain_db: -18, duck_db: 12, attack_ms: 60, release_ms: 450, fade_ms: 1500}
```

| field | default | what |
|---|---|---|
| `src` | (required) | audio file, relative to the edit's folder or absolute |
| `gain_db` | -18 | level of the bed against the file's own level |
| `duck_db` | 12 | how much quieter the bed gets while he talks |
| `attack_ms` | 60 | how fast it ducks when speech starts |
| `release_ms` | 450 | how fast it comes back after speech stops (250 ms hold first) |
| `fade_ms` | 1500 | fade in at the very start and out at the very end of the video |
| `threshold_db` | -45 | narration level (dBFS short-term peak) that counts as speech |

- The bed loops to the length of the video. A partial render (`--from/--to`) starts the bed at
  the same point of the loop a full render would, so previews sound like the final.
- Ducking is a sidechain on the narration mix (mic + system audio): an envelope follower with a
  250 ms hold so the bed does not pump between words, then the attack/release smoothing.
- The bed is mixed before `--loudnorm` (-14 LUFS), so the final level includes it.
- `cutroom render --no-music` leaves the bed out without editing the spec.
- `check` errors when `src` does not exist.

Measured on a real 90 s section (bed at -20 dB, duck 12 dB, no loudnorm): the bed sits at
-34 dBFS in pauses and -44 dBFS under speech (10 dB; short pauses do not fully release). On
synthetic speech bursts with long pauses the duck measures 11.5 dB.

## Memory

The soundtrack is mixed in 20 s blocks (12 blocks decoded at once) and streamed to the encoder,
so memory stays around 100 MB however long the video is. Split decodes join sample-exactly
(ffmpeg's accurate seek, verified on WAV and AAC), and the output is bit-identical to mixing the
whole track at once.
