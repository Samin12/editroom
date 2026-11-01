import React from "react";
import { Img, interpolateColors, staticFile, useCurrentFrame } from "remotion";
import {
  VelourCanvas,
  C,
  prog,
  mix,
  geoZoom,
  velour,
  EASE_OUT,
  EASE_IN_OUT,
  Headline,
  ArrowTo,
  TweetCard,
  BrowserCard,
  SourcePill,
  Highlight,
  TipOpener,
} from "../../../styles/velour/src";

// anim-mvas6evb: TIP 6 "Your AI Operating System" (scene bc54e, BEHIND, project 406211 to 498620 ms, 2773 frames, f0 to f2772).
// Stage 1920x1080 scaled x0.768 into the left panel by VelourCanvas (the whole stage is ours; talking head only, no screen).
// Word times (project ms): tips exports/_work/writer/words/bc54e.json (final). at(ms) = round((ms - 406211) * 30 / 1000) - 3,
// so every beat lands ~3 frames before its word. Every change is sequenced: the old element leaves (or the stack moves)
// before the new one enters; no two texts share a spot at any frame. Section brief: receipts/briefs/T6.md.
// Proof copies: public/opus-tips/proof/anim-mvas6evb/ (05b-honcho-readme-t3crop.png = 05b crop [20,20,1620,562], T3's crop).
//
// BEATS (frame, word, what shows, proof file)
//   f1      "Okay"          TipOpener "TIP 6 / Your AI Operating System" (shared kit component, default x 80, y 60, as in T2 to
//                             T10; v2) + Thariq's X Article card (TweetCard, view [30,1350], w 1058, x 431, y 236), label row "Thariq · Claude Code
//                             @anthropicai" + pill x.com/trq212. proof 03-ai-os-t3code/01-thariq-skills-tweet.png (tip6-01).
//   f68     "And"           Opener leaves (gone f76); f80 to f98 card + row ease up 61 px (to y 175); push 1.00 to 1.03 (title) f80 to f198.
//   f120    "most"          Box on the article title "Lessons from Building Claude Code: How We Use Skills" (json [60,970,1653,1186];
//                             drawn as [46,984,1667,1198] so the stroke clears the "L" and the colon by ~9 stage px).
//   f186    (pause)         Card + row leave (gone f198).
//   f197    "stop"          Headline "Stop treating your agent like a chat box" builds word by word (his words, script spelling).
//   f255    "chatbox"       Plain chat window (one prompt bubble, one reply bubble, no text) labelled "Chat box".
//   f270    ("chatbox" end) v3: headline leaves (gone f282).
//   f283    "Treat"         Chat window shrinks left, dims, a thin strike draws; its "Chat box" label counter-scales so it stays
//                             >= 22 final px (v3). OS frame (menu bar, three windows, dock of neutral glyphs, no logos) grows in on
//                             the right. v3: headline "Treat it like an operating system" builds from f283 (was f292), so "Treat"
//                             reads by 415.734 (f286).  f309 "operating": OS frame lights.
//   f364    "dude"          Generic agent figure (rounded square, two dot eyes; not a mascot) walks into the OS desktop.
//   f375    "lives"         Label "Lives inside your computer" under the frame.
//   f413    ("And")         Chat window + label leave (gone f425); f416 to f446 the OS frame (content inside) glides to centre,
//                             grows 10%, then drifts while the figure idles (blink + bob). f460 headline leaves (gone f472).
//   f472    "AI"            Title "AI operating system" lands in the frame's title bar.
//   f551    "built"         The frame (outline, windows, dock, figure) collapses into slab 1 (number + memory-chip glyph from its
//                             first frame); title rides up above the stack. Slabs rise: 2 f563 "out", 3 f573 "of", 4 f579 "four".
//   f593    "layers"        Tag "4 layers" right of the stack (his words).
//   f642    "context"       Slab 1 lights, label "Context".  Chips "Memory" f680, "Files" f701, "Knowledge" f720 (his words).
//   f732    ("knowledge,")  v3: chips fade f732 to f740 (gone before the slab shrinks).  f736 title and tag leave (gone f748).
//   f740    ("which")       The stack tucks into the left rail f740 to f752 (Context lit, rest 40%). v3: the slab name cross-fades
//                             (out in its big spot, in at its rail spot) instead of reflowing past the glyph.
//   f750    "which"         Callback: tag "From Tip 3" + Thariq auto-memory post (T3's TweetCard crop view [36,548], w 1482,
//                             x 374, y 297), label row. proof 02-memory-handoff-prompts/01-thariq-automemory-tweet.png (tip3-01).
//   f761    "covered"       Marker on "remembers what it learns across sessions" (T3's draw box [314,357,1177,400], pad 0).
//   f854    ("connectors")  Card leaves (gone f866).
//   f866    "connectors"    Rail chip 2 lights + "Connectors". Agent node + six generic tool tiles (glyphs only, no brands);
//                             tiles plug in one by one f911 "our" to f956; all six light f966 "all".
//   f979    "tools"         Label "Access to all our tools".  f1012 diagram leaves (gone f1024).
//   f1024   "Composio"      BrowserCard composio.dev (hero rows 0 to 640: WORKS WITH bar, headline, subline), pill.
//                             proof 03-ai-os-t3code/04-composio-tools-crop.png (tip6-04).
//   f1073   "to" / "1500"   Box on the page's own "to 1,500+ apps" (measured [212,320,1054,462]), complete f1083.
//   f1104   ("that")        Card leaves (gone f1116).  f1116 BrowserCard composio.dev/toolkits (light page, dimmed 12%).
//                             proof 03-ai-os-t3code/05c-composio-toolkits-count-tight-crop.png (tip6-05).
//   f1147   "natively"      Box (not a ring) on "All toolkits 1613" (measured [82,208,768,270]).
//   f1183   "Third"         Rail chip 3 lights + "Skills".  f1194 toolkits card leaves (gone f1206).
//   f1206   "is"            BrowserCard code.claude.com/docs/en/skills. proof 03-ai-os-t3code/06-skills-docs-dark-crop.png (tip6-06).
//   f1214   "skills"        Box on the title "Extend Claude with skills" (v3: drawn [28,104,1368,236], ~25 source px clear of the ink).
//   f1272   "recipes"       Box fades; marker on "Create a SKILL.md file with instructions".  f1295 card leaves (gone f1307).
//   f1307   "loads"         BrowserCard platform.claude.com Agent Skills overview (left-aligned window, right-edge fade).
//                             proof 03-ai-os-t3code/06b-agent-skills-overview-crop.png (tip6-06b). f1317 marker "Skills load on demand,".
//   f1379   "re-explain"    Marker runs on over "so you don't have to repeat the same guidance across conversations." (to f1397).
//   f1414   ("And")         Card leaves (gone f1426).
//   f1423   "And fourth"    Rail chip 4 lights + "Routines"; BrowserCard code.claude.com/docs/en/routines rows 0 to 1050 (banner
//                             "Routines are in research preview" in view). proof 03-ai-os-t3code/07-routines-docs-dark-crop.png (tip6-07).
//   f1446   "routines"      Box on the title "Automate work with routines" (v3: drawn [27,104,1552,236]).
//   f1486   ("skills")      Routines card steps back (scale 0.96, fades) and is gone f1498; chip 3 lights again.
//   f1496   "skills"        The identical Skills docs card returns.  f1548 "over": marker on "keep pasting the same instructions,";
//                             rail chip 3 pulses f1548, f1560 "over", f1585 "constantly".  f1612 card leaves (gone f1624).
//   f1624   "put"           Rail arrow chip 3 -> chip 4 (chip 4 lit); routines card steps forward on rows 480 to 1530 (banner still
//                             in view).  f1653 "recurring": marker on "Scheduled: run on a recurring cadence like hourly, nightly,
//                             or weekly" (complete f1669).  f1704 card leaves (gone f1716).
//   f1714   "the morning"   T5's BrowserCard code.claude.com/docs/en/desktop-scheduled-tasks (rows 0 to 900). Two-segment marker
//                             "morning" f1724 + "briefings that pull from your calendar and inbox." f1729 to f1736.
//                             proof 02-memory-handoff-prompts/13b-desktop-scheduled-tasks-docs-crop.png (tip5-13b).
//   f1772   "proactive"     TagChip (moon) "Proactive nights" (his words, unsourced, styled as a chip not a docs line).
//   f1796   ("if")          13b card, pill, chip leave (gone f1808).
//   f1808   "if"            Boris Cherny's post (TweetCard view [0,1042], w 1477, x 377, y 50), label "Boris Cherny · creator of
//                             Claude Code (his own words)" + pill x.com/bcherny. Push 1.00 to 1.03 f1808 to f1971 (centre).
//                             proof 03-ai-os-t3code/02-boris-automating-tweet.png (tip6-02).
//   f1853   "automatically" Two-segment marker "the best engineers I knew spent" / "a lot of time automating their work".
//   f1883   "then"          The post scrolls to rows 631 to 1673 (to f1919).
//   f1929   "easier"/"runs" Marker on ONLY "that class of issue can be fully" / "automated forever." (complete f1950).
//                             "routine" is never marked.
//   f1971   "own"           Boris, his row and the rail leave (gone f1983).
//   f1984   "And"           Mindset card (his words, unattributed: no name, face, handle or quote marks): kicker "The mindset".
//   f2019   "mindset"       Accent rule draws under the kicker.
//   f2066   "turn"          Card grows; three workflow cards deal in with a repeat glyph.  f2101 "workflows": label "Workflows".
//   f2119   "into"          Arrow draws to an "Agents" node (the same agent glyph), lands f2128 "agents".
//   f2158   "more"          Card grows; line "More time for the work only you can do".  f2205 "only": accent underline on "only you".
//   f2232   ("So")          Card leaves (gone f2244).
//   f2244   "So"            The stack returns big (slabs 600x120, x 260 to 860, y 170/330/490/650, named), rising bottom to top
//                             f2244 to f2274. Check items are 46 px at x 920.
//   f2259   "for"           Header "For this step".  (Nothing for "make sure you watch this video".)
//   f2335   "install"       Check "Install Composio" beside slab 2 (slab 2 lights).
//   f2374   "connect"       Check "Connect your connectors" beside slab 2.
//   f2410   "write"         Check "Skills for workflows you repeat" beside slab 3 (slab 3 lights).
//   f2508   "set"           Check "Set up routines" beside slab 4 (slab 4 lights).  f2575 "background": sub-chip "Work in the
//                             background" with a slowly turning clock glyph.
//   f2604   ("Honcho")      Checklist dims to 30%, then hides as the README card turns opaque (so it never reads through).
//   f2614   "Honcho"        T3's Honcho README card rises (BrowserCard chrome copied from the kit, github.com/plastic-labs/honcho,
//                             x 64, y 160, w 1792, h 607; y 160 so it covers the whole checklist). proof
//                             02-memory-handoff-prompts/05b-honcho-readme-crop.png (tip3-05), T3's crop [20,20,1620,562] at 1.12.
//   f2642   "memory"        Marker on "Honcho is memory infrastructure".
//   f2663   ("underneath")  v3: the README card (and its pill) fades out while easing toward the base slot, fully gone f2673,
//                             BEFORE the checklist brightens (f2672 to f2684). It is never drawn over the stack.
//   f2674   "underneath"    v3: base slab "Honcho memory" (database glyph) rises in BELOW slab 1 (top edge >= y 810 at every
//                             frame; Context's bottom is y 770), settled ~f2687.
//   f2688   "system"        It locks in under slab 1 (accent + small bump scaled from its top edge; slab 1 lights).
//   f2720   "runs"          A soft light sweep runs up through all five slabs (to f2755), then holds.
//   f2760   ("efficiently") Tail: the whole "For this step" group (header, five slabs, checks, chip) fades + scales down 2% over
//                             f2760 to f2771 (v3: matches T7a.md REVISION 2 line 4); f2771 and f2772 are bare canvas, so the cut to
//                             T7a's bare f0 shows no one-frame vanish. NOTE: T6.md still says "no tail fade"; T7a.md (newer) expects
//                             this fade, so T6.md needs the same update (outside this job folder).
//
// Numbers on screen: "1,500+" (composio.dev hero), "1613" (composio.dev/toolkits, boxed), unhighlighted capture content (Thariq
// action bar 389/2.2K/16K/43K, toolkits category counts, "SHOWING 1-30 OF 1613", Honcho version badges), layer numbers 1 to 4,
// "4 layers", "Tip 3" (his words). No CountUp anywhere.
// Do not show (respected): any Hormozi name/face/handle/post, -tweet-page.png captures, auto-memory views row (row >= 567),
// 05-honcho-crop.png, any -highlight.png, the raw two-line Boris json box, a mark on Boris's "routine", a ring on "All toolkits
// 1613", the Composio hero below row 640, brand logos on connector tiles, a crab mascot, an Anthropic logo, 10,000 / 10% / 90%.

const JOB_START = 406211;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;

const T = {
  // opener + Thariq skills article
  openIn: 1,
  openOut: 68, // TipOpener fades 8 f, gone 76
  thIn: 1,
  thLift: 80,
  thBox: at(410300), // "most" 120
  thOut: 186, // gone 198
  // chat box -> operating system
  h1In: 197,
  chatIn: at(414820), // "chatbox." 255
  h1Out: 270, // v3: gone 282 ("chatbox." ends f270), so the old line is fully gone before the new one builds
  treat: at(415730), // "Treat" 283
  osIn: at(415730) + 8, // 291, after the chat window has moved left
  h2In: at(415730), // v3: 283, "Treat" (415.734, f286) lands on time (was 292)
  osLit: at(416620), // "operating" 309
  dude: at(418430), // "dude" 364
  lives: at(418810), // "lives" 375
  chatOut: 413, // gone 425
  glide: 416, // to 446
  h2Out: 460, // gone 472
  osTitle: at(422030), // "AI" 472
  morph: at(424670), // "built" 551, to 563
  slab2: at(425090), // "out" 563
  slab3: at(425400), // "of" 573
  slab4: at(425610), // "four" 579
  layers: at(426080), // "layers." 593
  context: at(427690), // "context." 642
  chipMem: at(428980), // 680
  chipFiles: at(429680), // 701
  chipKnow: at(430320), // 720
  bigOut: 736, // title, tag gone 748
  chipsOut: 732, // v3: chips fade f732 to f740, fully gone before the slab starts to shrink ("knowledge," ends f736)
  tuck: 740, // v3: to 752 (12 frames; was 738 to 752), starts only after the chips are gone
  // callback (Tip 3)
  memIn: 750,
  memMark: at(431680), // "covered" 761
  memOut: 854, // gone 866
  // connectors concept
  rail2: at(435250), // "connectors." 868
  connIn: 866,
  plug0: at(436690), // "our" 911
  allLit: at(438510), // "all" 966
  toolsLbl: at(438930), // "tools." 979
  connOut: 1012, // gone 1024
  // Composio
  compIn: 1024,
  compBox: at(442090), // "to" 1073, complete on "1500" 1083
  compOut: 1104, // gone 1116
  tkIn: 1116,
  tkBox: at(444550), // "natively" 1147
  rail3: at(445750), // "Third" 1183
  tkOut: 1194, // gone 1206
  // skills
  skIn: 1206,
  skBox: 1214, // "skills." 1216
  skMark: at(448720), // "recipes" 1272
  skOut: 1295, // gone 1307
  ovIn: 1307,
  ovMark1: 1317, // "loads" 1320
  ovMark2: at(452280), // "re-explain" 1379
  ovOut: 1414, // gone 1426
  // routines
  rail4: at(453740), // "And" (fourth) 1423
  rtIn: 1424,
  rtBox: at(454520), // "routines." 1446
  rtOut: 1486, // steps back, gone 1498
  rail3b: 1497, // chip 3 lit again ("skills" 1503)
  sk2In: 1496,
  sk2Mark: at(457920), // "over" 1548
  pulse2: at(458310), // "over" 1560
  pulse3: at(459150), // "constantly" 1585
  sk2Out: 1612, // gone 1624
  railArrow: at(460440), // "put" 1624
  rt2In: 1624,
  rt2Mark: at(461410), // "recurring" 1653, complete 1669 ("schedule" 1671)
  rt2Out: 1704, // gone 1716
  // morning briefing (T5's 13b card)
  sbIn: 1714,
  sbMark1: 1724, // "morning" 1729
  sbMark2: 1729, // complete 1736 ("briefing" 1738)
  nights: at(465390), // "proactive" 1772
  sbOut: 1796, // gone 1808
  // Boris
  boIn: 1808,
  boMark1: at(468090), // "automatically" 1853
  boScroll: at(469070), // "then" 1883
  boMark2: 1929,
  boMark2b: 1940, // complete 1950 ("runs" 1953)
  boOut: 1971, // "own." gone 1983 (rail too)
  // mindset (his line, unattributed)
  mdIn: 1984,
  mdRule: at(473620), // "mindset" 2019
  mdTurn: at(475180), // "turn" 2066
  mdWf: at(476340), // "workflows" 2101
  mdArrow: at(476940), // "into" 2119
  mdAgents: at(477260), // "agents," 2128
  mdMore: at(478240), // "more" 2158
  mdOnly: at(479820), // "only" 2205
  mdOut: 2232, // gone 2244
  // checklist
  ckIn: 2244,
  ckHead: at(481610), // "for" 2259
  ck1: at(484150), // "install" 2335
  ck2: at(485430), // "connect" 2374
  ck3: at(486630), // "write" 2410
  ck4: at(489920), // "set" 2508
  ckBg: at(492160), // "background" 2575
  ckDim: 2604,
  hoIn: 2614,
  hoMark: at(494380), // "memory" 2645 -> 2642
  hoMorph: at(495230), // "underneath" 2668
  hoOut: 2663, // v3: README card fades (and eases toward the base slot) f2663 to f2673, fully out before the stack brightens
  ckBack: 2672, // v3: checklist back to full brightness f2672 to f2684, ease-out (0 at f2672; the card is < 1% there, 0 at f2673)
  baseIn: 2674, // v3: base slab "Honcho memory" rises in BELOW slab 1 (top edge >= y 810 at every frame), settled ~f2687
  hoLock: at(495900), // "system" 2688
  sweep: 2720, // to 2755
  ckEnd: 2760, // v3: the whole "For this step" group fades + scales down 2% over f2760 to f2771 (T7a.md REVISION 2 wording); f2771 and f2772 are bare canvas
};

const P = "opus-tips/proof/anim-mvas6evb/";
const FONT = velour.font.display;
const CAPTURED = "Oct 16, 2026";
const THARIQ = "Thariq · Claude Code @anthropicai";
const BORIS = "Boris Cherny · creator of Claude Code (his own words)";

/* ------------------------------------------------------------------ helpers */

const alpha = (hex: string, a: number) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
};
const blurOf = (v: number) => (v > 0.02 ? `blur(${v.toFixed(2)}px)` : undefined);
/** Opacity of something that enters at `delay` (dur) and leaves at `out` (outDur). */
const inOut = (f: number, delay: number, dur: number, out?: number, outDur = 12) =>
  prog(f, delay, dur) * (out === undefined ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT));
/** 0 -> 1 -> 0 bump over `dur` frames from `s`. */
const bump = (f: number, s: number, dur = 10) => {
  const t = Math.min(1, Math.max(0, (f - s) / dur));
  return Math.sin(Math.PI * t);
};

type Rect = { x: number; y: number; w: number; h: number };
const lerpRect = (a: Rect, b: Rect, t: number): Rect => ({ x: mix(t, a.x, b.x), y: mix(t, a.y, b.y), w: mix(t, a.w, b.w), h: mix(t, a.h, b.h) });

/** Slow push wrapper (stage-wide box, so the origin is a stage point). Keeps the label/pill row outside. */
const Push: React.FC<{ from: number; to: number; ox: number; oy: number; z1?: number; children?: React.ReactNode }> = ({ from, to, ox, oy, z1 = 1.03, children }) => {
  const f = useCurrentFrame();
  const z = geoZoom(prog(f, from, Math.max(1, to - from), EASE_IN_OUT), 1, z1);
  return <div style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, transform: `scale(${z})`, transformOrigin: `${ox}px ${oy}px` }}>{children}</div>;
};

/** Person label (left) + source pill (right) under a proof; or the pill alone. */
const LabelRow: React.FC<{ x: number; y: number; w: number; label?: string; domain: string; delay: number; out?: number }> = ({ x, y, w, label, domain, delay, out }) => {
  const f = useCurrentFrame();
  const o = inOut(f, delay, 10, out, 8);
  const [who, ...rest] = (label ?? "").split(" · ");
  return (
    <div style={{ position: "absolute", left: x, top: y, width: w, display: "flex", alignItems: "center", justifyContent: label ? "space-between" : "flex-start", gap: 20 }}>
      {label ? (
        <div style={{ fontFamily: FONT, fontSize: 30, fontWeight: 600, color: C.muted, opacity: o, whiteSpace: "nowrap", letterSpacing: "-0.01em" }}>
          <span style={{ color: C.inkSoft, fontWeight: 750 }}>{who}</span>
          {rest.length ? ` · ${rest.join(" · ")}` : ""}
        </div>
      ) : null}
      <SourcePill domain={domain} captured={CAPTURED} delay={delay} out={out} style={{ whiteSpace: "nowrap" }} />
    </div>
  );
};

/** T5-style TagChip: a small pill with a glyph and a >= 32 px label. */
const TagChip: React.FC<{ text: string; glyph: React.ReactNode; size?: number; style?: React.CSSProperties }> = ({ text, glyph, size = 32, style }) => (
  <div
    style={{
      display: "inline-flex",
      alignItems: "center",
      gap: size * 0.45,
      padding: `${size * 0.36}px ${size * 0.75}px`,
      borderRadius: 999,
      background: C.paper,
      border: `2px solid ${C.line}`,
      boxShadow: velour.shadow.soft,
      fontFamily: FONT,
      fontSize: size,
      fontWeight: 650,
      color: C.ink,
      whiteSpace: "nowrap",
      ...style,
    }}
  >
    {glyph}
    {text}
  </div>
);

/* ------------------------------------------------------------------ glyphs (neutral, no brands) */

type G = { size: number; color: string; sw?: number };
const Svg: React.FC<G & { children: React.ReactNode }> = ({ size, color, sw = 1.9, children }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" style={{ flex: "none", display: "block" }}>
    {children}
  </svg>
);
const ChipGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <rect x={6} y={6} width={12} height={12} rx={2} />
    <rect x={9.5} y={9.5} width={5} height={5} rx={1} />
    <path d="M9.5 2.5v3M14.5 2.5v3M9.5 18.5v3M14.5 18.5v3M2.5 9.5h3M2.5 14.5h3M18.5 9.5h3M18.5 14.5h3" />
  </Svg>
);
const PlugGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M9 2.5v4.5M15 2.5v4.5M6 7h12v3.5a6 6 0 0 1-12 0Z M12 16.5v5" />
  </Svg>
);
const RecipeGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <rect x={4.5} y={3} width={15} height={18} rx={2} />
    <path d="M8 7.5h8M8 11.5h8M8 15.5h5" />
  </Svg>
);
const ClockGlyph: React.FC<G & { turn?: number }> = ({ turn = 0, ...p }) => (
  <Svg {...p}>
    <circle cx={12} cy={12} r={9} />
    <path d="M12 12V7" transform={`rotate(${turn} 12 12)`} />
    <path d="M12 12l3.5 2" />
  </Svg>
);
const MailGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <rect x={3} y={5} width={18} height={14} rx={2} />
    <path d="M3.8 6.5 12 13l8.2-6.5" />
  </Svg>
);
const CalendarGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <rect x={3.5} y={5} width={17} height={15.5} rx={2} />
    <path d="M3.5 10h17M8 3v4M16 3v4" />
  </Svg>
);
const DocGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M6 3h8l4 4v14H6Z M14 3v4h4 M9 12h6M9 16h6" />
  </Svg>
);
const ChatGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M4 5.5h16a1.5 1.5 0 0 1 1.5 1.5v9a1.5 1.5 0 0 1-1.5 1.5H10l-4.5 3.5v-3.5H4A1.5 1.5 0 0 1 2.5 16V7A1.5 1.5 0 0 1 4 5.5Z" />
  </Svg>
);
const CodeGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M8.5 8 4.5 12l4 4M15.5 8l4 4-4 4M13.5 6l-3 12" />
  </Svg>
);
const FolderGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
  </Svg>
);
const MoonGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5Z" />
  </Svg>
);
const RepeatGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <path d="M17 2.5l3 3-3 3M4 11.5v-2a4 4 0 0 1 4-4h12M7 21.5l-3-3 3-3M20 12.5v2a4 4 0 0 1-4 4H4" />
  </Svg>
);
const DatabaseGlyph: React.FC<G> = (p) => (
  <Svg {...p}>
    <ellipse cx={12} cy={5.5} rx={8} ry={3} />
    <path d="M4 5.5v13c0 1.66 3.58 3 8 3s8-1.34 8-3v-13M4 12c0 1.66 3.58 3 8 3s8-1.34 8-3" />
  </Svg>
);
const CheckIcon: React.FC<{ size: number; t: number }> = ({ size, t }) => (
  <svg width={size} height={size} viewBox="0 0 48 48" style={{ flex: "none", display: "block" }}>
    <circle cx={24} cy={24} r={22} fill={C.accent} />
    <path d="M14 24.5 21 31.5 34.5 17" fill="none" stroke={C.white} strokeWidth={5} strokeLinecap="round" strokeLinejoin="round" strokeDasharray={32} strokeDashoffset={32 * (1 - t)} />
  </svg>
);

const SLAB_GLYPH: React.FC<G>[] = [ChipGlyph, PlugGlyph, RecipeGlyph, ClockGlyph];
const SLAB_NAME = ["Context", "Connectors", "Skills", "Routines"];

/** The generic agent figure: a rounded square with two dot eyes (blinks, optional bob). Not a mascot. */
const AgentFigure: React.FC<{ size: number; f: number; lit?: number }> = ({ size, f, lit = 0 }) => {
  const ph = f % 36;
  const blink = ph < 4 ? 1 - Math.sin((Math.PI * ph) / 4) * 0.9 : 1;
  const eye = size * 0.13;
  return (
    <div
      style={{
        position: "relative",
        width: size,
        height: size,
        borderRadius: size * 0.3,
        background: C.inkSoft,
        boxShadow: `0 0 0 ${Math.max(3, size * 0.035)}px ${alpha(C.accent, 0.25 + 0.75 * lit)}, ${velour.shadow.soft}`,
      }}
    >
      {[0.33, 0.67].map((cx) => (
        <div
          key={cx}
          style={{
            position: "absolute",
            left: size * cx - eye / 2,
            top: size * 0.42 - eye / 2,
            width: eye,
            height: eye,
            borderRadius: eye,
            background: C.bg,
            transform: `scaleY(${blink})`,
          }}
        />
      ))}
      <div style={{ position: "absolute", left: size * 0.36, top: size * 0.64, width: size * 0.28, height: Math.max(3, size * 0.045), borderRadius: 9, background: alpha(C.bg, 0.55) }} />
    </div>
  );
};

/* ------------------------------------------------------------------ the stack (his 4-layer model) */

/** Content of one slab. r = 0 big layout (number, glyph, name in a row), 1 = rail layout (number + glyph over the name). */
const SlabBody: React.FC<{ n: number; r: number; lit: number; nameO: number; children?: React.ReactNode }> = ({ n, r, lit, nameO, children }) => {
  const Glyph = SLAB_GLYPH[n - 1];
  const bs = mix(r, 60, 40);
  const gs = mix(r, 54, 38);
  // v3: the name never reflows while the glyph moves. It fades out in its big-layout spot (r 0 to 0.3) and fades in at its
  // rail spot (r 0.7 to 1), so it is at opacity 0 through the middle of the tuck and never touches the glyph.
  const c01 = (v: number) => Math.min(1, Math.max(0, v));
  const names: { o: number; left: number; fs: number; cy: number }[] = [
    { o: nameO * (1 - c01(r / 0.3)), left: 182, fs: 46, cy: 60 },
    { o: nameO * c01((r - 0.7) / 0.3), left: 20, fs: 32, cy: 88 },
  ];
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: mix(r, 26, 18),
          top: mix(r, 30, 14),
          width: bs,
          height: bs,
          borderRadius: bs,
          background: interpolateColors(lit, [0, 1], [C.dark, C.accent]),
          color: interpolateColors(lit, [0, 1], [C.inkSoft, C.white]),
          fontFamily: FONT,
          fontSize: bs * 0.5,
          fontWeight: 800,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        {n}
      </div>
      <div style={{ position: "absolute", left: mix(r, 104, 70), top: mix(r, 33, 15) }}>
        <Glyph size={gs} color={interpolateColors(lit, [0, 1], [C.muted, C.accent])} />
      </div>
      {names.map((nm, i) =>
        nm.o > 0 ? (
          <div
            key={i}
            style={{
              position: "absolute",
              left: nm.left,
              top: nm.cy - nm.fs * 0.62,
              fontFamily: FONT,
              fontSize: nm.fs,
              fontWeight: 750,
              letterSpacing: "-0.02em",
              lineHeight: 1.24,
              color: C.ink,
              whiteSpace: "nowrap",
              opacity: nm.o,
              transform: `translateX(${(1 - nameO) * -12}px)`,
            }}
          >
            {SLAB_NAME[n - 1]}
          </div>
        ) : null,
      )}
      {children}
    </>
  );
};

/** One slab container at `rect`. */
const Slab: React.FC<{ rect: Rect; n: number; r: number; lit: number; nameO: number; opacity?: number; dy?: number; scale?: number; blur?: number; children?: React.ReactNode }> = ({
  rect,
  n,
  r,
  lit,
  nameO,
  opacity = 1,
  dy = 0,
  scale = 1,
  blur = 0,
  children,
}) => (
  <div
    style={{
      position: "absolute",
      left: rect.x,
      top: rect.y,
      width: rect.w,
      height: rect.h,
      borderRadius: 20,
      overflow: "hidden",
      boxSizing: "border-box",
      background: interpolateColors(lit, [0, 1], [C.paper, C.accentSoft]),
      border: `2.5px solid ${interpolateColors(lit, [0, 1], [C.line, C.accent])}`,
      boxShadow: velour.shadow.soft,
      opacity,
      transform: `translateY(${dy}px) scale(${scale})`,
      filter: blurOf(blur),
    }}
  >
    <SlabBody n={n} r={r} lit={lit} nameO={nameO}>
      {children}
    </SlabBody>
  </div>
);

const BIG: Rect[] = [710, 570, 430, 290].map((y) => ({ x: 510, y, w: 900, h: 120 }));
const RAIL: Rect[] = [687, 549, 411, 273].map((y) => ({ x: 48, y, w: 270, h: 120 }));
const CHECK: Rect[] = [650, 490, 330, 170].map((y) => ({ x: 260, y, w: 600, h: 120 }));
const BASE: Rect = { x: 260, y: 810, w: 600, h: 120 };

/** Which rail chip is active, as smooth 0..1 per slab. */
const ACTIVE: [number, number][][] = [
  [[T.context, T.rail2]],
  [[T.rail2, T.rail3]],
  [
    [T.rail3, T.rail4],
    [T.rail3b, T.railArrow],
  ],
  [
    [T.rail4, T.rail3b],
    [T.railArrow, 99999],
  ],
];
const litOf = (f: number, n: number) => Math.min(1, ACTIVE[n - 1].reduce((s, [a, b]) => s + prog(f, a, 8) * (1 - prog(f, b, 8)), 0));
const NAME_AT = [T.context, T.rail2, T.rail3, T.rail4];

/** Slab 1's chips "Memory", "Files", "Knowledge" (his words), in the big layout only. */
const ContextChips: React.FC<{ f: number }> = ({ f }) => {
  const items: [string, number][] = [
    ["Memory", T.chipMem],
    ["Files", T.chipFiles],
    ["Knowledge", T.chipKnow],
  ];
  const o = 1 - prog(f, T.chipsOut, 8, EASE_IN_OUT); // v3: gone at f740, before the slab starts to shrink
  return (
    <div style={{ position: "absolute", left: 388, top: 0, height: 120, display: "flex", alignItems: "center", gap: 12, opacity: o }}>
      {items.map(([t, a]) => {
        const p = prog(f, a, 10);
        const s = p === 0 ? 0.6 : mix(p, 0.6, 1) + 0.06 * bump(f, a + 4, 8);
        return (
          <div
            key={t}
            style={{
              opacity: Math.min(1, p * 2),
              transform: `scale(${s})`,
              padding: "8px 18px",
              borderRadius: 999,
              background: C.dark,
              border: `2px solid ${alpha(C.accent, 0.6)}`,
              fontFamily: FONT,
              fontSize: 30,
              fontWeight: 650,
              color: C.ink,
              whiteSpace: "nowrap",
            }}
          >
            {t}
          </div>
        );
      })}
    </div>
  );
};

/** The spine from f563 to f1983: big stack, then the left rail. Slab 1 before f563 is the OS frame morph (OsFrame). */
const StackA: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.morph || f > T.boOut + 13) return null;
  const tuck = prog(f, T.tuck, 12, EASE_IN_OUT);
  const exitO = 1 - prog(f, T.boOut, 12, EASE_IN_OUT);
  const rise = [T.morph, T.slab2, T.slab3, T.slab4];
  return (
    <>
      {[1, 2, 3, 4].map((n) => {
        if (n === 1 && f < T.morph + 12) return null;
        const p = n === 1 ? 1 : prog(f, rise[n - 1], 12);
        if (p <= 0) return null;
        const lit = litOf(f, n);
        const dimO = mix(tuck, 1, mix(lit, 0.4, 1));
        const pulse = n === 3 ? 0.05 * (bump(f, T.sk2Mark) + bump(f, T.pulse2) + bump(f, T.pulse3)) : 0;
        return (
          <Slab
            key={n}
            rect={lerpRect(BIG[n - 1], RAIL[n - 1], tuck)}
            n={n}
            r={tuck}
            lit={lit}
            nameO={prog(f, NAME_AT[n - 1], 10)}
            opacity={p * dimO * exitO}
            dy={(1 - p) * 34}
            scale={mix(p, 0.96, 1) * (1 + pulse)}
            blur={(1 - p) * 8}
          >
            {n === 1 && f < T.chipsOut + 8 ? <ContextChips f={f} /> : null}
          </Slab>
        );
      })}
      {/* "4 layers" tag (his words) */}
      {f < T.bigOut + 12 ? (
        <div
          style={{
            position: "absolute",
            left: 1446,
            top: 528,
            opacity: inOut(f, T.layers, 10, T.bigOut),
            transform: `translateX(${(1 - prog(f, T.layers, 10)) * -16}px)`,
            padding: "12px 26px",
            borderRadius: 999,
            background: C.accentSoft,
            border: `2px solid ${C.accent}`,
            fontFamily: FONT,
            fontSize: 36,
            fontWeight: 750,
            color: C.accent,
            whiteSpace: "nowrap",
          }}
        >
          4 layers
        </div>
      ) : null}
      {/* rail arrow: skills flow into routines */}
      {f >= T.railArrow ? <ArrowTo x1={334} y1={498} x2={334} y2={326} bend={0.12} delay={T.railArrow} dur={14} width={5} out={T.rt2Out} /> : null}
    </>
  );
};

/* ------------------------------------------------------------------ chat box -> operating system -> slab 1 */

const OS_W = 1000;
const OS_H = 600;
const OS_BAR = 76;

const Skel: React.FC<{ w: number; c?: string; h?: number; style?: React.CSSProperties }> = ({ w, c = C.darkLine, h = 12, style }) => (
  <div style={{ width: w, height: h, borderRadius: h, background: c, ...style }} />
);

const MiniWindow: React.FC<{ x: number; y: number; w: number; h: number; lines: number[]; glyph?: React.ReactNode }> = ({ x, y, w, h, lines, glyph }) => (
  <div style={{ position: "absolute", left: x, top: y, width: w, height: h, borderRadius: 16, background: C.dark, border: `2px solid ${C.darkLine}`, overflow: "hidden", boxShadow: velour.shadow.soft }}>
    <div style={{ height: 34, display: "flex", alignItems: "center", gap: 8, padding: "0 14px", background: C.darkSoft }}>
      {[0, 1, 2].map((i) => (
        <div key={i} style={{ width: 10, height: 10, borderRadius: 5, background: C.faint }} />
      ))}
    </div>
    <div style={{ padding: 18, display: "flex", flexDirection: "column", gap: 12 }}>
      {glyph}
      {lines.map((l, i) => (
        <Skel key={i} w={l} />
      ))}
    </div>
  </div>
);

/** OS desktop content at base size 1000x600 (menu bar drawn by the frame). */
const OsDesktop: React.FC<{ f: number }> = ({ f }) => {
  const walk = prog(f, T.dude, 16, EASE_IN_OUT);
  const bob = walk < 1 ? -Math.abs(Math.sin(walk * Math.PI * 3)) * 16 : -Math.abs(Math.sin(((f - T.dude) / 30) * Math.PI)) * 6;
  const dock = [FolderGlyph, DocGlyph, ChatGlyph, CodeGlyph, MailGlyph, CalendarGlyph];
  return (
    <div style={{ position: "absolute", left: 0, top: OS_BAR, width: OS_W, height: OS_H - OS_BAR }}>
      <MiniWindow x={56} y={28} w={330} h={196} lines={[220, 250, 160]} />
      <MiniWindow x={330} y={84} w={350} h={190} lines={[270, 200, 230]} />
      <MiniWindow x={656} y={28} w={290} h={176} lines={[200, 150]} />
      {/* dock */}
      <div
        style={{
          position: "absolute",
          left: OS_W / 2 - 250,
          top: 424,
          width: 500,
          height: 84,
          borderRadius: 26,
          background: alpha(C.dark, 0.92),
          border: `2px solid ${C.darkLine}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-around",
          padding: "0 18px",
          boxSizing: "border-box",
        }}
      >
        {dock.map((Gl, i) => (
          <div key={i} style={{ width: 56, height: 56, borderRadius: 14, background: C.darkSoft, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <Gl size={32} color={C.muted} />
          </div>
        ))}
      </div>
      {/* the agent figure walks in */}
      {f >= T.dude ? (
        <div style={{ position: "absolute", left: mix(walk, -150, 444), top: 296 + bob, opacity: Math.min(1, prog(f, T.dude, 6) * 1.2) }}>
          <AgentFigure size={112} f={f} lit={prog(f, T.lives, 10)} />
        </div>
      ) : null}
    </div>
  );
};

/** The chat window (diagram, not a product UI): prompt bubble, reply bubble, no text. Base 640x440. */
const ChatWindow: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.chatIn || f > T.chatOut + 13) return null;
  const p = prog(f, T.chatIn, 12);
  const m = prog(f, T.treat, 14, EASE_IN_OUT);
  const o = p * (1 - prog(f, T.chatOut, 12, EASE_IN_OUT)) * mix(m, 1, 0.45);
  const cx = mix(m, 960, 372);
  const cy = 588;
  const s = mix(m, 1.18, 0.62) * mix(p, 0.95, 1);
  const strike = prog(f, T.treat + 8, 12, EASE_IN_OUT);
  // v3: the "Chat box" label counter-scales as the window shrinks (34 -> 49.5 local px), so after the shrink it is
  // 49.5 x 0.62 = 30.7 stage px = 23.6 final px (>= 22 px floor). The title bar grows 70 -> 82 local px to fit it.
  const lblK = mix(m, 1, 1.455);
  const barH = mix(m, 70, 82);
  return (
    <div
      style={{
        position: "absolute",
        left: cx - 320,
        top: cy - 220 + (1 - p) * 30,
        width: 640,
        height: 440,
        transform: `scale(${s})`,
        transformOrigin: "50% 50%",
        opacity: o,
        filter: blurOf((1 - p) * 8),
      }}
    >
      <div style={{ position: "absolute", inset: 0, borderRadius: 26, background: C.paper, border: `2px solid ${C.line}`, overflow: "hidden", boxShadow: velour.shadow.card }}>
        <div style={{ height: barH, display: "flex", alignItems: "center", gap: 14, padding: "0 24px", background: C.dark, fontFamily: FONT, fontSize: 34 * lblK, fontWeight: 700, color: C.ink, whiteSpace: "nowrap" }}>
          <ChatGlyph size={34 * mix(m, 1, 1.3)} color={C.muted} />
          Chat box
        </div>
        {/* prompt bubble (right) */}
        <div style={{ position: "absolute", right: 28, top: 104, width: 300, padding: "20px 22px", borderRadius: 22, borderBottomRightRadius: 6, background: C.accentSoft, display: "flex", flexDirection: "column", gap: 12 }}>
          <Skel w={240} c={alpha(C.accent, 0.55)} />
          <Skel w={170} c={alpha(C.accent, 0.55)} />
        </div>
        {/* reply bubble (left) */}
        <div style={{ position: "absolute", left: 28, top: 218, width: 380, padding: "20px 22px", borderRadius: 22, borderBottomLeftRadius: 6, background: C.dark, display: "flex", flexDirection: "column", gap: 12 }}>
          <Skel w={320} />
          <Skel w={280} />
          <Skel w={190} />
        </div>
        {/* input */}
        <div style={{ position: "absolute", left: 28, right: 28, bottom: 24, height: 58, borderRadius: 29, background: C.bg, border: `2px solid ${C.line}` }} />
      </div>
      {/* thin strike */}
      <svg width={640} height={440} style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }}>
        <line x1={-10} y1={450} x2={-10 + 660 * strike} y2={450 - 460 * strike} stroke={C.accent} strokeWidth={7} strokeLinecap="round" opacity={strike > 0 ? 1 : 0} />
      </svg>
    </div>
  );
};

/** OS frame centre/scale before the morph. */
const osPose = (f: number) => {
  const g = prog(f, T.glide, 30, EASE_IN_OUT);
  const drift = geoZoom(prog(f, T.glide + 30, T.morph - T.glide - 30, EASE_IN_OUT), 1, 1.025);
  return { cx: mix(g, 1180, 960), cy: mix(g, 580, 560), s: mix(g, 0.92, 1.1) * drift };
};

/** OS frame: grows in at "Treat", glides to centre, then collapses into slab 1 (f551 to f563). */
const OsFrame: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.osIn || f >= T.morph + 12) return null;
  const p = prog(f, T.osIn, 14);
  const pose = osPose(f);
  const m = prog(f, T.morph, 12, EASE_IN_OUT);
  const r0: Rect = { x: pose.cx - (OS_W * pose.s) / 2, y: pose.cy - (OS_H * pose.s) / 2, w: OS_W * pose.s, h: OS_H * pose.s };
  const rect = lerpRect(r0, BIG[0], m);
  const lit = prog(f, T.osLit, 10) * (1 - m);
  // the desktop content collapses WITH the box (squashed to the slab), fading over the whole morph
  const sx = rect.w / OS_W;
  const sy = mix(m, rect.w / OS_W, rect.h / OS_H);
  const osO = 1 - prog(f, T.morph, 8, EASE_IN_OUT);
  const slabO = prog(f, T.morph + 5, 7);
  const entS = mix(p, 0.86, 1);
  return (
    <div
      style={{
        position: "absolute",
        left: rect.x,
        top: rect.y,
        width: rect.w,
        height: rect.h,
        borderRadius: mix(m, 24 * pose.s, 20),
        overflow: "hidden",
        boxSizing: "border-box",
        background: C.paper,
        border: `${mix(m, 3, 2.5)}px solid ${interpolateColors(lit, [0, 1], [C.line, C.accent])}`,
        boxShadow: `${velour.shadow.card}, 0 0 ${60 * lit}px ${alpha(C.accent, 0.25 * lit)}`,
        opacity: p,
        transform: `scale(${entS})`,
        filter: blurOf((1 - p) * 8),
      }}
    >
      <div style={{ position: "absolute", left: 0, top: 0, width: OS_W, height: OS_H, transform: `scale(${sx}, ${sy})`, transformOrigin: "0 0", opacity: osO }}>
        {/* menu bar */}
        <div style={{ position: "absolute", left: 0, top: 0, width: OS_W, height: OS_BAR, background: C.dark, display: "flex", alignItems: "center", gap: 10, padding: "0 24px", boxSizing: "border-box" }}>
          {[0, 1, 2].map((i) => (
            <div key={i} style={{ width: 14, height: 14, borderRadius: 7, background: C.faint }} />
          ))}
        </div>
        <OsDesktop f={f} />
      </div>
      {slabO > 0 ? (
        <div style={{ position: "absolute", inset: 0, opacity: slabO }}>
          <SlabBody n={1} r={0} lit={0} nameO={0} />
        </div>
      ) : null}
    </div>
  );
};

/** Title "AI operating system": lands in the OS title bar at "AI", rides up above the stack at "built". */
const OsTitle: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.osTitle || f > T.bigOut + 12) return null;
  const pose = osPose(Math.min(f, T.morph));
  const barCy = pose.cy - (OS_H * pose.s) / 2 + (OS_BAR * pose.s) / 2;
  const up = prog(f, T.morph, 14, EASE_IN_OUT);
  const cy = mix(up, barCy, 205);
  const size = mix(up, 52 * pose.s, 62);
  const p = prog(f, T.osTitle, 10);
  const o = p * (1 - prog(f, T.bigOut, 12, EASE_IN_OUT));
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        width: 1920,
        top: cy - size * 0.62,
        textAlign: "center",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 800,
        letterSpacing: "-0.025em",
        lineHeight: 1.24,
        color: C.ink,
        opacity: o,
        transform: `translateY(${(1 - p) * 14}px)`,
        filter: blurOf((1 - p) * 6),
        whiteSpace: "nowrap",
      }}
    >
      AI operating system
    </div>
  );
};

const ConceptText: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.h1In || f > T.h2Out + 13) return null;
  const livesP = prog(f, T.lives, 10);
  return (
    <>
      {f < T.h1Out + 13 ? (
        <Headline
          text="Stop treating your agent like a chat box"
          delay={T.h1In}
          stagger={3}
          size={72}
          align="center"
          out={T.h1Out}
          style={{ position: "absolute", left: 0, top: 70, width: 1920 }}
        />
      ) : null}
      {f >= T.h2In - 1 ? (
        <Headline
          text="Treat it like an operating system"
          delay={T.h2In}
          stagger={3}
          size={72}
          align="center"
          accent={["operating", "system"]}
          out={T.h2Out}
          style={{ position: "absolute", left: 0, top: 70, width: 1920 }}
        />
      ) : null}
      {f >= T.lives && f < T.chatOut + 13 ? (
        <div
          style={{
            position: "absolute",
            left: 1180 - 500,
            width: 1000,
            top: 876,
            textAlign: "center",
            fontFamily: FONT,
            fontSize: 42,
            fontWeight: 700,
            color: C.inkSoft,
            opacity: livesP * (1 - prog(f, T.chatOut, 12, EASE_IN_OUT)),
            transform: `translateY(${(1 - livesP) * 14}px)`,
            filter: blurOf((1 - livesP) * 6),
          }}
        >
          Lives inside your computer
        </div>
      ) : null}
    </>
  );
};

/* ------------------------------------------------------------------ opener: Thariq's skills article */

const Opener: React.FC = () => {
  const f = useCurrentFrame();
  if (f > T.thOut + 13) return null;
  // v2: the shared opener sits at its default x 80, y 60 (as in T2 to T10), so the card starts at y 236 (clear of the
  // headline's descenders, the same top as T4 and T7a) and lifts 61 px to y 175 once the opener is gone.
  const lift = (TH_Y0 - TH_Y1) * prog(f, T.thLift, 18, EASE_IN_OUT);
  const x = 431;
  const w = 1058;
  const s = w / 1824;
  const cardH = (1350 - 30) * s;
  const y = TH_Y0 - lift;
  return (
    <>
      <TipOpener n={6} name="Your AI Operating System" delay={T.openIn} out={T.openOut} />
      <Push from={T.thLift} to={T.thOut + 12} ox={x + 856 * s} oy={TH_Y1 + (1078 - 30) * s}>
        <TweetCard
          src={`${P}01-thariq-skills-tweet.png`}
          srcW={1824}
          srcH={3900}
          x={x}
          y={y}
          w={w}
          view={[30, 1350]}
          delay={T.thIn}
          out={T.thOut}
          push={[1, 1]}
          highlights={[{ box: [46, 984, 1667, 1198], at: T.thBox, kind: "box", pad: 0 }]}
        />
      </Push>
      <LabelRow x={x} y={y + cardH + 14} w={w} label={THARIQ} domain="x.com/trq212" delay={T.thIn + 4} out={T.thOut} />
    </>
  );
};
const TH_Y0 = 236;
const TH_Y1 = 175;

/* ------------------------------------------------------------------ callback: Thariq auto-memory (T3's card) */

const Callback: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.memIn - 1 || f > T.memOut + 13) return null;
  const x = 374;
  const y = 297;
  const w = 1482;
  const h = (548 - 36) * (w / 1824);
  const tagP = inOut(f, T.memIn, 10, T.memOut);
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: x,
          top: 236,
          opacity: tagP,
          transform: `translateY(${(1 - prog(f, T.memIn, 10)) * 12}px)`,
          display: "inline-flex",
          alignItems: "center",
          gap: 12,
          padding: "8px 22px",
          borderRadius: 999,
          background: C.accentSoft,
          border: `2px solid ${alpha(C.accent, 0.7)}`,
          fontFamily: FONT,
          fontSize: 30,
          fontWeight: 750,
          color: C.accent,
          whiteSpace: "nowrap",
        }}
      >
        <ChipGlyph size={28} color={C.accent} />
        From Tip 3
      </div>
      <Push from={T.memIn} to={T.memOut + 12} ox={x + w / 2} oy={y + h / 2}>
        <TweetCard
          src={`${P}01-thariq-automemory-tweet.png`}
          srcW={1824}
          srcH={1827}
          x={x}
          y={y}
          w={w}
          view={[36, 548]}
          delay={T.memIn}
          out={T.memOut}
          push={[1, 1]}
          highlights={[{ box: [314, 357, 1177, 400], at: T.memMark, kind: "marker", pad: 0 }]}
        />
      </Push>
      <LabelRow x={x} y={y + h + 22} w={w} label={THARIQ} domain="x.com/trq212" delay={T.memIn + 4} out={T.memOut} />
    </>
  );
};

/* ------------------------------------------------------------------ connectors concept */

const AG = { x: 740, y: 540 };
const TILE_POS = [-66, -39.6, -13.2, 13.2, 39.6, 66].map((d) => {
  const a = (d * Math.PI) / 180;
  return { x: AG.x + 720 * Math.cos(a), y: AG.y + 330 * Math.sin(a) };
});
const TILE_GLYPH: React.FC<G>[] = [MailGlyph, CalendarGlyph, DocGlyph, ChatGlyph, CodeGlyph, FolderGlyph];

const Connectors: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.connIn || f > T.connOut + 13) return null;
  const o = 1 - prog(f, T.connOut, 12, EASE_IN_OUT);
  const ap = prog(f, T.connIn, 12);
  const all = prog(f, T.allLit, 10);
  const lbl = prog(f, T.toolsLbl, 10);
  const TS = 140;
  return (
    <div style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, opacity: o, filter: blurOf((1 - o) * 6) }}>
      {/* lines */}
      <svg width={1920} height={1080} style={{ position: "absolute", left: 0, top: 0 }}>
        {TILE_POS.map((t, i) => {
          const a = T.plug0 + i * 9;
          const lp = prog(f, a - 4, 10, EASE_IN_OUT);
          const x1 = AG.x + 104;
          const y1 = AG.y + (t.y - AG.y) * 0.2;
          const x2 = t.x - TS / 2 - 4;
          const y2 = t.y;
          const len = Math.hypot(x2 - x1, y2 - y1);
          return (
            <g key={i}>
              <line
                x1={x1}
                y1={y1}
                x2={x2}
                y2={y2}
                stroke={interpolateColors(all, [0, 1], [C.darkLine, C.accent])}
                strokeWidth={5}
                strokeLinecap="round"
                strokeDasharray={len}
                strokeDashoffset={len * (1 - lp)}
              />
              <circle cx={x2} cy={y2} r={8} fill={interpolateColors(all, [0, 1], [C.muted, C.accent])} opacity={lp >= 1 ? 1 : 0} />
            </g>
          );
        })}
      </svg>
      {/* agent node */}
      <div style={{ position: "absolute", left: AG.x - 100, top: AG.y - 100 + (1 - ap) * 30, opacity: ap, filter: blurOf((1 - ap) * 8) }}>
        <AgentFigure size={200} f={f} lit={all} />
        <div style={{ position: "absolute", left: -60, width: 320, top: 222, textAlign: "center", fontFamily: FONT, fontSize: 40, fontWeight: 750, color: C.ink }}>Agent</div>
      </div>
      {/* tiles */}
      {TILE_POS.map((t, i) => {
        const a = T.plug0 + i * 9;
        const p = prog(f, a, 10, (v) => v);
        const sb = 1.70158 * 1.2;
        const back = p === 0 ? 0 : 1 + (sb + 1) * Math.pow(p - 1, 3) + sb * Math.pow(p - 1, 2);
        const Gl = TILE_GLYPH[i];
        const lit = Math.max(all, 0.0);
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: t.x - TS / 2,
              top: t.y - TS / 2,
              width: TS,
              height: TS,
              borderRadius: 30,
              boxSizing: "border-box",
              background: interpolateColors(lit, [0, 1], [C.paper, C.accentSoft]),
              border: `3px solid ${interpolateColors(lit, [0, 1], [C.line, C.accent])}`,
              boxShadow: velour.shadow.soft,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              opacity: Math.min(1, p * 2),
              transform: `scale(${mix(back, 0.6, 1) * (1 + 0.06 * bump(f, T.allLit, 10))})`,
            }}
          >
            <Gl size={66} color={interpolateColors(lit, [0, 1], [C.inkSoft, C.accent])} sw={1.8} />
          </div>
        );
      })}
      {/* label (his words) */}
      <div
        style={{
          position: "absolute",
          left: 350,
          width: 1530,
          top: 60,
          textAlign: "center",
          fontFamily: FONT,
          fontSize: 62,
          fontWeight: 800,
          letterSpacing: "-0.03em",
          color: C.ink,
          opacity: lbl,
          transform: `translateY(${(1 - lbl) * 16}px)`,
          filter: blurOf((1 - lbl) * 6),
        }}
      >
        Access to <span style={{ color: C.accent }}>all</span> our tools
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------ proof cards (BrowserCards) */

/** Highlight in a BrowserCard's image area from a SOURCE box, display scale s, window origin (ox, oy) in source px. */
const HL: React.FC<{ box: [number, number, number, number]; s: number; ox?: number; oy?: number; delay: number; dur?: number; kind: "box" | "marker"; pad?: number; out?: number }> = ({
  box,
  s,
  ox = 0,
  oy = 0,
  delay,
  dur = 10,
  kind,
  pad = 0,
  out,
}) => (
  <Highlight x={(box[0] - ox) * s} y={(box[1] - oy) * s} w={(box[2] - box[0]) * s} h={(box[3] - box[1]) * s} delay={delay} dur={dur} kind={kind} pad={pad} out={out} />
);

const ComposioCards: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.compIn - 1 || f > T.tkOut + 13) return null;
  const cs = 1498 / 1920;
  const ks = 0.8;
  return (
    <>
      {f <= T.compOut + 13 ? (
        <>
          <Push from={T.compIn} to={T.compOut + 12} ox={1115} oy={505}>
            <BrowserCard url="composio.dev" src={`${P}04-composio-tools-crop.png`} x={366} y={224} w={1498} h={499} delay={T.compIn} out={T.compOut} push={[1, 1]} fit="cover" objectPosition="50% 0%">
              <HL box={[212, 320, 1054, 462]} s={cs} delay={T.compBox} dur={10} kind="box" />
            </BrowserCard>
          </Push>
          <LabelRow x={366} y={224 + 64 + 499 + 20} w={1498} domain="composio.dev" delay={T.compIn + 4} out={T.compOut} />
        </>
      ) : null}
      {f >= T.tkIn - 1 ? (
        <>
          <Push from={T.tkIn} to={T.tkOut + 12} ox={1115} oy={505}>
            <BrowserCard url="composio.dev/toolkits" src={`${P}05c-composio-toolkits-count-tight-crop.png`} x={385} y={125} w={1460} h={696} delay={T.tkIn} out={T.tkOut} push={[1, 1]} fit="cover" objectPosition="50% 0%">
              {/* light page: a light dim so it sits on the dark canvas */}
              <div style={{ position: "absolute", inset: 0, background: alpha(C.bg, 0.12) }} />
              <HL box={[82, 208, 768, 270]} s={ks} delay={T.tkBox} dur={10} kind="box" />
            </BrowserCard>
          </Push>
          <LabelRow x={385} y={125 + 64 + 696 + 22} w={1460} domain="composio.dev/toolkits" delay={T.tkIn + 4} out={T.tkOut} />
        </>
      ) : null}
    </>
  );
};

const SK = { x: 372, y: 50, w: 1485, s: 1485 / 2250, h: 1210 * (1485 / 2250) };

/** The Skills docs card (identical both times). */
const SkillsCard: React.FC<{ delay: number; out: number; children?: React.ReactNode }> = ({ delay, out, children }) => (
  <>
    <Push from={delay} to={out + 12} ox={1115} oy={SK.y + (64 + SK.h) / 2}>
      <BrowserCard url="code.claude.com/docs/en/skills" src={`${P}06-skills-docs-dark-crop.png`} x={SK.x} y={SK.y} w={SK.w} h={SK.h} delay={delay} out={out} push={[1, 1]} fit="cover" objectPosition="50% 0%">
        {children}
      </BrowserCard>
    </Push>
    <LabelRow x={SK.x} y={SK.y + 64 + SK.h + 18} w={SK.w} domain="code.claude.com" delay={delay + 4} out={out} />
  </>
);

const SkillsBeats: React.FC = () => {
  const f = useCurrentFrame();
  const ovS = 691 / 1080;
  return (
    <>
      {f >= T.skIn - 1 && f <= T.skOut + 13 ? (
        <SkillsCard delay={T.skIn} out={T.skOut}>
          {/* title ink x 53 to 1343, y 129 to 211 (json crop [52,103,1347,238]); v3: drawn with ~25 source px of margin on every
              side; the top edge (y 104 minus half-stroke) still clears the "SKILLS" eyebrow (ink ends ~y 70) by ~30 source px */}
          <HL box={[28, 104, 1368, 236]} s={SK.s} delay={T.skBox} dur={12} kind="box" out={T.skMark} />
          <HL box={[806, 528, 1754, 584]} s={SK.s} delay={T.skMark} dur={12} kind="marker" />
        </SkillsCard>
      ) : null}
      {f >= T.ovIn - 1 && f <= T.ovOut + 13 ? (
        <>
          <Push from={T.ovIn} to={T.ovOut + 12} ox={1115} oy={127 + (64 + 691) / 2}>
            <BrowserCard
              url="platform.claude.com/docs/en/agents-and-tools/agent-skills/overview"
              src={`${P}06b-agent-skills-overview-crop.png`}
              x={379}
              y={127}
              w={1472}
              h={691}
              delay={T.ovIn}
              out={T.ovOut}
              push={[1, 1]}
              fit="cover"
              objectPosition="0% 0%"
            >
              {/* right-edge fade in the capture's own background (sampled rgb 21,21,21), rows 0 to 976 only */}
              <div style={{ position: "absolute", left: 1402, top: 0, width: 70, height: 624, background: "linear-gradient(90deg, rgba(21,21,21,0) 0%, rgba(21,21,21,1) 100%)" }} />
              <HL box={[364, 979, 832, 1040]} s={ovS} delay={T.ovMark1} dur={12} kind="marker" />
              <HL box={[833, 979, 2275, 1040]} s={ovS} delay={T.ovMark2} dur={18} kind="marker" />
            </BrowserCard>
          </Push>
          <LabelRow x={379} y={127 + 64 + 691 + 18} w={1472} domain="platform.claude.com" delay={T.ovIn + 4} out={T.ovOut} />
        </>
      ) : null}
      {f >= T.sk2In - 1 && f <= T.sk2Out + 13 ? (
        <SkillsCard delay={T.sk2In} out={T.sk2Out}>
          <HL box={[577, 840, 1383, 901]} s={SK.s} delay={T.sk2Mark} dur={12} kind="marker" />
        </SkillsCard>
      ) : null}
    </>
  );
};

const RT = { x: 372, y: 92, w: 1485, h: 693, s: 1485 / 2250 };

/** Routines docs: rows 0 to 1050 first, then (after the skills card) rows 480 to 1530; the banner stays in view in both. */
const RoutinesBeats: React.FC = () => {
  const f = useCurrentFrame();
  return (
    <>
      {f >= T.rtIn - 1 && f <= T.rtOut + 13 ? (
        <RoutinesCard delay={T.rtIn} out={T.rtOut} pos="50% 0%" stepBack>
          {/* title ink x 52 to 1527 (json crop [52,103,1531,238]); v3: drawn with ~25 source px of margin on every side; the top
              edge still clears the "CLAUDE CODE IN THE CLOUD" eyebrow (ink ends ~y 70) by ~30 source px */}
          <HL box={[27, 104, 1552, 236]} s={RT.s} delay={T.rtBox} dur={12} kind="box" />
        </RoutinesCard>
      ) : null}
      {f >= T.rt2In - 1 && f <= T.rt2Out + 13 ? (
        <RoutinesCard delay={T.rt2In} out={T.rt2Out} pos="50% 100%">
          <HL box={[142, 1338, 1678, 1399]} s={RT.s} oy={480} delay={T.rt2Mark} dur={16} kind="marker" />
        </RoutinesCard>
      ) : null}
    </>
  );
};

const RoutinesCard: React.FC<{ delay: number; out: number; pos: string; stepBack?: boolean; children?: React.ReactNode }> = ({ delay, out, pos, stepBack, children }) => {
  const f = useCurrentFrame();
  // step back: the card shrinks to 0.96 while it fades (Reveal's own exit runs at the same time)
  const sb = stepBack ? prog(f, out, 12, EASE_IN_OUT) : 0;
  // step forward: enters from 0.96
  const sf = stepBack ? 1 : mix(prog(f, delay, 14), 0.96, 1);
  const cx = RT.x + RT.w / 2;
  const cy = RT.y + (64 + RT.h) / 2;
  return (
    <>
      <div style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, transform: `scale(${mix(sb, 1, 0.96) * sf})`, transformOrigin: `${cx}px ${cy}px` }}>
        <Push from={delay} to={out + 12} ox={cx} oy={cy}>
          <BrowserCard url="code.claude.com/docs/en/routines" src={`${P}07-routines-docs-dark-crop.png`} x={RT.x} y={RT.y} w={RT.w} h={RT.h} delay={delay} out={out} push={[1, 1]} fit="cover" objectPosition={pos}>
            {children}
          </BrowserCard>
        </Push>
      </div>
      <LabelRow x={RT.x} y={RT.y + 64 + RT.h + 18} w={RT.w} domain="code.claude.com" delay={delay + 4} out={out} />
    </>
  );
};

/** T5's 13b card + the "Proactive nights" chip. */
const MorningCard: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.sbIn - 1 || f > T.sbOut + 13) return null;
  const s = 1511 / 2290;
  const x = 360;
  const y = 150;
  const h = 594;
  const chipP = prog(f, T.nights, 12, (v) => v);
  const sb = 1.70158 * 1.2;
  const back = chipP === 0 ? 0 : 1 + (sb + 1) * Math.pow(chipP - 1, 3) + sb * Math.pow(chipP - 1, 2);
  const chipO = Math.min(1, chipP * 2) * (1 - prog(f, T.sbOut, 12, EASE_IN_OUT));
  return (
    <>
      <Push from={T.sbIn} to={T.sbOut + 12} ox={x + 1511 / 2} oy={y + (64 + h) / 2}>
        <BrowserCard url="code.claude.com/docs/en/desktop-scheduled-tasks" src={`${P}13b-desktop-scheduled-tasks-docs-crop.png`} x={x} y={y} w={1511} h={h} delay={T.sbIn} out={T.sbOut} push={[1, 1]} fit="cover" objectPosition="50% 0%">
          <HL box={[1840, 739, 2020, 786]} s={s} delay={T.sbMark1} dur={5} kind="marker" pad={6} />
          <HL box={[65, 823, 1134, 870]} s={s} delay={T.sbMark2} dur={7} kind="marker" pad={6} />
        </BrowserCard>
      </Push>
      <LabelRow x={x} y={y + 64 + h + 22} w={1511} domain="code.claude.com" delay={T.sbIn + 4} out={T.sbOut} />
      {f >= T.nights ? (
        <div style={{ position: "absolute", left: x + 1511, top: y + 64 + h + 18, transform: `translateX(-100%) scale(${mix(back, 0.6, 1)})`, transformOrigin: "100% 50%", opacity: chipO }}>
          <TagChip text="Proactive nights" glyph={<MoonGlyph size={34} color={C.accent} />} />
        </div>
      ) : null}
    </>
  );
};

/** Boris Cherny's post (the script's quick cut). */
const BorisCard: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.boIn - 1 || f > T.boOut + 13) return null;
  const x = 377;
  const y = 50;
  const w = 1477;
  const h = 1042 * (w / 1824);
  return (
    <>
      <Push from={T.boIn} to={T.boOut + 12} ox={x + w / 2} oy={y + h / 2}>
        <TweetCard
          src={`${P}02-boris-automating-tweet.png`}
          srcW={1824}
          srcH={3291}
          x={x}
          y={y}
          w={w}
          view={[0, 1042]}
          scroll={{ from: T.boScroll, to: T.boScroll + 36, toY: 631 }}
          delay={T.boIn}
          out={T.boOut}
          push={[1, 1]}
          highlights={[
            { box: [1095, 234, 1762, 292], at: T.boMark1, kind: "marker", pad: 0 },
            { box: [55, 294, 774, 352], at: T.boMark1 + 7, kind: "marker", pad: 0 },
            { box: [1112, 1194, 1743, 1252], at: T.boMark2, kind: "marker", pad: 0 },
            { box: [55, 1254, 454, 1303], at: T.boMark2b, kind: "marker", pad: 0 },
          ]}
        />
      </Push>
      <LabelRow x={x} y={y + h + 22} w={w} label={BORIS} domain="x.com/bcherny" delay={T.boIn + 4} out={T.boOut} />
    </>
  );
};

/* ------------------------------------------------------------------ mindset (his line, unattributed) */

const Mindset: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.mdIn || f > T.mdOut + 13) return null;
  const W = 1400;
  const X = 960 - W / 2;
  const p = prog(f, T.mdIn, 12);
  const o = p * (1 - prog(f, T.mdOut, 12, EASE_IN_OUT));
  const g1 = prog(f, T.mdTurn, 12, EASE_IN_OUT);
  const g2 = prog(f, T.mdMore, 12, EASE_IN_OUT);
  // compact (kicker only), grows as each element arrives; it stays centred on the stage, so the kicker moves up
  const H = 196 + 330 * g1 + 136 * g2;
  const TOP = 540 - H / 2;
  const rule = prog(f, T.mdRule, 14, EASE_IN_OUT);
  const wfLbl = prog(f, T.mdWf, 10);
  const agP = prog(f, T.mdAgents - 4, 12, (v) => v);
  const sb = 1.70158 * 1.2;
  const agBack = agP === 0 ? 0 : 1 + (sb + 1) * Math.pow(agP - 1, 3) + sb * Math.pow(agP - 1, 2);
  const more = prog(f, T.mdMore + 5, 10);
  const only = prog(f, T.mdOnly, 12, EASE_IN_OUT);
  // diagram centres (card-relative)
  const WF = { x: 440, y: 330 };
  const AGN = { x: 980, y: 330 };
  const lbl: React.CSSProperties = { position: "absolute", width: 400, top: 452, textAlign: "center", fontFamily: FONT, fontSize: 44, fontWeight: 750, color: C.ink };
  return (
    <div style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, opacity: o, filter: blurOf((1 - p) * 8 + (1 - o) * 4) }}>
      <div
        style={{
          position: "absolute",
          left: X,
          top: TOP + (1 - p) * 30,
          width: W,
          height: H,
          borderRadius: 34,
          overflow: "hidden",
          background: C.paper,
          border: `2px solid ${C.line}`,
          boxShadow: velour.shadow.card,
          transform: `scale(${mix(p, 0.97, 1)})`,
        }}
      >
        {/* kicker (his word) + accent rule */}
        <div style={{ position: "absolute", left: 0, width: W, top: 44, textAlign: "center", fontFamily: FONT, fontSize: 60, fontWeight: 800, letterSpacing: "-0.025em", lineHeight: 1.15, color: C.ink }}>
          The mindset
        </div>
        <div style={{ position: "absolute", left: W / 2 - 110, top: 130, width: 220, height: 6, borderRadius: 4, background: C.accent, transform: `scaleX(${rule})`, transformOrigin: "50% 50%" }} />
        {/* workflows: three identical cards dealt in, each with a repeat glyph */}
        {[2, 1, 0].map((i) => {
          const d = prog(f, T.mdTurn + (2 - i) * 4, 12);
          return (
            <div
              key={i}
              style={{
                position: "absolute",
                left: WF.x - 140 + i * 18,
                top: WF.y - 88 - i * 18 + (1 - d) * 40,
                width: 280,
                height: 176,
                borderRadius: 22,
                background: C.dark,
                border: `2px solid ${C.darkLine}`,
                opacity: d,
                boxShadow: velour.shadow.soft,
                padding: 24,
                boxSizing: "border-box",
                display: "flex",
                flexDirection: "column",
                gap: 14,
              }}
            >
              <RepeatGlyph size={46} color={C.accent} sw={2.1} />
              <Skel w={200} />
              <Skel w={140} />
            </div>
          );
        })}
        <div style={{ ...lbl, left: WF.x - 200, opacity: wfLbl, transform: `translateY(${(1 - wfLbl) * 10}px)` }}>Workflows</div>
        {/* agents node: the same agent glyph */}
        <div style={{ position: "absolute", left: AGN.x - 90, top: AGN.y - 90, opacity: Math.min(1, agP * 2), transform: `scale(${mix(agBack, 0.6, 1)})` }}>
          <AgentFigure size={180} f={f} lit={prog(f, T.mdAgents, 10)} />
        </div>
        <div style={{ ...lbl, left: AGN.x - 200, opacity: prog(f, T.mdAgents, 10) }}>Agents</div>
        {/* his line */}
        <div style={{ position: "absolute", left: 0, width: W, top: 556, textAlign: "center", fontFamily: FONT, fontSize: 54, fontWeight: 750, letterSpacing: "-0.02em", color: C.ink, opacity: more, transform: `translateY(${(1 - more) * 14}px)`, filter: blurOf((1 - more) * 6) }}>
          More time for the work{" "}
          <span style={{ position: "relative", display: "inline-block", color: interpolateColors(only, [0, 1], [C.ink, C.accent]) }}>
            only you
            <span style={{ position: "absolute", left: 0, right: 0, bottom: -4, height: 6, borderRadius: 4, background: C.accent, transform: `scaleX(${only})`, transformOrigin: "0 50%" }} />
          </span>{" "}
          can do
        </div>
      </div>
      {/* arrow workflows -> agents (stage coords; drawn after the card has its final top for this phase) */}
      {f >= T.mdArrow ? <ArrowTo x1={X + WF.x + 172} y1={TOP + WF.y} x2={X + AGN.x - 118} y2={TOP + AGN.y} bend={0} delay={T.mdArrow} dur={10} width={6} /> : null}
    </div>
  );
};

/* ------------------------------------------------------------------ checklist + Honcho base */

const CheckRow: React.FC<{ text: string; at: number; x: number; cy: number; size?: number; f: number }> = ({ text, at: a, x, cy, size = 46, f }) => {
  const p = prog(f, a, 14);
  const tick = prog(f, a + 5, 12);
  const ln = prog(f, a, 10, EASE_IN_OUT);
  if (p <= 0) return null;
  return (
    <>
      <div style={{ position: "absolute", left: 868, top: cy - 2, width: 40 * ln, height: 4, borderRadius: 2, background: alpha(C.accent, 0.7) }} />
      <div style={{ position: "absolute", left: x, top: cy - size * 0.66, display: "flex", alignItems: "center", gap: 18, opacity: p, transform: `translateX(${(1 - p) * -24}px)` }}>
        <CheckIcon size={size * 1.1} t={tick} />
        <div style={{ fontFamily: FONT, fontSize: size, fontWeight: 720, letterSpacing: "-0.02em", color: C.ink, whiteSpace: "nowrap", lineHeight: 1.3 }}>{text}</div>
      </div>
    </>
  );
};

const HON = { x: 64, y: 160, w: 1792, h: 607 };

const Checklist: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.ckIn) return null;
  // dims to 30% before the README card, hides fully while that card turns opaque (never reads through it),
  // and returns to full brightness only after the card is fully out (v3: f2672 to f2684; the card is gone at f2673)
  const d1 = prog(f, T.ckDim, 5, EASE_IN_OUT);
  const d2 = prog(f, T.hoIn - 4, 4, (v) => v);
  const back = prog(f, T.ckBack, 12); // ease-out: picks up right after the card is gone, so the handoff barely dips
  const dim = mix(d1, 1, 0.3) * (1 - d2) * (1 - back) + back;
  const rise = [T.ckIn, T.ckIn + 6, T.ckIn + 12, T.ckIn + 18];
  const litAt = [T.hoLock, T.ck1, T.ck3, T.ck4];
  const sweep = prog(f, T.sweep, 35, EASE_IN_OUT);
  const sweepY = mix(sweep, 1010, 110);
  const sweepO = sweep > 0 && sweep < 1 ? 1 : 0;
  const bgP = prog(f, T.ckBg, 12);
  // tail exit (T7a.md REVISION 2 line 4): fade + 2% scale-down over f2760 to f2771, so f2771 and f2772 are bare canvas and the
  // cut to T7a's bare f0 shows no one-frame vanish
  const endP = prog(f, T.ckEnd, 11, EASE_IN_OUT);
  if (endP >= 1) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
        opacity: 1 - endP,
        transform: endP > 0 ? `scale(${(1 - 0.02 * endP).toFixed(4)})` : undefined,
        transformOrigin: "960px 540px",
      }}
    >
      <Headline text="For this step" delay={T.ckHead} stagger={3} size={64} style={{ position: "absolute", left: 260, top: 58 }} />
      <div style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, opacity: dim }}>
        {[1, 2, 3, 4].map((n) => {
          const p = prog(f, rise[n - 1], 12);
          const lit = prog(f, litAt[n - 1], 10);
          return <Slab key={n} rect={CHECK[n - 1]} n={n} r={0} lit={lit} nameO={1} opacity={p} dy={(1 - p) * 34} scale={mix(p, 0.96, 1)} blur={(1 - p) * 8} />;
        })}
        <CheckRow f={f} text="Install Composio" at={T.ck1} x={920} cy={515} />
        <CheckRow f={f} text="Connect your connectors" at={T.ck2} x={920} cy={587} />
        <CheckRow f={f} text="Skills for workflows you repeat" at={T.ck3} x={920} cy={390} />
        <CheckRow f={f} text="Set up routines" at={T.ck4} x={920} cy={200} />
        {bgP > 0 ? (
          <div style={{ position: "absolute", left: 990, top: 236, opacity: bgP, transform: `translateY(${(1 - bgP) * 10}px)` }}>
            <TagChip text="Work in the background" size={32} glyph={<ClockGlyph size={34} color={C.accent} turn={(f - T.ckBg) * 4} />} />
          </div>
        ) : null}
      </div>
      <HonchoMorph />
      {/* light sweep up through all five slabs */}
      {sweepO ? (
        <div style={{ position: "absolute", left: 256, top: 166, width: 608, height: 768, overflow: "hidden", borderRadius: 22, pointerEvents: "none" }}>
          <div
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              top: sweepY - 166 - 110,
              height: 220,
              background: `linear-gradient(180deg, ${alpha(C.accent, 0)} 0%, ${alpha(C.accent, 0.28)} 50%, ${alpha(C.accent, 0)} 100%)`,
              mixBlendMode: "screen",
            }}
          />
        </div>
      ) : null}
    </div>
  );
};

/** T3's Honcho README card (f2614 to f2673), then the base slab "Honcho memory" UNDER slab 1 (v3).
 *  The card fades fully out (shrinking toward the base slot) by f2673, BEFORE the checklist brightens (f2672 to f2684), so it is
 *  never drawn over the stack. The base slab then rises in only below slab 1: its top edge stays at y >= 810 at every frame
 *  (Context's bottom edge is y 770), and it locks at "system" (f2688). */
const HonchoMorph: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.hoIn) return null;
  const p = prog(f, T.hoIn, 18);
  const out = prog(f, T.hoOut, 10, EASE_IN_OUT);
  const s = 1.12;
  const bp = prog(f, T.baseIn, 12);
  const lock = prog(f, T.hoLock, 10);
  const lockBump = bump(f, T.hoLock, 10);
  return (
    <>
      {out < 1 ? (
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
            opacity: 1 - out,
            transform: out > 0 ? `scale(${mix(out, 1, 0.88).toFixed(4)})` : undefined,
            transformOrigin: `${BASE.x + BASE.w / 2}px ${BASE.y + BASE.h / 2}px`,
            filter: blurOf(out * 6),
          }}
        >
          <div
            style={{
              position: "absolute",
              left: HON.x,
              top: HON.y + (1 - p) * 36,
              width: HON.w,
              height: 64 + HON.h,
              borderRadius: 22,
              overflow: "hidden",
              background: C.paper,
              boxShadow: velour.shadow.card,
              // opaque within 5 frames, so the dimmed checklist never reads through the rising card
              opacity: prog(f, T.hoIn, 5, (v) => v),
              transform: `scale(${mix(p, 0.97, 1)})`,
              filter: blurOf((1 - p) * 10),
            }}
          >
            {/* kit BrowserCard chrome, copied */}
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
                github.com/plastic-labs/honcho
              </div>
            </div>
            <div style={{ position: "relative", width: HON.w, height: HON.h, overflow: "hidden" }}>
              <Img src={staticFile(`${P}05b-honcho-readme-t3crop.png`)} style={{ width: HON.w, height: HON.h, display: "block" }} />
              <HL box={[5, 443, 517, 487]} s={s} delay={T.hoMark} dur={10} kind="marker" />
            </div>
          </div>
          <LabelRow x={HON.x} y={HON.y + 64 + HON.h + 18} w={HON.w} domain="github.com/plastic-labs/honcho" delay={T.hoIn + 4} />
        </div>
      ) : null}
      {bp > 0 ? (
        <div
          style={{
            position: "absolute",
            left: BASE.x,
            // rises from below only (dy >= 0) and scales from its TOP edge, so the top never goes above y 810
            top: BASE.y + (1 - bp) * 30,
            width: BASE.w,
            height: BASE.h,
            borderRadius: 20,
            overflow: "hidden",
            boxSizing: "border-box",
            background: interpolateColors(lock, [0, 1], [C.paper, C.accentSoft]),
            border: `2.5px solid ${interpolateColors(lock, [0, 1], [C.line, C.accent])}`,
            boxShadow: velour.shadow.soft,
            opacity: bp,
            transform: `scale(${mix(bp, 0.96, 1) * (1 + 0.03 * lockBump)})`,
            transformOrigin: "50% 0%",
            filter: blurOf((1 - bp) * 8),
          }}
        >
          <div style={{ position: "absolute", left: 30, top: 32 }}>
            <DatabaseGlyph size={54} color={interpolateColors(lock, [0, 1], [C.muted, C.accent])} />
          </div>
          <div style={{ position: "absolute", left: 108, top: 60 - 46 * 0.62, fontFamily: FONT, fontSize: 46, fontWeight: 750, letterSpacing: "-0.02em", lineHeight: 1.24, color: C.ink, whiteSpace: "nowrap" }}>
            Honcho memory
          </div>
        </div>
      ) : null}
    </>
  );
};

/* ------------------------------------------------------------------ scene */

const Scene: React.FC = () => {
  return (
    <VelourCanvas tone="dark">
      <Opener />
      <ConceptText />
      <ChatWindow />
      <OsFrame />
      <OsTitle />
      <StackA />
      <Callback />
      <Connectors />
      <ComposioCards />
      <SkillsBeats />
      <RoutinesBeats />
      <MorningCard />
      <BorisCard />
      <Mindset />
      <Checklist />
    </VelourCanvas>
  );
};

export default Scene;
