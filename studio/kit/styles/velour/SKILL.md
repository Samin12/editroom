<!-- guide-version: 4 -->
# Velour style: design guide and living learnings

> **PANEL MODE (Samin, 2026-10-09).** Every animation now plays INSIDE the left screen panel of his
> layout (where the screen recording sits), with his camera tile and the step badge on the right,
> outside the render. `VelourCanvas` does this automatically: it paints the drifting dot canvas,
> draws the panel (rounded, shadowed) at x 19, y 19, 1474x1042, and scales your 1920x1080 stage
> uniformly (x0.768) into it. Keep authoring at 1920x1080. The bottom-right camera keep-out no longer
> applies (the camera is outside the panel), so use the whole stage; text must stay legible at 0.77x
> (body >= 36 px, labels >= 28 px in stage pixels). Do not nest a second full VelourCanvas; nested ones
> render plain.

> **DARK MODE (Samin's call, 2026-10-09, overrides anything below that says cream).** The whole
> video sits on the Borumi canvas: near-black `#121212` with a grey dot grid (40 px pitch). Both
> `VelourCanvas` tones render that canvas (`tone="dark"` adds a warm centre glow). The palette is
> semantic and already dark: `C.ink` is LIGHT primary text, `C.muted` secondary, `C.paper` a raised
> dark card, `C.dark` a second lighter card tone, `C.accent` terracotta. Never put `C.ink` text on a
> white or light surface you painted yourself; never paint a full-frame background over the canvas.
> Media cards carry a faint light edge so they lift off the dark canvas.

The look for Samin's AI Influencer video: **a premium editorial explainer**. Warm cream paper
(or deep charcoal for dramatic beats), ink-black Inter headlines that build word by word, one
terracotta accent for the subject, rounded media cards and phones that PLAY the real footage
(Jean Phil's viral originals, Gaston Velour's recreations, side-by-sides, ads). It matches the
asset deck he already approved (cream background, terracotta numbered cards, black cards).
It is NOT hand-drawn: do not use the sketch primitives except, sparingly, the engine's
`SketchArrow`/`SketchCircle` in accent as a hand-made annotation on top of a photo.

Import everything from `../../../styles/velour/src`:
`VelourCanvas, Reveal, Pop, FadeOut, Headline, Kicker, Pill, CountUp, MediaFrame, MediaFill, Phone,
NumberCard, Checklist, Stamp, ArrowTo, ChatBubble, StepTracker, ChapterSlate, MiniTracker,
STEP_SHORT, STEP_TITLES, C (colours), prog, mix, geoZoom, EASE_OUT, EASE_IN_OUT, compact, money`.
Read `styles/velour/src/components.tsx` once: every prop is documented there.

## Non-negotiables
1. **Real footage over pictures.** When the narration names a video, a character, a view count or
   an ad, show the real clip playing (`Phone` for 9:16 clips, `MediaFrame` for 16:9), not a
   static still. Samin rejected "literal pictures" standing in for footage.
2. **One idea per beat, the proof is the biggest thing.** A number he says is the biggest thing in
   the frame (`CountUp`). A beat needs at least 2 s on screen; fewer strong beats beat many.
3. **Sync to the words.** `brief.md` gives every word's time from t=0. Land each beat ~4 frames
   BEFORE the word it illustrates (`SEC(t) - 4`). Clear the stage between beats with `out` props
   or `FadeOut` so beats do not pile up.
4. **Behind mode keep-out.** The presenter's camera is pinned bottom-right over the render:
   nothing essential at x >= 1488 AND y >= 772 (1920x1080). Plan layouts left-weighted or
   centered-high; phones and cards may sit right as long as their bottom edge stays above y 760.
5. **Colour discipline.** Cream canvas, ink text, ONE terracotta subject. Mint (`C.mint`) only for
   Gaston-related highlights. Charcoal tone (`<VelourCanvas tone="dark">`) for the cold-open hook
   and dramatic reveals; cream everywhere else. Never hardcode hexes: use `C.*`.
6. **Motion grammar.** Entrances rise + de-blur (`Reveal`, `Headline`, `MediaFrame` do it);
   chips and stamps `Pop`; numbers count; media push in slowly (geometric, eased, 1.00 to 1.06,
   built into `MediaFrame`/`Phone`). Keep holds alive (a push, a counter, a pulse). No hard
   jumps, no CSS transitions, everything a function of the frame.
7. **No em dashes** in on-screen text. Short words: headlines under ~7 words.
8. Videos are muted and need `startFrom` (source frame, 30 fps) when a specific moment matters;
   clips shorter than the hold need `loopFrames` (for example 8 s clips: `loopFrames={240}`).

## Layout defaults (1920x1080)
- Safe margin 120 px left/right, 90 px top. Headline block left at x 128; kicker above it.
- Headline sizes: hero 120 to 140, section 96, supporting 64; body 34 to 40, weight 500.
- Phones: h 760 to 860 for a hero phone, 560 to 640 when three sit in a row.
- Cards: radius 28 to 34, shadow built in. Gaps 40 to 56.

## Asset catalog (paths are relative to public/, pass them as `src`)
Videos (all muted h264 30 fps):
- `aiinf/vid/jp1-sun-shine-at-night.mp4` 15.0 s 9:16: Jean Phil (the viral AI character the video is about; long blond bob, brown suit) walking up grand stone steps with columns. His post did 4.76M views.
- `aiinf/vid/jp2-paris-fashion-week.mp4` 15.0 s 9:16: Jean Phil at Paris Fashion Week, chandelier hall.
- `aiinf/vid/jp3-oui-madame.mp4` 15.0 s 9:16: Jean Phil on a rooftop with a guy in a white hoodie ("Oui Madame").
- `aiinf/vid/gaston-jp1-sun-shine.mp4`, `gaston-jp2-fashion-week.mp4`, `gaston-jp3-oui-madame.mp4` 14.7 s 9:16: the SAME three videos recreated with Gaston Velour (Samin's AI influencer: towering copper swirl hair, mint velvet suit, tinted aviators, curled mustache, gold medallion) via Higgsfield Genjutsu.
- `aiinf/vid/sbs-jp1.mp4`, `sbs-jp2.mp4`, `sbs-jp3.mp4` 14.7 s 16:9: side-by-side on cream, "JEAN PHIL / the viral original" left vs "GASTON VELOUR / our AI influencer" right, VS badge.
- `aiinf/vid/progression-jeanphil-gaston-tumbler.mp4` 15.0 s 1620x960: three panels, Jean Phil original, Gaston swap, Gaston holding a pink tumbler (product placement).
- `aiinf/vid/gaston-pink-tumbler.mp4` 14.7 s 9:16: Gaston on the steps holding the pink tumbler (the product-placement ad). The tumbler is only in his hand from ~7.5 s (use `startFrom={225}`); Gaston sits in the lower half of the frame (`objectPosition: "50% 80%"`).
- `aiinf/vid/trend-dance.mp4`, `trend-walk.mp4`, `trend-reaction.mp4`, `trend-interview.mp4`, `trend-podcast.mp4` 8.0 s 9:16: Gaston in Seedance 2.0 trend clips (hallway dance, city walk, reaction, street interview, podcast mic).
- `aiinf/vid/ad-energy-drink.mp4` (Gaston drinking "VELOUR FUEL" can, mint backdrop), `ad-skincare.mp4` (Gaston applying serum, gilded room), `ad-software.mp4` (Gaston at a laptop dashboard) 5.0 s 9:16: product ads.
- `aiinf/gen/gaston-surf-wave.mp4` 5.0 s 16:9 1280x720: Gaston standing on a surfboard riding inside a big curling turquoise wave at golden hour, spray, camera tracking (his suit reads dark teal in the backlight). Use `loopFrames={150}` or slow it (`playbackRate={0.7}`) for longer holds.
- `aiinf/gen/gaston-flat-ocean.mp4` 5.0 s 16:9 1280x720: wide shot, Gaston alone sitting cross-legged on a surfboard on a dead-flat mirror ocean, no waves, slow push-in. Lonely and absurd.
- `aiinf/gen/gaston-front-cutout.png` 900x1536 transparent: Gaston full body front view, mint suit (cut from the character sheet). Perfect for a silhouette (`filter: "brightness(0)"`) that reveals into colour.
- `aiinf/gen/gaston-face-cutout.png` 932x1158 transparent: Gaston head and shoulders (brown jacket).
- REAL PROOF from Samin's screen recording (Launchpoint, the platform that pays creators per view):
  - `aiinf/vid/launchpoint-hero.mp4` 9.3 s 1280x540 (2.37:1): the Launchpoint Explore hero banner for Higgsfield's "THE AI INFLUENCER INVASION" campaign, animated (clip reel, a green views/earnings graph counting up, "$1,000,000"), with a "$1 million budget" badge and pills "AI Videos ONLY", "Up to $2,000/Video", "$2 CPM".
  - `aiinf/img/launchpoint-explore.png` 2130x2220: the whole Explore page (hero banner + the campaign list with per-video rates).
  - `aiinf/img/launchpoint-rates.png` 2130x1050: just the campaign list. Rows (2 columns, ~260 px tall each, starting at y 0): Higgsfield AI $78-$10,075 per video | Playad $65-$15,065; Pump.fun $54-$10,000 | HeyGen $75-$15,000; Polymarket $45-$10,000 | Krea AI $75-$10,020; Invo $51-$9,045 | Whop $67-$4,500; High Volume Shopping App $4-$1,944 | Underdog AI $43-$3,040. In this image: row 1 y ~0-180, row 2 y ~230-410, row 3 y ~460-640, row 4 y ~690-870; left card x ~0-1020, right card x ~1065-2130. Verify positions in a still before drawing on them.
  - `aiinf/img/launchpoint-higgsfield-rates.png` 1638x1827: the Higgsfield AI campaign pop-up: "Best pay $78-$10,075 per video", "WE JUST GAVE OUT 55K IN CASH!!!", Pay $53-$10,050 per video, tiers 1K views $53, 10K views $80, 50K views $200, 100K views $350, post on TikTok/Instagram/YouTube/Snapchat/Facebook.
  - `aiinf/img/contract-table.png` 1890x1200 and `aiinf/img/contract-page.png` 2310x2256: the Launchpoint creator agreement payout table (1,000 views $2 ... 1,000,000 views $2,000; max $2,000 per unique video).
Images:
- `aiinf/img/character-sheet.jpg` 16:9: Gaston front / back / face panels on light grey (the character sheet).
- `aiinf/img/base.jpg` 16:9: Gaston face close-up in a brown suit on white. `thumb-cash.jpg` (Gaston fanning cash, dark lounge), `thumb-shocked.jpg` (Gaston shocked, orange backdrop), both 16:9.
- `aiinf/img/failure-1..4.jpg` 9:16: four DIFFERENT generic men in mint suits (what happens without a character sheet: the face changes every video). `consistent-cafe/gym/beach/office.jpg` 9:16: the SAME Gaston in four places.
- `aiinf/img/ad-energy-drink.jpg`, `ad-skincare.jpg`, `ad-software.jpg` 9:16 ad stills; `street.jpg`, `podcast.jpg` 9:16 Gaston stills; `pink-tumbler.jpg` 3:2 Samin holding the real pink tumbler.
- `aiinf/img/jp1-sun-shine-at-night.jpg`, `jp2-...`, `jp3-...` 9:16 stills of the Jean Phil originals.

## The chapter system
Eight steps (1-based): `STEP_TITLES` = How AI influencers make money, Connect Claude to
Higgsfield, Design your character, Make your first video, Recreate viral videos, Add product
placements, Get paid per view, Put it on autopilot. `STEP_SHORT` are the tracker labels.
- `ChapterSlate step={n}` is a complete chapter card scene (cream, giant ghost number, kicker
  "STEP n OF 8", the title, optional media card, the tracker filling to step n).
- `StepTracker current={n}` can be dropped into any scene (roadmap: `current={0}` with the nodes
  revealing one by one, then light them as the narration names them).
- `MiniTracker step={n}` is the small top-right badge rendered as a transparent still.

## Learnings log (append here; newest wins)
- 2026-10-09: dark mode. Both tones render the Borumi dark dot-grid canvas (#121212, 40 px dots). Design light-on-dark; cards and media sit on the dots.
