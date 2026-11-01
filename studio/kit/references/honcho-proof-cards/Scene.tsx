import React from "react";
import { Img, interpolateColors, staticFile, useCurrentFrame } from "remotion";
import { VelourCanvas, C, prog, mix, geoZoom, velour, EASE_OUT, EASE_IN_OUT, Headline, Kicker, ArrowTo } from "../../../styles/velour/src";
import { HubDiagram, NodeBox, BoxRing, HubText, HUB_NODE, HUB_TOP_LABEL_Y, accentA } from "./HubDiagram";

// anim-mvagv66m: H1 "What Honcho does, part 1" (scene 48b6, BEHIND, project 321702 to 389247 ms, 2027 frames).
// Stage 1920x1080 scaled x0.768 into the left panel by VelourCanvas (the whole stage is ours; no footage).
// Word times (project ms): honcho exports/_work/writer/words/48b6.json (final). at(ms) = round((ms - 321702) * 30 / 1000) - 3,
// so every beat lands ~3 frames before its word. Every change is sequenced: the old element leaves (or the stack moves)
// before the new one enters; no two texts share a spot at any frame. HubDiagram.tsx (this folder) is the shared hub that
// H2, H3, D1, K1 and E1 import unchanged.
//
// BEATS (frame, word, what shows, proof)
//   f0       "So let me show you"   Headline "Why that's so impactful" rises in (12 f, de-blur), centred.
//   f40      "impactful"            "impactful" turns accent and an accent underline draws under it.
//   f62      ("impactful." ends)    Headline leaves (gone f74).
//   f74      "Honcho," (f87)        Browser card github.com/plastic-labs/honcho: the repo header strip (logo, "honcho",
//                                     "Public", Notifications, Fork 942, Star 7.6k), lands f84. Slow push 1.00 to 1.03 on the
//                                     whole proof f84 to f312. Source pill "github.com · captured Oct 15, 2026" f82 (left).
//                                     proof 04-honcho-repo/01-github-repo-crop.png -> gh-header.png (rows 8-136; the left part
//                                     x 0-372 and the right part x 1750-2480 joined with 100 px of the page's own white, so
//                                     the buttons read ~44 px on stage; no pixel altered; the blurred commit row is below).
//   f159     "fastest"              ONE accent ring on the "Star 7.6k" button (x 970-1177, y 36-91 in gh-header.png).
//   f165     ("growing")            Accent pill "7.6K+ stars" under the ring (static; never a counter or a rank).
//   f196     "projects"             Honest line under the pills: "Star count only. No ranking found that proves 'fastest'."
//   f300     ("why." ends)          Card, pills and line leave (gone f312).
//   f314     "So first of all"      Kicker "Honcho's own line" + browser card honcho.dev with the home hero sentence, word for
//                                     word: "Honcho is open-source AI memory for agents. It remembers each user across
//                                     sessions, tools and machines, so no session starts cold." Pill "honcho.dev · captured
//                                     Oct 15, 2026" f324. proof 05-honcho-concepts/01-cold-start-home-crop.png -> cold-line.png
//                                     (x 418-1622, y 262-382; text x 458-1582, y 290-351).
//   f354     ("solve")              The stack moves down 72 px (12 f), then
//   f366     "the cold start"       headline "The cold start problem" (his words) builds above it, in by ~f384.
//   f384     "cold"                 ONE accent box around "no session starts cold." (x 740-1143, y 58-100 in cold-line.png).
//   f422     ("problem." ends)      Quote card, kicker, pill and headline leave (gone f434).
//   f436     "start a new chat"     Plain chat card "New chat" (diagram, not a product UI) with "Start a new chat." in it
//                                     from its first frame.
//   f484     "forget"               Line 2 "Normally it'll forget who you are" fades in (lands f494).
//   f543     "relationship"         Line 3 "there's no working relationship" (lands f553).
//   f572     ("So" f584)            Chat card leaves (gone f584).
//   f586     "So if you don't"      Boxes "Chat 1" (420, 520) and "Chat 2" (1500, 520) rise in, no links.
//   f628     "Honcho" (f641)        Headline "Without Honcho" builds at the top.
//   f680     "you" (f695)           "You" node (Honcho-node geometry, 960, 520) rises in; f692 accent arrows Chat 1 -> You,
//                                     f700 You -> Chat 2.
//   f709     "memory" (f722)        Label "Memory broker or memory router" under the You node (his words only).
//   f774     ("router." ends f785)  You node, arrows, label and "Without Honcho" leave (gone f786).
//   f784     "But with Honcho,"     The Honcho node rises at centre (lands f796); headline "With Honcho" builds f796 (lands
//                                     on "Honcho," f810).
//   f840     "set"                  Accent arrows Chat 1 -> Honcho and Chat 2 -> Honcho draw.
//   f890     "which"                Accent ring on Chat 1; f907 it lifts, f913 "on," a ring draws on Chat 2.
//   f926     "always" (f939)        Label "It will always have context" above the node (his words).
//   f980                            Chat 2 ring lifts.
//   f1004    "new" (f1019)          Box "New session" (960, 800) rises in, arrow up into the node f1012.
//   f1088    "work"                 Accent ring on the Honcho node (no "better" label: opinion, not in a proof file).
//   f1126    ("better." ends)       Whole diagram leaves (gone f1138, "Alright?").
//   f1144    "So the continuity"    Headline "Continuity" + boxes "Session 1/2/3" (300x140, x 480/960/1440, y 540) arrive
//                                     together (content with its container).
//   f1213    "agents"               A thin accent line draws through the three boxes (behind them), linking them.
//   f1264    "and you can have"     The boxes slide together into one bar (x 340-1580, y 494-586), message ticks inside;
//   f1290    "conversations"        label "Very long conversations" (y 700).
//   f1336    ("agents," ends)       Label leaves; f1340 the bar splits back into Session 1/2/3 (the line shows again);
//   f1347    "remember" (f1360)     label "Remembers things from session to session" in the same spot (after the old one left).
//   f1412    "Okay."                Continuity leaves (gone f1424).
//   f1428    "the second one"       Kicker "The second one" (top, centred); f1440 the Honcho node rises alone at centre.
//   f1514    "how" (f1531)          Headline "How we're going to be using Honcho" builds under the kicker; leaves f1564.
//   f1577    "look"                 HUB: five chips + five lines enter unlit (muted); kicker leaves f1576.
//   f1617    "Honcho"               Node pulses 1.00 to 1.03 and back.
//   f1703    "shared"               Accent ring on the node; label "One shared brain" above it (his metaphor, his words).
//   f1718    "And it doesn't"       Docs line in the free band at the top (y 130), verbatim: "All integrations share one Honcho workspace, so memory
//                                     formed in one carries over to the rest." Pill "honcho.dev/docs · captured Oct 15, 2026"
//                                     f1724. proof 05-honcho-concepts/02-integrations-docs-highlight.png (text source).
//   f1776    "Claude"               Chip "Claude Code" lights, its line draws in accent.
//   f1804    "Codex"                "Codex" lights.  f1826 "open code" -> "OpenCode".  f1847 "open claw" -> "OpenClaw".
//   f1873    "Hermes"               "Hermes Agent" lights.
//   f1893    "matter."              Docs line and pill leave (gone f1905).
//   f1918    "whatever" (f1933)     Bottom label "Whatever is the best tool for the job, we can use that." (his words).
//   f2016    (tail, v2)             Everything on the stage (full hub, five lit chips, ring, both labels) fades out together:
//                                     opacity 1 to 0 with EASE_IN_OUT bezier(0.65,0,0.35,1) and scale 1.00 to 0.98 (centre),
//                                     f2016 to f2024. The VelourCanvas canvas, panel and drifting dots are untouched.
//   f2025    (end, to f2026)        Empty panel only (dots drifting), so the join into H2 never pops between two graphics.
// Do not show (respected): 01-github-repo-full.png / 18-github-repo-folders-full.png (contributor handle + avatars),
// star-history 9,605 / Global Rank / Top Committers, OSS Insight 7,574 / 931, "fastest growing" as a fact, 10x, 99%,
// "free"/"local"/"private", Pi / Kilo Code / DeepSeek Harness / "25 available", diachronic, a moving star counter,
// "shared brain" as Honcho's term (the docs line says "one Honcho workspace").
// prep-proof.py in this folder builds both PNGs under public/honcho/proof/anim-mvagv66m/ from the proof pack.

const JOB_START = 321702;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;

const T = {
  // A: headline
  hAccent: at(323141), // "impactful." 40
  hOut: 62,
  // B: github.com
  ghIn: 74, // lands f84, "Honcho," f87
  ghPill: 82,
  ghRing: at(327105), // "fastest" 159
  ghStars: 165,
  ghHonest: at(328346), // "projects" 196
  ghOut: 300, // gone 312 ("So" first of all 312)
  // C: honcho.dev cold start
  coIn: 314,
  coPill: 324,
  coShift: 354,
  coHead: 366,
  coBox: at(334592), // "cold" 384
  coOut: 422, // gone 434
  // D: new chat
  chIn: 436, // "start" 455
  chL2: at(338276) - 10, // "forget" 494 (lands)
  chL3: at(340238) - 10, // "relationship" 553 (lands)
  chOut: 572, // gone 584 ("So" 584)
  // E: without / with Honcho
  wChats: 586,
  wLabel: 628, // "Honcho" 641
  youIn: at(344883) - 12, // "you" 692 (lands)
  youArrow: at(344883), // 692
  broker: at(345764) - 10, // "memory" 719 (lands)
  youOut: 774, // gone 786 ("But" 785)
  hoIn: 784,
  withLabel: 796, // "Honcho," 810
  hoArrows: at(349808), // "set" 840
  ring1: at(351469), // "which" 890
  ring2: at(352230), // "on," 913
  ctx: at(352991) - 10, // "always" 936 (lands)
  ringOff: 980,
  newIn: at(355654) - 12, // "new" 1016 (lands)
  nodeRing: at(358076), // "work" 1088
  wOut: 1126, // gone 1138 ("Alright?" 1136)
  // F: continuity
  cIn: 1144, // "So the continuity" ("So" 1152); headline lands ~1156, before "continuity" 1171
  cLine: at(362220), // "agents" 1213
  cMerge: 1264,
  cLabel1: 1290, // "conversations" 1304
  cLabel1Out: 1336,
  cSplit: 1340,
  cLabel2: at(367045) - 10, // "remember" 1357 (lands)
  cOut: 1412, // gone 1424 ("Okay." 1422)
  // G: the second one + hub
  sKicker: 1428, // "second" 1439
  sNode: 1440,
  sHead: 1514, // "how" 1528
  sHeadOut: 1564,
  hub: 1577, // "look" 1587
  kOut: 1576,
  pulse: at(375698), // "Honcho" 1617
  ringHub: at(378557), // "shared" 1703
  src: 1718, // "And" 1730
  srcPill: 1724,
  litClaude: at(380999), // 1776
  litCodex: at(381940), // 1804
  litOpenCode: at(382681), // 1826
  litOpenClaw: at(383382), // 1847
  litHermes: at(384222), // 1873
  srcOut: at(384903), // "matter." 1893
  endLabel: at(386144) - 12, // "whatever" 1930 (lands)
  // v2 tail: the whole stage fades out f2016 to f2024, gone f2025 to f2026 (the placement may drop f2026)
  tailOut: 2016,
  tailDur: 8,
};

const FONT = velour.font.display;
const P = "honcho/proof/anim-mvagv66m/";
const BAR = 72;
const CARD_SHADOW = `${velour.shadow.card}, 0 0 0 1.5px rgba(255,255,255,0.10)`;
const blurOf = (v: number) => (v > 0.02 ? `blur(${v.toFixed(2)}px)` : undefined);

/* ------------------------------------------------------------------ glyphs (as in I1 / V1) */

const LockGlyph: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" style={{ flex: "none" }}>
    <rect x={5} y={10.5} width={14} height={10} rx={2.4} fill={color} />
    <path d="M8.2 10.5V8a3.8 3.8 0 0 1 7.6 0v2.5" fill="none" stroke={color} strokeWidth={2.2} />
  </svg>
);

const LinkGlyph: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" style={{ flex: "none" }}>
    <path d="M10 14a4.5 4.5 0 0 0 6.4 0l3.2-3.2a4.5 4.5 0 0 0-6.4-6.4L12 5.6" stroke={color} strokeWidth={2.4} strokeLinecap="round" />
    <path d="M14 10a4.5 4.5 0 0 0-6.4 0l-3.2 3.2a4.5 4.5 0 0 0 6.4 6.4l1.2-1.2" stroke={color} strokeWidth={2.4} strokeLinecap="round" />
  </svg>
);

const ChatGlyph: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" style={{ flex: "none" }}>
    <path d="M4 5.5h16a1.5 1.5 0 0 1 1.5 1.5v9a1.5 1.5 0 0 1-1.5 1.5H10l-4.5 3.5v-3.5H4A1.5 1.5 0 0 1 2.5 16V7A1.5 1.5 0 0 1 4 5.5Z" stroke={color} strokeWidth={2.2} strokeLinejoin="round" />
  </svg>
);

const CrossGlyph: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" style={{ flex: "none" }}>
    <circle cx={12} cy={12} r={10} stroke={color} strokeWidth={2.2} />
    <path d="M8.5 8.5l7 7M15.5 8.5l-7 7" stroke={color} strokeWidth={2.4} strokeLinecap="round" />
  </svg>
);

/* --------------------------------------------------------------- shared bits (V1 helpers, copied) */

/** Source pill (I1 style). `align` left puts its left edge at x; centre centres it on x. */
const SourcePill: React.FC<{ text: string; tail: string; at: number; x?: number; top: number; align?: "center" | "left"; size?: number; height?: number }> = ({
  text,
  tail,
  at: start,
  x = 960,
  top,
  align = "center",
  size = 33,
  height = 62,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 12, EASE_OUT);
  if (p <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top,
        height,
        transform: `translate(${align === "center" ? "-50%" : "0px"}, ${((1 - p) * 10).toFixed(2)}px)`,
        opacity: p,
        display: "flex",
        alignItems: "center",
        gap: size * 0.42,
        padding: `0 ${size * 0.85}px`,
        borderRadius: 999,
        background: C.paper,
        border: `1.5px solid ${C.darkLine}`,
        boxSizing: "border-box",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 600,
        whiteSpace: "nowrap",
        color: C.muted,
      }}
    >
      <LinkGlyph size={size * 0.9} color={C.accent} />
      <span style={{ color: C.inkSoft }}>{text}</span>
      <span>·</span>
      <span>{tail}</span>
    </div>
  );
};

type Rect = { x: number; y: number; w: number; h: number };

/** Accent ring drawn on (pathLength), in SOURCE px inside the scaled screenshot; `s` keeps the stroke in stage px (V1). */
const Ring: React.FC<{ r: Rect; at: number; s: number; radius: number; stroke?: number }> = ({ r, at: start, s, radius, stroke = 6 }) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 11, EASE_IN_OUT);
  if (p <= 0) return null;
  const sw = stroke / s;
  const glow = 0.45 + 0.55 * prog(f, start + 7, 10);
  return (
    <svg
      width={r.w + sw * 2}
      height={r.h + sw * 2}
      style={{ position: "absolute", left: r.x - sw, top: r.y - sw, overflow: "visible", filter: `drop-shadow(0 0 ${((14 * glow) / s).toFixed(2)}px ${accentA(0.35 + 0.35 * glow)})` }}
    >
      <rect
        x={sw}
        y={sw}
        width={r.w}
        height={r.h}
        rx={radius}
        fill={accentA(0.1 * glow)}
        stroke={C.accent}
        strokeWidth={sw}
        strokeLinecap="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1 - p}
      />
    </svg>
  );
};

/** Browser chrome bar (V1 style): traffic lights + URL pill with lock, domain and muted path. */
const BrowserBar: React.FC<{ w: number; domain: string; path: string }> = ({ w, domain, path }) => (
  <div style={{ position: "absolute", left: 0, top: 0, width: w, height: BAR, display: "flex", alignItems: "center", gap: 24, padding: "0 26px", boxSizing: "border-box" }}>
    <div style={{ display: "flex", gap: 11, flex: "none" }}>
      {[C.red, C.gold, C.green].map((c) => (
        <div key={c} style={{ width: 16, height: 16, borderRadius: 99, background: c, opacity: 0.9 }} />
      ))}
    </div>
    <div
      style={{
        flex: 1,
        minWidth: 0,
        height: 48,
        borderRadius: 24,
        background: C.bg,
        border: `1.5px solid ${C.darkLine}`,
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "0 22px",
        fontFamily: FONT,
        fontSize: 28,
        fontWeight: 600,
        color: C.onDark,
        whiteSpace: "nowrap",
        overflow: "hidden",
      }}
    >
      <LockGlyph size={24} color={C.muted} />
      <span style={{ flex: "none" }}>{domain}</span>
      {path ? <span style={{ color: C.muted, fontWeight: 500, marginLeft: -12, overflow: "hidden", textOverflow: "ellipsis", minWidth: 0 }}>{path}</span> : null}
    </div>
  </div>
);

/** A screenshot at scale `s` (native-size layer scaled from 0 0, so children use SOURCE px) (V1). */
const Shot: React.FC<{ src: string; iw: number; ih: number; s: number; filter?: string; children?: React.ReactNode }> = ({ src, iw, ih, s, filter, children }) => (
  <div style={{ position: "absolute", left: 0, top: 0, width: iw, height: ih, transform: `scale(${s})`, transformOrigin: "0 0" }}>
    <Img src={staticFile(src)} style={{ position: "absolute", left: 0, top: 0, width: iw, height: ih, maxWidth: "none", filter }} />
    {children}
  </div>
);

/** Browser card shell: rises in at `start` (10 f). */
const BrowserCard: React.FC<{ x: number; y: number; w: number; ch: number; start: number; domain: string; path: string; pageBg: string; children?: React.ReactNode }> = ({
  x,
  y,
  w,
  ch,
  start,
  domain,
  path,
  pageBg,
  children,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 10, EASE_OUT);
  if (p <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: BAR + ch,
        borderRadius: 26,
        overflow: "hidden",
        background: C.dark,
        boxShadow: CARD_SHADOW,
        opacity: p,
        transform: `translateY(${((1 - p) * 24).toFixed(2)}px) scale(${mix(p, 0.975, 1).toFixed(4)})`,
        filter: blurOf((1 - p) * 8),
      }}
    >
      <BrowserBar w={w} domain={domain} path={path} />
      <div style={{ position: "absolute", left: 0, top: BAR, width: w, height: ch, overflow: "hidden", background: pageBg }}>{children}</div>
    </div>
  );
};

/** Small text line that rises in at `start`, centred on x or left-aligned. */
const Line: React.FC<{ text: React.ReactNode; start: number; top: number; size?: number; weight?: number; color?: string; x?: number; align?: "center" | "left"; dur?: number }> = ({
  text,
  start,
  top,
  size = 34,
  weight = 600,
  color = C.muted,
  x = 960,
  align = "center",
  dur = 10,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, dur, EASE_OUT);
  if (p <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: align === "center" ? x - 940 : x,
        width: align === "center" ? 1880 : undefined,
        top,
        textAlign: align,
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        letterSpacing: "-0.01em",
        color,
        whiteSpace: "nowrap",
        opacity: p,
        transform: `translateY(${((1 - p) * 12).toFixed(2)}px)`,
        filter: blurOf((1 - p) * 6),
      }}
    >
      {text}
    </div>
  );
};

/* ------------------------------------------------------------------ A: headline */

const OpenHeadline: React.FC = () => {
  const f = useCurrentFrame();
  if (f > T.hOut + 12) return null;
  const p = prog(f, 0, 12, EASE_OUT);
  const o = 1 - prog(f, T.hOut, 12, EASE_IN_OUT);
  const ac = prog(f, T.hAccent, 10, EASE_OUT);
  const ul = prog(f, T.hAccent, 12, EASE_IN_OUT);
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        width: 1920,
        top: 466,
        display: "flex",
        justifyContent: "center",
        fontFamily: FONT,
        fontSize: 124,
        fontWeight: 800,
        letterSpacing: "-0.035em",
        lineHeight: 1.04,
        color: C.ink,
        whiteSpace: "nowrap",
        opacity: p * o,
        transform: `translateY(${((1 - p) * 36 - (1 - o) * 14).toFixed(2)}px) scale(${(mix(p, 0.97, 1) * mix(o, 0.98, 1)).toFixed(4)})`,
        filter: blurOf((1 - p) * 10 + (1 - o) * 6),
      }}
    >
      <span>Why that's so&nbsp;</span>
      <span style={{ position: "relative", color: interpolateColors(ac, [0, 1], [C.ink, C.accent]) }}>
        impactful
        <span style={{ position: "absolute", left: 4, bottom: -14, height: 9, borderRadius: 9, width: `calc(${(ul * 100).toFixed(2)}% - 8px)`, background: C.accent, boxShadow: `0 0 18px ${accentA(0.5 * ul)}` }} />
      </span>
    </div>
  );
};

/* ------------------------------------------------------------------ B: github.com repo header */

// gh-header.png 1202x128 at s 1.45 -> 1743 x 186 (button text ~44 px on stage).
const GH = { iw: 1202, ih: 128, s: 1.45 };
const GH_W = Math.round(GH.iw * GH.s);
const GH_CH = Math.round(GH.ih * GH.s);
const GH_X = Math.round(960 - GH_W / 2);
const GH_STACK = BAR + GH_CH + 24 + 62 + 34 + 44;
const GH_Y = Math.round((1080 - GH_STACK) / 2);
const GH_RING: Rect = { x: 960, y: 27, w: 227, h: 74 }; // "Star 7.6k" button (x 970-1177, y 36-91)
const GH_RING_CX = GH_X + (GH_RING.x + GH_RING.w / 2) * GH.s;

const RepoBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.ghIn - 1 || f > T.ghOut + 12) return null;
  const o = 1 - prog(f, T.ghOut, 12, EASE_IN_OUT);
  const push = geoZoom(prog(f, T.ghIn + 10, T.ghOut + 12 - T.ghIn - 10, EASE_IN_OUT), 1, 1.03);
  const sp = prog(f, T.ghStars, 12, EASE_OUT);
  const rowTop = GH_Y + BAR + GH_CH + 24;
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        opacity: o,
        filter: blurOf((1 - o) * 6),
        transform: `scale(${push.toFixed(5)})`,
        transformOrigin: `960px ${GH_Y + 180}px`,
      }}
    >
      <BrowserCard x={GH_X} y={GH_Y} w={GH_W} ch={GH_CH} start={T.ghIn} domain="github.com" path="/plastic-labs/honcho" pageBg={C.white}>
        <Shot src={`${P}gh-header.png`} iw={GH.iw} ih={GH.ih} s={GH.s} filter="brightness(0.93)">
          <Ring r={GH_RING} at={T.ghRing} s={GH.s} radius={14} />
        </Shot>
      </BrowserCard>
      <SourcePill text="github.com" tail="captured Oct 15, 2026" at={T.ghPill} x={GH_X} align="left" top={rowTop} />
      {sp > 0 ? (
        <div
          style={{
            position: "absolute",
            left: GH_RING_CX,
            top: rowTop,
            height: 62,
            transform: `translate(-50%, ${((1 - sp) * 10).toFixed(2)}px)`,
            opacity: sp,
            filter: blurOf((1 - sp) * 6),
            display: "flex",
            alignItems: "center",
            padding: "0 30px",
            borderRadius: 999,
            background: accentA(0.16),
            boxShadow: `inset 0 0 0 2px ${accentA(0.75)}`,
            fontFamily: FONT,
            fontSize: 36,
            fontWeight: 800,
            letterSpacing: "-0.015em",
            color: C.ink,
            whiteSpace: "nowrap",
          }}
        >
          <span style={{ color: C.accent }}>7.6K+</span>&nbsp;stars
        </div>
      ) : null}
      <Line text="Star count only. No ranking found that proves ‘fastest’." start={T.ghHonest} top={rowTop + 62 + 34} size={34} weight={600} color={C.muted} />
    </div>
  );
};

/* ------------------------------------------------------------------ C: honcho.dev, no session starts cold */

// cold-line.png 1204x120 at s 1.4 -> 1686 x 168 (page text ~46 px on stage).
const CO = { iw: 1204, ih: 120, s: 1.4 };
const CO_W = Math.round(CO.iw * CO.s);
const CO_CH = Math.round(CO.ih * CO.s);
const CO_X = Math.round(960 - CO_W / 2);
const CO_STACK = 36 + 22 + BAR + CO_CH + 24 + 62; // kicker, card, pill
const CO_K0 = Math.round((1080 - CO_STACK) / 2);
const CO_HEAD_H = 100;
const CO_HEAD_GAP = 44;
const CO_H_TOP = Math.round((1080 - (CO_HEAD_H + CO_HEAD_GAP + CO_STACK)) / 2);
const CO_K1 = CO_H_TOP + CO_HEAD_H + CO_HEAD_GAP;
const CO_BOX: Rect = { x: 740, y: 58, w: 403, h: 42 }; // "no session starts cold." (text x 750-1133, y 68-89)

const ColdBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.coIn - 1 || f > T.coOut + 12) return null;
  const o = 1 - prog(f, T.coOut, 12, EASE_IN_OUT);
  const sh = prog(f, T.coShift, 12, EASE_IN_OUT);
  const k = mix(sh, CO_K0, CO_K1);
  const cardTop = k + 36 + 22;
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      {f >= T.coHead - 1 ? (
        <Headline
          text="The cold start problem"
          delay={T.coHead}
          stagger={3}
          size={96}
          align="center"
          accent={["cold", "start"]}
          style={{ position: "absolute", left: 0, width: 1920, top: CO_H_TOP, whiteSpace: "nowrap", color: C.ink }}
        />
      ) : null}
      <Kicker text="Honcho's own line" delay={T.coIn} size={30} style={{ position: "absolute", left: CO_X + 4, top: k }} />
      <BrowserCard x={CO_X} y={cardTop} w={CO_W} ch={CO_CH} start={T.coIn} domain="honcho.dev" path="" pageBg={C.bg}>
        <Shot src={`${P}cold-line.png`} iw={CO.iw} ih={CO.ih} s={CO.s}>
          <Ring r={CO_BOX} at={T.coBox} s={CO.s} radius={8} />
        </Shot>
      </BrowserCard>
      <SourcePill text="honcho.dev" tail="captured Oct 15, 2026" at={T.coPill} top={cardTop + BAR + CO_CH + 24} />
    </div>
  );
};

/* ------------------------------------------------------------------ D: a new chat */

const CH_W = 1160;
const CH_HEAD = 84;
const CH_H = 470;
const CH_X = 960 - CH_W / 2;
const CH_Y = Math.round((1080 - CH_H) / 2);

const ChatBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.chIn - 1 || f > T.chOut + 12) return null;
  const p = prog(f, T.chIn, 12, EASE_OUT);
  const o = 1 - prog(f, T.chOut, 12, EASE_IN_OUT);
  const row = (start: number | null, text: string, top: number) => {
    const q = start == null ? 1 : prog(f, start, 10, EASE_OUT);
    if (q <= 0) return null;
    return (
      <div
        style={{
          position: "absolute",
          left: 64,
          top,
          display: "flex",
          alignItems: "center",
          gap: 22,
          fontSize: 42,
          fontWeight: 650,
          letterSpacing: "-0.015em",
          color: C.inkSoft,
          whiteSpace: "nowrap",
          opacity: q,
          transform: `translateY(${((1 - q) * 12).toFixed(2)}px)`,
          filter: blurOf((1 - q) * 6),
        }}
      >
        <CrossGlyph size={40} color={C.muted} />
        {text}
      </div>
    );
  };
  return (
    <div
      style={{
        position: "absolute",
        left: CH_X,
        top: CH_Y,
        width: CH_W,
        height: CH_H,
        borderRadius: 30,
        overflow: "hidden",
        background: C.paper,
        boxShadow: CARD_SHADOW,
        fontFamily: FONT,
        opacity: p * o,
        transform: `translateY(${((1 - p) * 30 - (1 - o) * 12).toFixed(2)}px) scale(${(mix(p, 0.97, 1) * mix(o, 0.98, 1)).toFixed(4)})`,
        filter: blurOf((1 - p) * 8 + (1 - o) * 6),
      }}
    >
      <div style={{ position: "absolute", left: 0, top: 0, width: CH_W, height: CH_HEAD, background: C.dark, display: "flex", alignItems: "center", gap: 18, padding: "0 30px", boxSizing: "border-box" }}>
        <ChatGlyph size={34} color={C.muted} />
        <div style={{ fontSize: 34, fontWeight: 700, color: C.ink, letterSpacing: "-0.01em" }}>New chat</div>
      </div>
      <div style={{ position: "absolute", left: 64, top: CH_HEAD + 52, fontSize: 52, fontWeight: 800, letterSpacing: "-0.025em", color: C.ink, whiteSpace: "nowrap" }}>Start a new chat.</div>
      <div style={{ position: "absolute", left: 64, right: 64, top: CH_HEAD + 52 + 62 + 30, height: 2, background: C.line }} />
      {row(T.chL2, "Normally it'll forget who you are", CH_HEAD + 52 + 62 + 30 + 36)}
      {row(T.chL3, "there's no working relationship", CH_HEAD + 52 + 62 + 30 + 36 + 56 + 32)}
    </div>
  );
};

/* ------------------------------------------------------------------ E: without / with Honcho */

const CHAT_W = 300;
const CHAT_H = 140;
const C1 = { cx: 420, cy: 520 };
const C2 = { cx: 1500, cy: 520 };
const NS = { cx: 960, cy: 800, w: 340, h: 140 };
const NODE_L = HUB_NODE.cx - HUB_NODE.w / 2; // 790
const NODE_R = HUB_NODE.cx + HUB_NODE.w / 2; // 1130
const AG = 14; // arrow gap from box edges
const TOP_HEAD_Y = 150;

const DiagramBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.wChats - 1 || f > T.wOut + 14) return null;
  const o = 1 - prog(f, T.wOut, 12, EASE_IN_OUT);
  const chatIcon = <ChatGlyph size={38} color={C.muted} />;
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      {/* headline: Without Honcho, then With Honcho in the same spot (the old one is gone first) */}
      {f < T.youOut + 13 ? (
        <Headline
          text="Without Honcho"
          delay={T.wLabel}
          stagger={4}
          size={76}
          align="center"
          accent={["Honcho"]}
          out={T.youOut}
          style={{ position: "absolute", left: 0, width: 1920, top: TOP_HEAD_Y, whiteSpace: "nowrap", color: C.ink }}
        />
      ) : null}
      {f >= T.withLabel - 1 ? (
        <Headline
          text="With Honcho"
          delay={T.withLabel}
          stagger={4}
          size={76}
          align="center"
          accent={["Honcho"]}
          style={{ position: "absolute", left: 0, width: 1920, top: TOP_HEAD_Y, whiteSpace: "nowrap", color: C.ink }}
        />
      ) : null}
      {/* arrows under the boxes */}
      <ArrowTo x1={C1.cx + CHAT_W / 2 + AG} y1={520} x2={NODE_L - AG} y2={520} bend={0} delay={T.youArrow} dur={14} out={T.youOut} />
      <ArrowTo x1={NODE_R + AG} y1={520} x2={C2.cx - CHAT_W / 2 - AG} y2={520} bend={0} delay={T.youArrow + 8} dur={14} out={T.youOut} />
      {f >= T.hoArrows ? (
        <>
          <ArrowTo x1={C1.cx + CHAT_W / 2 + AG} y1={520} x2={NODE_L - AG} y2={520} bend={0} delay={T.hoArrows} dur={16} />
          <ArrowTo x1={C2.cx - CHAT_W / 2 - AG} y1={520} x2={NODE_R + AG} y2={520} bend={0} delay={T.hoArrows + 4} dur={16} />
        </>
      ) : null}
      {f >= T.newIn + 8 ? <ArrowTo x1={960} y1={NS.cy - NS.h / 2 - AG} x2={960} y2={HUB_NODE.cy + HUB_NODE.h / 2 + AG} bend={0} delay={T.newIn + 8} dur={14} /> : null}
      {/* boxes */}
      <NodeBox cx={C1.cx} cy={C1.cy} w={CHAT_W} h={CHAT_H} r={26} label="Chat 1" size={42} at={T.wChats} icon={chatIcon} />
      <NodeBox cx={C2.cx} cy={C2.cy} w={CHAT_W} h={CHAT_H} r={26} label="Chat 2" size={42} at={T.wChats + 4} icon={chatIcon} />
      <NodeBox cx={HUB_NODE.cx} cy={HUB_NODE.cy} label="You" size={48} at={T.youIn} out={T.youOut} />
      <HubText text="Memory broker or memory router" cy={660} at={T.broker} dur={10} out={T.youOut} size={42} weight={750} />
      {f >= T.hoIn - 1 ? <NodeBox cx={HUB_NODE.cx} cy={HUB_NODE.cy} label="Honcho" size={48} tone="accent" at={T.hoIn} /> : null}
      {f >= T.newIn - 1 ? <NodeBox cx={NS.cx} cy={NS.cy} w={NS.w} h={NS.h} r={26} label="New session" size={40} at={T.newIn} /> : null}
      {/* rings: Chat 1, then Chat 2, then the Honcho node */}
      <BoxRing cx={C1.cx} cy={C1.cy} w={CHAT_W} h={CHAT_H} r={26} m={12} at={T.ring1} out={T.ring2 - 6} />
      <BoxRing cx={C2.cx} cy={C2.cy} w={CHAT_W} h={CHAT_H} r={26} m={12} at={T.ring2} out={T.ringOff} />
      <BoxRing cx={HUB_NODE.cx} cy={HUB_NODE.cy} w={HUB_NODE.w} h={HUB_NODE.h} r={HUB_NODE.r} at={T.nodeRing} />
      <HubText text="It will always have context" cy={HUB_TOP_LABEL_Y} at={T.ctx} dur={10} size={42} weight={750} />
    </div>
  );
};

/* ------------------------------------------------------------------ F: continuity */

const S_W = 300;
const S_H = 140;
const S_CX = [480, 960, 1440];
const S_CY = 540;
const BAR_X0 = 340;
const BAR_X1 = 1580;
const BAR_H = 92;
// message ticks inside the merged bar (fixed pattern: two rows, alternating, like a long chat)
const TICK_W = [72, 50, 88, 58, 66, 44, 80];
const TICKS = Array.from({ length: 22 }, (_, i) => ({ x: 40 + i * 52.5, w: TICK_W[i % TICK_W.length], up: i % 2 === 0 }));

const ContinuityBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.cIn - 1 || f > T.cOut + 13) return null;
  const o = 1 - prog(f, T.cOut, 12, EASE_IN_OUT);
  const m = prog(f, T.cMerge, 20, EASE_IN_OUT) - prog(f, T.cSplit, 18, EASE_IN_OUT);
  const third = (BAR_X1 - BAR_X0) / 3;
  // the session labels are gone before the bar covers the seams (m 0.75), and back as soon as the boxes separate
  const textO = Math.max(0, Math.min(1, (0.75 - m) / 0.45));
  const barO = Math.max(0, Math.min(1, (m - 0.7) / 0.3));
  const lineP = prog(f, T.cLine, 20, EASE_IN_OUT);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <HubText text="Continuity" cy={272} at={T.cIn} dur={12} size={96} weight={800} />
      {/* the linking line, behind the boxes */}
      {lineP > 0 ? (
        <svg style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080 }} viewBox="0 0 1920 1080">
          <path d={`M ${S_CX[0]} ${S_CY} L ${S_CX[2]} ${S_CY}`} fill="none" stroke={accentA(0.22)} strokeWidth={16} strokeLinecap="round" strokeDasharray={960} strokeDashoffset={960 * (1 - lineP)} />
          <path d={`M ${S_CX[0]} ${S_CY} L ${S_CX[2]} ${S_CY}`} fill="none" stroke={C.accent} strokeWidth={5} strokeLinecap="round" strokeDasharray={960} strokeDashoffset={960 * (1 - lineP)} />
        </svg>
      ) : null}
      {S_CX.map((cx, i) => {
        const p = prog(f, T.cIn + i * 3, 12, EASE_OUT);
        if (p <= 0) return null;
        const x0 = mix(m, cx - S_W / 2, BAR_X0 + i * third);
        const w = mix(m, S_W, third);
        const h = mix(m, S_H, BAR_H);
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: x0,
              top: S_CY - h / 2,
              width: w,
              height: h,
              borderRadius: mix(m, 26, 0) + (i === 0 || i === 2 ? m * 35 : 0),
              background: C.paper,
              border: `3px solid ${C.darkLine}`,
              boxSizing: "border-box",
              boxShadow: CARD_SHADOW,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontFamily: FONT,
              fontSize: 42,
              fontWeight: 800,
              letterSpacing: "-0.025em",
              color: C.ink,
              whiteSpace: "nowrap",
              opacity: p,
              transform: `translateY(${((1 - p) * 30).toFixed(2)}px) scale(${mix(p, 0.96, 1).toFixed(4)})`,
              filter: blurOf((1 - p) * 8),
            }}
          >
            <span style={{ opacity: textO }}>Session {i + 1}</span>
          </div>
        );
      })}
      {barO > 0 ? (
        <div
          style={{
            position: "absolute",
            left: BAR_X0,
            top: S_CY - BAR_H / 2,
            width: BAR_X1 - BAR_X0,
            height: BAR_H,
            borderRadius: BAR_H / 2,
            background: C.paper,
            border: `3px solid ${C.accent}`,
            boxSizing: "border-box",
            boxShadow: `${CARD_SHADOW}, 0 0 50px -16px ${accentA(0.5)}`,
            opacity: barO,
          }}
        >
          {TICKS.map((t, i) => (
            <div
              key={i}
              style={{
                position: "absolute",
                left: t.x,
                top: t.up ? 22 : 50,
                width: t.w,
                height: 14,
                borderRadius: 7,
                background: t.up ? C.muted : C.faint,
                opacity: 0.8 * barO,
              }}
            />
          ))}
        </div>
      ) : null}
      <HubText text="Very long conversations" cy={700} at={T.cLabel1} dur={10} out={T.cLabel1Out} outDur={10} size={44} weight={750} />
      <HubText text="Remembers things from session to session" cy={700} at={T.cLabel2} dur={10} size={44} weight={750} />
    </div>
  );
};

/* ------------------------------------------------------------------ G: the second one + the hub */

const SRC_TOP = 130; // the free band above the hub (chips start at y 256); the pill sits between the top chips

const HubBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.sKicker - 1) return null;
  const ko = 1 - prog(f, T.kOut, 12, EASE_IN_OUT);
  const so = 1 - prog(f, T.srcOut, 12, EASE_IN_OUT);
  return (
    <div style={{ position: "absolute", inset: 0 }}>
      {ko > 0 ? (
        <div style={{ position: "absolute", left: 0, width: 1920, top: 128, display: "flex", justifyContent: "center", opacity: ko }}>
          <Kicker text="The second one" delay={T.sKicker} size={32} />
        </div>
      ) : null}
      {f < T.sHeadOut + 13 && f >= T.sHead - 1 ? (
        <Headline
          text="How we're going to be using Honcho"
          delay={T.sHead}
          stagger={2}
          size={76}
          align="center"
          accent={["Honcho"]}
          out={T.sHeadOut}
          style={{ position: "absolute", left: 0, width: 1920, top: 200, whiteSpace: "nowrap", color: C.ink }}
        />
      ) : null}
      <HubDiagram
        nodeAt={T.sNode}
        chipsAt={T.hub}
        pulseAt={T.pulse}
        ringAt={T.ringHub}
        topLabel={{ text: "One shared brain", at: T.ringHub }}
        lit={{ claudeCode: T.litClaude, codex: T.litCodex, opencode: T.litOpenCode, openclaw: T.litOpenClaw, hermes: T.litHermes }}
        bottomLabel={{ text: "Whatever is the best tool for the job, we can use that.", at: T.endLabel }}
      />
      {so > 0 && f >= T.src - 1 ? (
        <div style={{ position: "absolute", inset: 0, opacity: so, filter: blurOf((1 - so) * 5) }}>
          <Line
            text="“All integrations share one Honcho workspace, so memory formed in one carries over to the rest.”"
            start={T.src}
            top={SRC_TOP}
            size={31}
            weight={600}
            color={C.inkSoft}
            dur={12}
          />
          <SourcePill text="honcho.dev/docs" tail="captured Oct 15, 2026" at={T.srcPill} top={SRC_TOP + 48} size={28} height={52} />
        </div>
      ) : null}
    </div>
  );
};

/* ------------------------------------------------------------------ tail fade (v2) */

/**
 * v2: everything drawn on the stage fades out at the tail (opacity 1 to 0, house ease-in-out, scale 1.00 to 0.98)
 * over f2016 to f2024 and stays gone on f2025 to f2026, so the join to H2 shows the empty panel. The wrapper is an
 * unstyled absolute box until f2016 (frames 0 to 2015 render exactly as v1); the VelourCanvas canvas, panel and
 * drifting dots sit outside it and are never touched.
 */
const TailFade: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const f = useCurrentFrame();
  const fading = f >= T.tailOut;
  const p = prog(f, T.tailOut, T.tailDur, EASE_IN_OUT);
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        ...(fading ? { opacity: f >= T.tailOut + T.tailDur ? 0 : 1 - p, transform: `scale(${mix(p, 1, 0.98).toFixed(5)})`, transformOrigin: "960px 540px" } : null),
      }}
    >
      {children}
    </div>
  );
};

/* ------------------------------------------------------------------ scene */

const Scene: React.FC = () => (
  <VelourCanvas tone="dark">
    <TailFade>
      <OpenHeadline />
      <RepoBeat />
      <ColdBeat />
      <ChatBeat />
      <DiagramBeat />
      <ContinuityBeat />
      <HubBeat />
    </TailFade>
  </VelourCanvas>
);

export default Scene;
