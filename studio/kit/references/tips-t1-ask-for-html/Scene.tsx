import React from "react";
import { Img, OffthreadVideo, Sequence, staticFile, useCurrentFrame } from "remotion";
import {
  VelourCanvas,
  C,
  velour,
  prog,
  mix,
  geoZoom,
  EASE_OUT,
  EASE_IN_OUT,
  Reveal,
  Headline,
  Pill,
  Highlight,
  SourcePill,
  TipOpener,
} from "../../../styles/velour/src";

// anim-mvas6emb: TIP 1 "Learn With Interactive HTML" (scene 094f, BEHIND, project 37121 to 104951 ms, 2035 frames).
// Stage 1920x1080, scaled x0.768 into the left panel by VelourCanvas (camera tile and badge sit outside the render).
// Word times (project ms): tips exports/_work/writer/words/094f.json (final). at(ms) = round((ms - 37121) * 30 / 1000) - 3,
// so every beat lands ~3 frames before its word. Section brief: tips Agent/receipts/briefs/T1.md (rev 4).
// Every change is sequenced: the old element leaves before the new one enters; no two texts share a spot at any frame.
// Proof images live in public/opus-tips/proof/anim-mvas6emb/ (copied unchanged from the proof pack / T1-assets).
//
// BEATS (frame, word, what shows, proof file)
//   f0     "So let's start"      Canvas only; the shared TipOpener 'TIP 1' + 'Learn With Interactive HTML' (96 px) rises in at
//                                its DEFAULT spot x 80, y 60 like every other tip (in by ~f12). (v3: was centred with a 1.02 push.)
//   f60    ("list." ends)         The opener docks into the compact header exactly like T2: x 80, dockY 28, dockScale 0.5, 12 f
//                                (parked by f72, before the profile card rises at f74).
//   f74    "So this is Thariq"    Thariq's real X profile in the tweet card (banner bottom, avatar, name, @trq212, bio,
//                                followers); slow push to 1.03 toward the bio. SourcePill 'x.com/trq212 · captured Oct 16, 2026'.
//                                proof 01-hook-html-voice/03-thariq-profile-crop.png (view rows 120 to 1060).
//   f134   (header leaves)        TIP header fades out (gone f142).
//   f141   "Claude Code"          Marker on the bio 'Claude Code @anthropicai' (src [43,831,499,868]); label chip
//                                'Thariq · Claude Code @anthropicai' slides in left of the pill row. Never 'built/creator'.
//   f182   ("Anthrop." ends)      Card, marker, chip and pill leave (card gone f194).
//   f194   "shared"               Thariq's real /eli5 post (header, 3 text lines, top of the attached ELI5 page); pill
//                                'x.com/trq212'. proof 01-hook-html-voice/02-thariq-eli5-tweet.png (view 30 to 1089).
//   f225   "people inside Anthropic"   Marker 'people at Anthropic' (glyphs [159,200,495,237]).
//   f275   "have been using a lot"     Second segment 'have been using a lot recently:' ([508,200,1022,237]); both out f346.
//   f356   "ELI5"                 Rounded box on 'ELI5' ([1035,200,1104,237], pad 5; a ring would touch the colon); out f416.
//   f429   "slash ELI5"           Marker '/eli5' ([52,300,126,337]).
//   f506   "whatever you want explained"  Marker '<what you want explained>' ([139,300,609,337]); both out f534.
//   f545   "Claude is gonna build"  Eased scroll of the same post to view 382 to 1441 (quote line 1 keeps a top margin, no
//                                half-cut line): the post's own ELI5 page fills the card. Push origin sits near the card top.
//   f611   "HTML page"            Thin CLOSED outline on the attached page (top 522, sides 50/1470, bottom drawn at source row
//                                1395, ~12 px inside the card edge at the card's push, so all four sides show), stroke 3; out f638.
//   f646   "big pictures and very few words"  Marker 'big pictures and few words' ([270,450,727,487]).
//   f694   ("words." ends)        Card and pill leave (gone f706).
//   f706   "Karpathy"             Karpathy's real post (each instance pushes 1.00 to 1.03 toward its own safe origin), instance 1 (header, 'structure your response as HTML' paragraph,
//                                'third of our brains' paragraph, 1) 2) 3) list); pill 'x.com/karpathy'.
//                                proof 01-hook-html-voice/04-karpathy-ask-html-tweet.png (view 30 to 1089).
//   f734   "said"                 Marker 'structure your' ([1214,200,1455,237]); f740 "the": 'response as HTML' ([53,250,369,287]).
//   f764   ("thing." ends)        Inside the same card, a true 10-frame crossfade (f764 to f774): instance 2 (view 1600 to 2659:
//                                TLDR paragraph + the quoted card of Thariq's X Article and its title) fades in ON TOP while
//                                instance 1 stays fully opaque underneath; instance 1 is dropped at f774. No dip, no empty card,
//                                no slide between the layers (v2 fix; v1 faded out then in and blinked empty at f768).
//   f779   "hot"                  Marker sweeps 'hot tip try ask for HTML' ([299,1800,700,1837]) to 'HTML' f835.
//   f849   ("HTML." ends)         Card and pill leave (gone f861).
//   f861   "And the reason"       Kicker 'WHY IT WORKS' + OpenStax Biology 2e page (browser card openstax.org/books/biology-2e/
//                                pages/36-5-vision) arrive together; whole card pushes 1.00 to 1.01 around the marked sentence (v2: was 1.02).
//                                SourcePill 'openstax.org · captured Oct 16, 2026'. proof T1-assets/t1-openstax-cortex-crop.png
//                                (= pack 07-openstax-third-of-cortex-crop.png at x 60 / y 30).
//   f917   "About a third"        Marker sweeps 'about one-third of the human cerebral cortex' to 'brain' (f950).
//   f950   "brain"                Label chip 'About a third of the cortex: vision' (left of the pill row).
//   f961   "is built to process"  Marker extends over 'is dedicated' (f961) and 'to analyzing and perceiving visual information'
//                                (f973 to f1006, before 'see' f1008).
//   f1012  ("see." ends)          OpenStax card, kicker, chip and pill leave (gone f1024).
//   f1024  "So when you're"       Concept: a portrait page card fills fast with grey text bars (placeholder bars, no words);
//                                slow push.  f1114 "wall": label 'A wall of text' under it.
//   f1150  ("text" ends)          Wall card slides out left and fades (gone f1160).
//   f1162  "is the slowest way"   Cambridge Core chapter page (browser card cambridge.org/core/books/abs/multimedia-learning/
//                                multimedia-principle), enters in 8 f. Row: label 'Richard E. Mayer · Multimedia Learning' +
//                                SourcePill 'cambridge.org · captured Oct 16, 2026'. proof T1-assets/t1-mayer-principle-crop.png
//                                (= pack 90-gaps/05-mayer-multimedia-principle-crop-wide.png at x 395 / y 85).
//                                Whole card pushes 1.00 to 1.02 around the sentence.
//   f1170  "slowest"              Marker sweeps 'People learn better from words and pictures than from words alone.' to 'it' f1195.
//                                The word 'slowest' is never printed (the research is about learning better, not faster).
//   f1206  "So here's the tip"    Kicker 'THE TIP' in the band above the Mayer card (stays as the header to f1315).
//   f1225  ("tip." ends)          Mayer card, label and pill leave (gone f1237).
//   f1237  "Next time you need"   The same wall-of-text card returns on the left, label 'The docs'.
//   f1285  "don't read the docs"  An accent strike draws across the card, the card dims to 40%; headline 'Don't read the docs'
//                                (his words) builds on the right.
//   f1315  ("docs." ends)         Kicker, docs card, strike and headline leave (gone f1327).
//   f1327  "Ask your agent"       His real prompt from his own recording (chat title 'GEO vs SEO interactive guide' + prompt
//                                bubble) in the tweet card look (crop inset 20 px on its own background in a 1712-wide card), slow push 1.02
//                                centred on the content (v2). Pill 'Samin's Claude app · his own prompt'.
//                                proof T1-assets/t1-prompt-crop.png (his recording, media 50 s, x 960-2480 / y 30-720).
//   f1355  "build me an interactive HTML page that explains this topic"  Marker sweeps line 1 ([336,311,1424,355]) to 'topic'
//                                f1455; out f1480.
//   f1491  "click through"        Marker 'clickable' ([1119,437,1296,481]), f1496 'sections I can step through' ([334,500,894,544]).
//   f1508  ("it." ends)           Prompt card and pill leave (gone f1520).
//   f1520  "I did this for SEO"   THE REAL PAGE Claude built: opus-tips/broll/geo-clickthrough.mp4, muted, own 16:9, no zoom
//                                (x 280, y 30, 1360x765). Pill 'geo-vs-seo.html · built by Claude for Samin'. Segments
//                                (overlapping, incoming fades in over the outgoing): A f1520 clip 0.00 | B f1680 clip 7.63 |
//                                C f1746 clip 12.10 | D f1812 clip 16.63 | E f1878 clip 26.50 (slide 7).
//   f1544  "SEO"                  Ring on the slide-1 title 'SEO'; f1568 "GEO": ring on 'GEO'; both out f1605.
//   f1688  "clicked"              Small ring pulse on the page's real next button as the cursor clicks (f1690).
//   f1736  "the agent [builds]"   Claude's real reply strip under the footage: 'Created geo-vs-seo.html, ran a command… +142 -0'
//                                / 'I built geo-vs-seo.html and opened it in your default browser.' (v2: 14 px inner padding).
//                                proof T1-assets/t1-reply-built-crop.png (his recording, x 1030-2295 / y 768-948).
//   f1751  ("pills" = builds)     Marker on 'I built geo-vs-seo.html'; strip out f1806.
//   f1777  "fraction of the time" Footage only (slides 4 and 5 turn by themselves). No timer, number or 'x faster'.
//   f1878  "if I were you"        Segment E: slide 7 'SEO gets you found. GEO gets you said.'
//   f1890  (to f1908)             The footage frame eases (as a whole, no zoom inside) to a thumbnail on the left; pill rides under.
//   f1912  "pick"                 Headline 'Pick one thing.'  f1933 "you've": subline 'you've been putting off learning'.
//   f1983  "try"                  Headline 'Try this today.'  Holds fully composed to f2024 ('today.' ends f2013).
//   f2024  (to f2034)             v3 tail exit: CTA group, GEO thumbnail and its pill fade out together over 10 f, opacity 0 on the
//                                last frame f2034, so the cut to T2 (bare canvas at its f0/f1) never blinks.
//
// Honest notes: Thariq is labelled with his bio only; the cortex sentence (not 'whole brain') is the proof for 'a third of
// your brain'; Mayer's principle is about learning better (never 'slowest'); no numbers for 'single guide' or 'fraction of
// the time'; the transcript's 'hot topic' is shown as Karpathy's real words 'hot tip try ask for HTML'. No view counts marked.

const JOB_START = 37121;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;

const P = "opus-tips/proof/anim-mvas6emb/";
const FONT = velour.font.display;
const CAPTURED = "Oct 16, 2026";

const T = {
  // A: TIP opener
  opIn: 0,
  opDock: 60, // parked by 72
  opOut: 134, // gone 142
  // B: Thariq profile
  prIn: 74, // "So" 74 (at 39678)
  prBio: at(41934), // "Claude" 141
  prOut: 182, // card gone 194
  // C: /eli5 post
  elIn: 194, // "shared" 198
  elPeople: at(44724), // 225
  elHave: at(46388), // 275
  elPairOut: 346,
  elRing: at(49074), // "ELI5." 356
  elRingOut: 416,
  elSlash: at(51528), // 429
  elWhat: at(54099), // 506
  elLine2Out: 534,
  elScrollFrom: 545,
  elScrollTo: 581,
  elMedia: at(57582), // "HTML" 611
  elMediaOut: 638,
  elBig: at(58768), // "big" 646
  elOut: 694, // gone 706
  // D: Karpathy
  kaIn: 706,
  kaS1: at(61686), // "said" 734
  kaS2: at(61867), // "the" 740
  kaSwap: 764, // 764-769 out, 769-774 in
  kaHot: at(63188), // "hot" 779
  kaHotEnd: at(65070), // "HTML." 835
  kaOut: 849, // gone 861
  // E: OpenStax
  osIn: 861,
  osA: at(67788), // "About" 917
  osAEnd: at(68874), // "brain" 950
  osLabel: at(68874), // 950
  osB: at(69254), // "is" 961
  osC: 973,
  osCEnd: 1006, // "see." 1008
  osOut: 1012, // gone 1024
  // F: wall of text
  waIn: 1024,
  waLabel: at(74339), // "wall" 1114
  waOut: 1150, // gone 1160
  // G: Mayer
  maIn: 1162,
  maMark: at(76221), // "slowest" 1170
  maMarkEnd: at(77062), // "it." 1195
  maOut: 1225, // gone 1237
  tipKicker: at(77424), // "So" here's the tip 1206
  tipOut: 1315,
  // H: the docs
  doIn: 1237,
  doStrike: at(80038), // "don't" 1285
  doOut: 1315, // gone 1325/1327
  // I: his prompt
  quIn: 1327,
  quMark: at(82378), // "build" 1355
  quMarkEnd: at(85711), // "topic," 1455
  quMarkOut: 1480,
  quClick1: at(86912), // "click" 1491
  quClick2: at(87093), // "through" 1496
  quOut: 1508, // gone 1520
  // J: the real page (B-roll)
  brIn: 1520,
  brSeo: at(88674), // 1544
  brGeo: at(89495), // 1568
  brRingsOut: 1605,
  brNext: 1688, // cursor click f1690
  stIn: 1736,
  stMark: at(95581), // "pills" 1751
  stOut: 1806,
  thumb: 1890, // to 1908
  ctaPick: at(100947), // 1912
  ctaSub: at(101648), // 1933
  ctaTry: at(103309), // 1983
  endOut: 2024, // v3: whole close (CTA + GEO thumbnail + pill) fades f2024 to f2034 ('today.' ends f2013), gone ON f2034
};

/** v3 tail exit: 1 until T.endOut, 0 on the last frame (f2034), so the T1|T2 join (T2 f0 is bare canvas) never blinks. */
const tailOut = (f: number) => 1 - prog(f, T.endOut, 10, EASE_IN_OUT);

/* ------------------------------------------------------------------ helpers */

type Box = [number, number, number, number];

const inOut = (f: number, delay: number, dur: number, out?: number, outDur = 8): number =>
  prog(f, delay, dur) * (out === undefined ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT));

/** Translucent accent swipe (same look as the velour Highlight marker) with a LINEAR sweep, so long sweeps track the words. */
const Marker: React.FC<{ x: number; y: number; w: number; h: number; delay: number; dur?: number; pad?: number; out?: number }> = ({
  x,
  y,
  w,
  h,
  delay,
  dur = 10,
  pad = 4,
  out,
}) => {
  const f = useCurrentFrame();
  const t = prog(f, delay, dur, (v) => v);
  const o = out === undefined ? 1 : 1 - prog(f, out, 8, EASE_IN_OUT);
  if (t <= 0 || o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: x - pad,
        top: y - pad,
        width: (w + 2 * pad) * t,
        height: h + 2 * pad,
        borderRadius: 8,
        background: C.accent,
        opacity: 0.3 * o,
      }}
    />
  );
};

/** A source-px box drawn as a marker inside a layer scaled by `s` (box is [x0,y0,x1,y1] in the PNG). */
const BoxMarker: React.FC<{ box: Box; s: number; delay: number; dur?: number; out?: number; pad?: number }> = ({ box, s, ...rest }) => (
  <Marker x={box[0] * s} y={box[1] * s} w={(box[2] - box[0]) * s} h={(box[3] - box[1]) * s} {...rest} />
);

/** The tweet-card look (velour TweetCard): rounded dark card, thin C.line outline, card shadow, 12 f rise + de-blur,
 *  12 f exit, and a slow geometric push over the card's OWN life (delay to out + 12) toward `focus` (card px). */
const ShotCard: React.FC<{
  x: number;
  y: number;
  w: number;
  h: number;
  delay: number;
  out: number;
  dur?: number;
  push?: number;
  focus: [number, number];
  children?: React.ReactNode;
}> = ({ x, y, w, h, delay, out, dur = 12, push = 1.03, focus, children }) => {
  const f = useCurrentFrame();
  const zoom = geoZoom(prog(f, delay, out + 12 - delay, EASE_IN_OUT), 1, push);
  return (
    <Reveal delay={delay} dur={dur} out={out} outDur={12} style={{ position: "absolute", left: x, top: y, width: w }}>
      <div
        style={{
          position: "relative",
          width: w,
          height: h,
          borderRadius: 26,
          overflow: "hidden",
          background: C.paperEdge,
          boxShadow: velour.shadow.card,
          outline: `2px solid ${C.line}`,
        }}
      >
        <div style={{ position: "absolute", inset: 0, transform: `scale(${zoom})`, transformOrigin: `${focus[0]}px ${focus[1]}px` }}>
          {children}
        </div>
      </div>
    </Reveal>
  );
};

/** The PNG at scale s, with its source row `top` at the card's top edge; children use DISPLAY px of the whole PNG. */
const ShotLayer: React.FC<{ src: string; srcW: number; srcH: number; s: number; top: number; opacity?: number; dy?: number; children?: React.ReactNode }> = ({
  src,
  srcW,
  srcH,
  s,
  top,
  opacity = 1,
  dy = 0,
  children,
}) => (
  <div style={{ position: "absolute", left: 0, top: -top * s + dy, width: srcW * s, height: srcH * s, opacity }}>
    <Img src={staticFile(P + src)} style={{ position: "absolute", left: 0, top: 0, width: srcW * s, height: srcH * s, maxWidth: "none" }} />
    {children}
  </div>
);

/** The velour BrowserCard look (title bar with three dots + URL pill, then the capture), with an entrance of `dur` frames,
 *  a 12 f exit and an optional whole-card scale (no push inside the image, so line starts never clip). Children sit over
 *  the IMAGE AREA in image-area px. */
const Browser: React.FC<{
  url: string;
  src: string;
  x: number;
  y: number;
  w: number;
  h: number;
  delay: number;
  out: number;
  dur?: number;
  scale?: number;
  origin?: string;
  children?: React.ReactNode;
}> = ({ url, src, x, y, w, h, delay, out, dur = 12, scale = 1, origin = "50% 50%", children }) => (
  <div style={{ position: "absolute", left: x, top: y, width: w, transform: `scale(${scale})`, transformOrigin: origin }}>
    <Reveal delay={delay} dur={dur} out={out} outDur={12}>
      <div style={{ borderRadius: 22, overflow: "hidden", background: C.paper, boxShadow: velour.shadow.card }}>
        <div style={{ height: 64, display: "flex", alignItems: "center", gap: 12, padding: "0 22px", background: C.dark }}>
          {[C.red, C.gold, C.green].map((c) => (
            <div key={c} style={{ width: 16, height: 16, borderRadius: 8, background: c }} />
          ))}
          <div
            style={{
              marginLeft: 18,
              flex: 1,
              height: 40,
              borderRadius: 20,
              background: C.paper,
              color: C.inkSoft,
              fontFamily: FONT,
              fontSize: 26,
              display: "flex",
              alignItems: "center",
              padding: "0 22px",
              whiteSpace: "nowrap",
              overflow: "hidden",
            }}
          >
            {url}
          </div>
        </div>
        <div style={{ position: "relative", width: w, height: h, overflow: "hidden", background: C.white }}>
          <Img src={staticFile(P + src)} style={{ position: "absolute", left: 0, top: 0, width: w, height: h, maxWidth: "none" }} />
          {children}
        </div>
      </div>
    </Reveal>
  </div>
);

/** Fills its parent with a UNIFORM background corner (cw x ch source px) of the same PNG, stretched (CSS object-view-box).
 *  His recording crops carry cICP/gAMA colour chunks, so Chrome renders their #121212 background darker than any C.* fill;
 *  padding painted from the image itself is colour-managed identically and matches the crop exactly (no inner seam). */
const CropFill: React.FC<{ src: string; srcW: number; srcH: number; cw: number; ch: number }> = ({ src, srcW, srcH, cw, ch }) => (
  <Img
    src={staticFile(P + src)}
    style={
      {
        position: "absolute",
        left: 0,
        top: 0,
        width: "100%",
        height: "100%",
        maxWidth: "none",
        objectFit: "fill",
        objectViewBox: `inset(0px ${srcW - cw}px ${srcH - ch}px 0px)`,
      } as React.CSSProperties
    }
  />
);

/** SourcePill right-aligned to `right` (stage x of the card's right edge). */
const PillRight: React.FC<{ right: number; top: number; domain: string; delay: number; out: number }> = ({ right, top, domain, delay, out }) => (
  <div style={{ position: "absolute", right: 1920 - right, top }}>
    <SourcePill domain={domain} captured={CAPTURED} delay={delay} out={out} />
  </div>
);

/** Label chip on the left of a pill row (same height as the SourcePill), slides in from the left. */
const LabelChip: React.FC<{ text: string; left: number; top: number; delay: number; out: number }> = ({ text, left, top, delay, out }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 10);
  return (
    <div style={{ position: "absolute", left, top, opacity: inOut(f, delay, 10, out), transform: `translateX(${(1 - p) * -28}px)` }}>
      <Pill tone="light" size={28} style={{ padding: "10px 24px", fontWeight: 650 }}>
        {text}
      </Pill>
    </div>
  );
};

/** A plain pill for his OWN app / page (no 'captured' date): SourcePill look with an accent dot. */
const OwnPill: React.FC<{ text: string; delay: number; out?: number; style?: React.CSSProperties }> = ({ text, delay, out, style }) => {
  const f = useCurrentFrame();
  return (
    <div style={{ opacity: inOut(f, delay, 10, out), ...style }}>
      <Pill tone="light" size={28} icon={<div style={{ width: 10, height: 10, borderRadius: 5, background: C.accent }} />} style={{ padding: "10px 22px", fontWeight: 500, color: C.muted, gap: 12 }}>
        {text}
      </Pill>
    </div>
  );
};

/** Section kicker in the TipOpener's kicker look (accent rule + 34 px letter-spaced caps), 12 f in, 8 f out. */
const SectionKicker: React.FC<{ text: string; x: number; y: number; delay: number; out: number }> = ({ text, x, y, delay, out }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 12);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y + (1 - p) * 16,
        opacity: inOut(f, delay, 12, out),
        filter: p < 0.99 ? `blur(${(1 - p) * 6}px)` : undefined,
        display: "flex",
        alignItems: "center",
        gap: 20,
        fontFamily: FONT,
      }}
    >
      <div style={{ width: 54, height: 5, background: C.accent, borderRadius: 4 }} />
      <div style={{ fontSize: 34, fontWeight: 750, letterSpacing: "0.18em", color: C.accent, whiteSpace: "nowrap" }}>{text}</div>
    </div>
  );
};

/* ------------------------------------------------------------------ A: TIP opener */

// v3: the shared opener at its DEFAULT spot (x 80, y 60, same as every other tip), no wrapper and no extra push, docking
// with the SAME parameters as T2 (anim-mvas6ent: x 80, dockY 28, dockScale 0.5, 12 f). v2 centred it (x 333, y 464) with a
// 1.02 push and docked to x 40, y 16, which differed from the rest of the series.
const Opener: React.FC = () => <TipOpener n={1} name="Learn With Interactive HTML" delay={T.opIn} out={T.opOut} dockAt={T.opDock} />;

/* ------------------------------------------------------------------ B: Thariq profile */

// v3: card top 104 -> 132, so it keeps ~28 stage px under the docked header (glyphs end ~y 101 at dockY 28; v2 docked at y 16).
// Card 132 to 941, pill row 959 to ~1009: still clear of the stage bottom.
const PR = { x: 314, y: 132, w: 1292, s: 1292 / 1501, v0: 120, v1: 1060 };
const PR_H = Math.round((PR.v1 - PR.v0) * PR.s); // 809
const PR_ROW = PR.y + PR_H + 18;

const Profile: React.FC = () => (
  <>
    <ShotCard x={PR.x} y={PR.y} w={PR.w} h={PR_H} delay={T.prIn} out={T.prOut} focus={[270 * PR.s, (850 - PR.v0) * PR.s]}>
      <ShotLayer src="03-thariq-profile-crop.png" srcW={1501} srcH={1060} s={PR.s} top={PR.v0}>
        <BoxMarker box={[43, 831, 499, 868]} s={PR.s} delay={T.prBio} dur={10} out={T.prOut} pad={4} />
      </ShotLayer>
    </ShotCard>
    <LabelChip text="Thariq · Claude Code @anthropicai" left={PR.x} top={PR_ROW} delay={T.prBio} out={T.prOut} />
    <PillRight right={PR.x + PR.w} top={PR_ROW} domain="x.com/trq212" delay={T.prIn} out={T.prOut} />
  </>
);

/* ------------------------------------------------------------------ C: the /eli5 post */

const TW = { x: 314, y: 40, w: 1292, s: 1292 / 1520 }; // 0.85; viewport 900 stage px = 1059 source rows
const TW_H = Math.round(1059 * TW.s); // 900
const TW_ROW = TW.y + TW_H + 18;

const EL_TO = 382;
const ELI5_BOX_BOTTOM = 1395;
const Eli5: React.FC = () => {
  const f = useCurrentFrame();
  // view [30, 1089] -> [382, 1441]: quote line 1 (glyph top y 400) keeps a margin at the top edge, never half-cut.
  const top = mix(prog(f, T.elScrollFrom, T.elScrollTo - T.elScrollFrom, EASE_IN_OUT), 30, EL_TO);
  const s = TW.s;
  return (
    <>
      <ShotCard x={TW.x} y={TW.y} w={TW.w} h={TW_H} delay={T.elIn} out={T.elOut} focus={[760 * s, 60]}>
        <ShotLayer src="02-thariq-eli5-tweet.png" srcW={1520} srcH={1848} s={s} top={top}>
          <BoxMarker box={[159, 200, 495, 237]} s={s} delay={T.elPeople} dur={12} out={T.elPairOut} />
          <BoxMarker box={[508, 200, 1022, 237]} s={s} delay={T.elHave} dur={14} out={T.elPairOut} />
          <Highlight x={1035 * s} y={200 * s} w={69 * s} h={37 * s} kind="box" stroke={4} pad={5} delay={T.elRing} dur={10} out={T.elRingOut} />
          <BoxMarker box={[52, 300, 126, 337]} s={s} delay={T.elSlash} dur={8} out={T.elLine2Out} />
          <BoxMarker box={[139, 300, 609, 337]} s={s} delay={T.elWhat} dur={14} out={T.elLine2Out} />
          {/* the attached page, CLOSED box (v2): measured top 522 and sides 50 / 1470; the page itself runs on below the card,
              so the bottom side is drawn at source row 1395, where it stays ~12 stage px inside the card's bottom edge at the
              card's push (1.029 to 1.030 over f611-f646, origin near the top: visible rows end at ~1412). All four sides show. */}
          <Highlight x={50 * s} y={522 * s} w={1420 * s} h={(ELI5_BOX_BOTTOM - 522) * s} kind="box" stroke={3} pad={0} delay={T.elMedia} dur={12} out={T.elMediaOut} />
          <BoxMarker box={[270, 450, 727, 487]} s={s} delay={T.elBig} dur={30} out={T.elOut} />
        </ShotLayer>
      </ShotCard>
      <PillRight right={TW.x + TW.w} top={TW_ROW} domain="x.com/trq212" delay={T.elIn} out={T.elOut} />
    </>
  );
};

/* ------------------------------------------------------------------ D: Karpathy */

/** Slow push inside one image layer (card px origin), so each instance pushes toward its own safe point. */
const PushBox: React.FC<{ zoom: number; origin: [number, number]; children?: React.ReactNode }> = ({ zoom, origin, children }) => (
  <div style={{ position: "absolute", inset: 0, transform: `scale(${zoom})`, transformOrigin: `${origin[0]}px ${origin[1]}px` }}>{children}</div>
);

const Karpathy: React.FC = () => {
  const f = useCurrentFrame();
  const s = TW.s;
  const zoom = geoZoom(prog(f, T.kaIn, T.kaOut + 12 - T.kaIn, EASE_IN_OUT), 1, 1.03);
  // TRUE crossfade (v2): instance 2 fades in ON TOP over f764-f774 while instance 1 stays fully opaque underneath, and
  // instance 1 is dropped only once instance 2 is fully in. Both PNG layers are opaque and cover the whole card, so the
  // card never dips or empties. No dy offsets: the two layers never slide against each other.
  const x2 = prog(f, T.kaSwap, 10, EASE_IN_OUT);
  return (
    <>
      <ShotCard x={TW.x} y={TW.y} w={TW.w} h={TW_H} delay={T.kaIn} out={T.kaOut} push={1} focus={[0, 0]}>
        {x2 < 1 ? (
          <PushBox zoom={zoom} origin={[760 * s, 200]}>
            <ShotLayer src="04-karpathy-ask-html-tweet.png" srcW={1520} srcH={3153} s={s} top={30}>
              <BoxMarker box={[1214, 200, 1455, 237]} s={s} delay={T.kaS1} dur={6} />
              <BoxMarker box={[53, 250, 369, 287]} s={s} delay={T.kaS2} dur={10} />
            </ShotLayer>
          </PushBox>
        ) : null}
        {x2 > 0 ? (
          // origin low in the card: the article title (view bottom, 9 px margin) never pushes past the edge
          <PushBox zoom={zoom} origin={[760 * s, 800]}>
            <ShotLayer src="04-karpathy-ask-html-tweet.png" srcW={1520} srcH={3153} s={s} top={1600} opacity={x2}>
              <BoxMarker box={[299, 1800, 700, 1837]} s={s} delay={T.kaHot} dur={T.kaHotEnd - T.kaHot} out={T.kaOut} />
            </ShotLayer>
          </PushBox>
        ) : null}
      </ShotCard>
      <PillRight right={TW.x + TW.w} top={TW_ROW} domain="x.com/karpathy" delay={T.kaIn} out={T.kaOut} />
    </>
  );
};

/* ------------------------------------------------------------------ E: OpenStax */

const OS = { x: 40, y: 130, w: 1840, h: 717 }; // crop 1670x651 at 1.1018
const OpenStax: React.FC = () => {
  const f = useCurrentFrame();
  // v2: push capped at 1.01 (was 1.02): the 1840-wide card keeps ~31 stage px to the stage edges at its peak.
  const sc = geoZoom(prog(f, T.osIn, T.osOut + 12 - T.osIn, EASE_IN_OUT), 1, 1.01);
  const row = OS.y + 64 + OS.h + 20;
  return (
    <>
      <SectionKicker text="WHY IT WORKS" x={40} y={46} delay={T.osIn} out={T.osOut} />
      <Browser
        url="openstax.org/books/biology-2e/pages/36-5-vision"
        src="t1-openstax-cortex-crop.png"
        x={OS.x}
        y={OS.y}
        w={OS.w}
        h={OS.h}
        delay={T.osIn}
        out={T.osOut}
        dur={12}
        scale={sc}
        origin={`${960 - OS.x}px ${845 - OS.y}px`}
      >
        <Marker x={916} y={611} w={1622 - 916} h={646 - 611} delay={T.osA} dur={T.osAEnd - T.osA} pad={4} />
        <Marker x={1634} y={611} w={1822 - 1634} h={646 - 611} delay={T.osB} dur={12} pad={4} />
        <Marker x={18} y={662} w={732 - 18} h={696 - 662} delay={T.osC} dur={T.osCEnd - T.osC} pad={4} />
      </Browser>
      <LabelChip text="About a third of the cortex: vision" left={OS.x} top={row} delay={T.osLabel} out={T.osOut} />
      <PillRight right={OS.x + OS.w} top={row} domain="openstax.org" delay={T.osIn} out={T.osOut} />
    </>
  );
};

/* ------------------------------------------------------------------ F/H: the wall-of-text card (concept, reused) */

const frac = (v: number) => v - Math.floor(v);
/** Deterministic line widths (0..1) for the placeholder text bars: paragraphs of 6 to 8 lines, last line short. */
const wallLines = (n: number): (number | null)[] => {
  const out: (number | null)[] = [];
  let inPara = 0;
  let paraLen = 7;
  for (let i = 0; out.length < n; i++) {
    const r = frac(Math.sin((i + 1) * 12.9898) * 43758.5453);
    if (inPara === paraLen - 1) {
      out.push(0.3 + 0.3 * r);
      if (out.length < n) out.push(null); // paragraph gap
      inPara = 0;
      paraLen = 6 + Math.floor(r * 3);
    } else {
      out.push(0.84 + 0.16 * r);
      inPara++;
    }
  }
  return out;
};

const WallCard: React.FC<{
  x: number;
  y: number;
  w: number;
  h: number;
  delay: number;
  label: string;
  labelAt: number;
  out: number;
  exitX?: number;
  strikeAt?: number;
}> = ({ x, y, w, h, delay, label, labelAt, out, exitX = 0, strikeAt }) => {
  const f = useCurrentFrame();
  const enter = prog(f, delay, 12);
  const exit = prog(f, out, 10, EASE_IN_OUT);
  const op = enter * (1 - exit);
  if (op <= 0) return null;
  const push = geoZoom(prog(f, delay, out + 10 - delay, EASE_IN_OUT), 1, 1.03);
  const dim = strikeAt === undefined ? 1 : mix(prog(f, strikeAt, 10, EASE_IN_OUT), 1, 0.4);
  const padX = 56;
  const pitch = 25;
  const n = Math.floor((h - 140) / pitch);
  const lines = wallLines(n);
  // content arrives WITH the card: the first 8 lines come in with the entrance, the rest pile in fast (1 per frame)
  const lineP = (i: number) => (i < 8 ? enter : prog(f, delay + 4 + (i - 8), 6));
  const strike = strikeAt === undefined ? 0 : prog(f, strikeAt + 2, 10, EASE_OUT);
  const labelP = prog(f, labelAt, 12);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        opacity: op,
        transform: `translate(${exitX * exit}px, ${(1 - enter) * 30}px)`,
        filter: enter < 0.99 ? `blur(${(1 - enter) * 8}px)` : undefined,
      }}
    >
      <div style={{ transform: `scale(${push})`, transformOrigin: "50% 50%", position: "relative", width: w, height: h }}>
        <div
          style={{
            position: "absolute",
            inset: 0,
            borderRadius: 24,
            background: C.paper,
            boxShadow: velour.shadow.card,
            outline: `2px solid ${C.line}`,
            overflow: "hidden",
            opacity: dim,
          }}
        >
          {/* title bar of the page */}
          <div
            style={{
              position: "absolute",
              left: padX,
              top: 52,
              height: 26,
              width: (w - 2 * padX) * 0.56,
              borderRadius: 6,
              background: C.muted,
              opacity: 0.55,
            }}
          />
          {lines.map((lw, i) =>
            lw === null ? null : (
              <div
                key={i}
                style={{
                  position: "absolute",
                  left: padX,
                  top: 108 + i * pitch,
                  height: 12,
                  width: (w - 2 * padX) * lw * lineP(i),
                  borderRadius: 5,
                  background: C.faint,
                  opacity: 0.8,
                }}
              />
            ),
          )}
        </div>
        {strike > 0 ? (
          <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }} width={w} height={h}>
            <line
              x1={-24}
              y1={h * 0.86}
              x2={-24 + (w + 48) * strike}
              y2={h * 0.86 - (h * 0.72) * strike}
              stroke={C.accent}
              strokeWidth={12}
              strokeLinecap="round"
            />
          </svg>
        ) : null}
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          top: h + 30,
          width: w,
          textAlign: "center",
          fontFamily: FONT,
          fontSize: 54,
          fontWeight: 750,
          letterSpacing: "-0.02em",
          color: C.ink,
          opacity: labelP,
          transform: `translateY(${(1 - labelP) * 18}px)`,
          filter: labelP < 0.99 ? `blur(${(1 - labelP) * 6}px)` : undefined,
        }}
      >
        {label}
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------ G: Mayer */

const MA = { x: 110, y: 170, w: 1700, h: 646 }; // crop 1645x625 at 1.0334
const Mayer: React.FC = () => {
  const f = useCurrentFrame();
  const sc = geoZoom(prog(f, T.maIn, T.maOut + 12 - T.maIn, EASE_IN_OUT), 1, 1.02);
  const row = MA.y + 64 + MA.h + 20;
  return (
    <>
      <Browser
        url="cambridge.org/core/books/abs/multimedia-learning/multimedia-principle"
        src="t1-mayer-principle-crop.png"
        x={MA.x}
        y={MA.y}
        w={MA.w}
        h={MA.h}
        delay={T.maIn}
        out={T.maOut}
        dur={8}
        scale={sc}
        origin={`${960 - MA.x}px ${810 - MA.y}px`}
      >
        <Marker x={401} y={557} w={1518 - 401} h={594 - 557} delay={T.maMark} dur={T.maMarkEnd - T.maMark} pad={6} />
      </Browser>
      <LabelChip text="Richard E. Mayer · Multimedia Learning" left={MA.x} top={row} delay={T.maIn} out={T.maOut} />
      <PillRight right={MA.x + MA.w} top={row} domain="cambridge.org" delay={T.maIn} out={T.maOut} />
    </>
  );
};

/* ------------------------------------------------------------------ H: the docs */

const Docs: React.FC = () => (
  <>
    <WallCard x={220} y={196} w={640} h={720} delay={T.doIn} label="The docs" labelAt={T.doIn + 6} out={T.doOut} strikeAt={T.doStrike} />
    <div style={{ position: "absolute", left: 980, top: 430, width: 860 }}>
      <Headline text={"Don't read\nthe docs"} delay={T.doStrike} size={116} accent={["Don't"]} out={T.doOut} />
    </div>
  </>
);

/* ------------------------------------------------------------------ I: his prompt */

// v2: the crop (1520x690 at 1.1 = 1672 wide) sits inside a 1712-wide card with 20 px of its own background on each side
// (painted from the PNG's own uniform corner by CropFill, so it matches exactly), and the push (1.02) is centred on the content (title x 31 to bubble x 1489, y 52 to 635),
// so the title and the bubble keep ~37 px to the card border through f1508 (v1: ~5 px / ~10 px).
const QU = { x: 104, y: 110, w: 1712, s: 1.1, pad: 20 };
const QU_H = Math.round(690 * QU.s); // 759
const Prompt: React.FC = () => {
  const s = QU.s;
  return (
    <>
      <ShotCard x={QU.x} y={QU.y} w={QU.w} h={QU_H} delay={T.quIn} out={T.quOut} push={1.02} focus={[QU.pad + 760 * s, 343.5 * s]}>
        <CropFill src="t1-prompt-crop.png" srcW={1520} srcH={690} cw={24} ch={40} />
        <div style={{ position: "absolute", left: QU.pad, top: 0, width: 1520 * s, height: QU_H }}>
          <ShotLayer src="t1-prompt-crop.png" srcW={1520} srcH={690} s={s} top={0}>
            <BoxMarker box={[336, 311, 1424, 355]} s={s} delay={T.quMark} dur={T.quMarkEnd - T.quMark} out={T.quMarkOut} />
            <BoxMarker box={[1119, 437, 1296, 481]} s={s} delay={T.quClick1} dur={5} out={T.quOut} />
            <BoxMarker box={[334, 500, 894, 544]} s={s} delay={T.quClick2} dur={10} out={T.quOut} />
          </ShotLayer>
        </div>
      </ShotCard>
      <div style={{ position: "absolute", right: 1920 - (QU.x + QU.w), top: QU.y + QU_H + 20 }}>
        <OwnPill text="Samin's Claude app · his own prompt" delay={T.quIn} out={T.quOut} />
      </div>
    </>
  );
};

/* ------------------------------------------------------------------ J/K: the real page (B-roll) + close */

const BR = { x: 280, y: 30, w: 1360, h: 765 }; // clip px x 0.70833
const SEGS = [
  { from: 1520, to: 1684, trim: 0, fade: 0 },
  { from: 1680, to: 1750, trim: 229, fade: 4 },
  { from: 1746, to: 1816, trim: 363, fade: 4 },
  { from: 1812, to: 1884, trim: 499, fade: 4 },
  { from: 1878, to: 2035, trim: 795, fade: 6 },
];

const Footage: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.brIn) return null;
  const p = prog(f, T.brIn, 12);
  const t = prog(f, T.thumb, 18, EASE_IN_OUT);
  const sc = mix(t, 1, 880 / BR.w);
  const fx = mix(t, BR.x, 60);
  const fy = mix(t, BR.y, 293);
  const pillTop = fy + BR.h * sc + 18;
  const pillCx = fx + (BR.w * sc) / 2;
  const tail = tailOut(f); // v3: thumbnail and its pill leave with the CTA, f2024 to f2034
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: fx,
          top: fy,
          width: BR.w,
          height: BR.h,
          transform: `translateY(${(1 - p) * 50}px) scale(${sc * mix(p, 0.94, 1)})`,
          transformOrigin: "0 0",
          opacity: p * tail,
          filter: p < 0.99 ? `blur(${(1 - p) * 10}px)` : undefined,
        }}
      >
        <div style={{ position: "absolute", inset: 0, borderRadius: 28, overflow: "hidden", background: C.dark, boxShadow: velour.shadow.card }}>
          {SEGS.map((sg) => {
            if (f < sg.from || f >= sg.to) return null;
            const o = sg.fade ? prog(f, sg.from, sg.fade, EASE_IN_OUT) : 1;
            return (
              <div key={sg.from} style={{ position: "absolute", inset: 0, opacity: o }}>
                <Sequence from={sg.from} durationInFrames={sg.to - sg.from} layout="none">
                  <OffthreadVideo
                    src={staticFile("opus-tips/broll/geo-clickthrough.mp4")}
                    muted
                    trimBefore={sg.trim}
                    style={{ width: "100%", height: "100%", objectFit: "contain" }}
                  />
                </Sequence>
              </div>
            );
          })}
          {/* rings circumscribe the measured title glyphs (clip [689,465,887,544] / [1026,465,1234,544] x 0.70833) with the arrow
              left free between them and 6 px above the subtitle (local y 418.6) */}
          <Highlight x={558.2 - 86} y={357.4 - 52} w={172} h={104} kind="ring" stroke={5} pad={0} delay={T.brSeo} dur={10} out={T.brRingsOut} />
          <Highlight x={800.4 - 90} y={357.4 - 52} w={180} h={104} kind="ring" stroke={5} pad={0} delay={T.brGeo} dur={10} out={T.brRingsOut} />
          <Highlight x={1306} y={715} w={37} h={37} kind="ring" stroke={3} pad={7} delay={T.brNext} dur={8} out={T.brNext + 10} />
        </div>
      </div>
      <div style={{ position: "absolute", left: pillCx, top: pillTop, transform: "translateX(-50%)", opacity: tail }}>
        <OwnPill text="geo-vs-seo.html · built by Claude for Samin" delay={T.brIn + 4} />
      </div>
    </>
  );
};

/** Claude's real reply line, under the footage (never over it). */
const ReplyStrip: React.FC = () => {
  const f = useCurrentFrame();
  const p = prog(f, T.stIn, 10);
  const o = p * (1 - prog(f, T.stOut, 8, EASE_IN_OUT));
  if (o <= 0) return null;
  const s = 0.8;
  // v2: 14 px inner padding in the crop's own background (CropFill: the PNG's uniform corner, so no inner seam), so
  // 'I built' and 'browser.' no longer touch the border. The strip stays centred on x 960; it grows down, not up, so it keeps its gap under the B-roll pill.
  const pad = 14;
  const w = 1265 * s + 2 * pad;
  const h = 180 * s + 2 * pad;
  return (
    <div
      style={{
        position: "absolute",
        left: 960 - w / 2,
        top: 881 + (1 - p) * 20,
        width: w,
        height: h,
        borderRadius: 14,
        overflow: "hidden",
        background: C.bg,
        outline: `2px solid ${C.line}`,
        boxShadow: velour.shadow.soft,
        opacity: o,
      }}
    >
      <CropFill src="t1-reply-built-crop.png" srcW={1265} srcH={180} cw={20} ch={20} />
      <div style={{ position: "absolute", left: pad, top: pad, width: 1265 * s, height: 180 * s }}>
        <Img src={staticFile(P + "t1-reply-built-crop.png")} style={{ position: "absolute", left: 0, top: 0, width: 1265 * s, height: 180 * s, maxWidth: "none" }} />
        <BoxMarker box={[29, 128, 477, 174]} s={s} delay={T.stMark} dur={10} />
      </div>
    </div>
  );
};

const Close: React.FC = () => {
  const f = useCurrentFrame();
  const sub = prog(f, T.ctaSub, 12);
  const tail = tailOut(f);
  // v3: the whole 'Pick one thing / Try this today' group eases out f2024 to f2034 (opacity 0 on the last frame) with a
  // small 12 px settle upward, together with the GEO thumbnail, so the cut to T2's bare f0 canvas never blinks.
  return (
    <div style={{ position: "absolute", left: 1010, top: 0, width: 860, height: 1080, opacity: tail, transform: `translateY(${(1 - tail) * -12}px)` }}>
      <div style={{ position: "absolute", left: 0, top: 368 }}>
        <Headline text="Pick one thing." delay={T.ctaPick} size={104} accent={["one"]} />
      </div>
      <div
        style={{
          position: "absolute",
          left: 4,
          top: 496,
          fontFamily: FONT,
          fontSize: 40,
          fontWeight: 500,
          color: C.muted,
          whiteSpace: "nowrap",
          opacity: sub,
          transform: `translateY(${(1 - sub) * 14}px)`,
        }}
      >
        you've been putting off learning
      </div>
      <div style={{ position: "absolute", left: 0, top: 590 }}>
        <Headline text="Try this today." delay={T.ctaTry} size={104} accent={["today."]} />
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------ scene */

const Scene: React.FC = () => {
  const f = useCurrentFrame();
  const live = (from: number, to: number) => f >= from && f < to;
  return (
    <VelourCanvas tone="dark">
      {live(0, T.opOut + 10) ? <Opener /> : null}
      {live(T.prIn, T.prOut + 14) ? <Profile /> : null}
      {live(T.elIn, T.elOut + 14) ? <Eli5 /> : null}
      {live(T.kaIn, T.kaOut + 14) ? <Karpathy /> : null}
      {live(T.osIn, T.osOut + 14) ? <OpenStax /> : null}
      {live(T.waIn, T.waOut + 12) ? (
        <WallCard x={640} y={96} w={640} h={780} delay={T.waIn} label="A wall of text" labelAt={T.waLabel} out={T.waOut} exitX={-140} />
      ) : null}
      {live(T.maIn, T.maOut + 14) ? <Mayer /> : null}
      {live(T.tipKicker, T.tipOut + 10) ? <SectionKicker text="THE TIP" x={110} y={84} delay={T.tipKicker} out={T.tipOut} /> : null}
      {live(T.doIn, T.doOut + 14) ? <Docs /> : null}
      {live(T.quIn, T.quOut + 14) ? <Prompt /> : null}
      <Footage />
      {live(T.stIn, T.stOut + 10) ? <ReplyStrip /> : null}
      {f >= T.ctaPick ? <Close /> : null}
    </VelourCanvas>
  );
};

export default Scene;
