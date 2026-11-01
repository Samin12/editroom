# The canvas: cutroom's dot grid, exactly

Section films rendered elsewhere (Remotion) sit full frame in the video. Where a film meets
cutroom's own picture (chapter cards, stretches without a film, the area around the camera tile),
the dot grid must not jump. A film that draws the canvas with these numbers matches cutroom pixel
for pixel.

`cutroom canvas --at 12.5 --json` prints these numbers for an edit (pass `--work <dir>` for its
`style`). `cutroom canvas --at 12.5 -o c.png` renders the reference frame to compare against.
`cutroom slots --json` repeats them under `canvas`.

## Parameters (defaults; `style:` in edit.yaml can change them)

| name | value at 1080p | style key |
|---|---|---|
| background | `#121212` | `canvas` |
| dot colour | `#2C2C2C` | `dot` |
| pitch | 40 px | `dot_pitch` |
| dot radius | 1.6 px | (fixed) |
| drift | 2.5 px/s, down and to the right (both axes) | `drift` |

At other heights, multiply by `scale = height / 1080`. The pitch is rounded: `pitch = max(4,
round(40 * scale))`.

## Formula

`t` is OUTPUT time in seconds, counted from the first frame of the finished video (not from the
start of a film). A film that starts at output time `slot0_ms` (from `cutroom slots`) uses
`t = (slot0_ms + frame * 1000 / fps) / 1000` for its frame `frame`.

```
off   = drift * scale * t                      (px)
off   = off mod (pitch * 1000)                 (wrap; the grid repeats every pitch anyway)
i     = floor(off)
k     = round((off - i) * 8)                   (the drift is quantized to 1/8 px)
if k == 8: i += 1; k = 0
offq  = i + k / 8

dot centres (frame px, pixel edges at integers, origin top-left):
  x = offq + n * pitch,  y = offq + m * pitch  for all integers n, m

pixel (x, y), centre (x + 0.5, y + 0.5):
  dx  = distance from x + 0.5 to the nearest dot column, wrapped:  u = (x + 0.5 - offq) mod pitch; dx = min(u, pitch - u)
  dy  = the same for rows
  cov = clamp(radius + 0.5 - sqrt(dx^2 + dy^2), 0, 1)
  rgb = round(bg + (dot - bg) * cov)           (per channel, then clamped to 0..255)
```

Example: at `t = 12.5 s`, `off = 31.25`, so dots sit at 31.25, 71.25, 111.25... on both axes.
Pixel (71, 71) is the brightest of that dot (`#2C2C2C`), and (69, 71) is 27.

## In Remotion

Draw the canvas into a `<canvas>` per frame. Build the 8 phase variants of one `pitch x pitch`
cell once, then tile the variant for `k`, shifted by `i`:

```ts
const BG = [0x12, 0x12, 0x12], DOT = [0x2c, 0x2c, 0x2c];
function cell(p: number, r: number, f: number): ImageData {
  const img = new ImageData(p, p);
  for (let y = 0; y < p; y++) for (let x = 0; x < p; x++) {
    const d = (v: number) => { const m = ((v - f) % p + p) % p; return Math.min(m, p - m); };
    const dx = d(x + 0.5), dy = d(y + 0.5);
    const cov = Math.min(1, Math.max(0, r + 0.5 - Math.hypot(dx, dy)));
    const o = (y * p + x) * 4;
    for (let c = 0; c < 3; c++) img.data[o + c] = Math.round(BG[c] + (DOT[c] - BG[c]) * cov);
    img.data[o + 3] = 255;
  }
  return img;
}
// per frame: t = (slot0_ms + frame * 1000 / fps) / 1000
// off = (2.5 * t) % (40 * 1000); i = Math.floor(off); k = Math.round((off - i) * 8); if (k === 8) { i++; k = 0; }
// then fill the frame with cell(40, 1.6, k / 8) tiled so that tile (0,0) starts at (i mod 40) - 40 on both axes
```

Check a film against cutroom: render its frame `f` and `cutroom canvas --at <(slot0_ms + f *
1000 / 30) / 1000>` and compare an empty corner of both.
