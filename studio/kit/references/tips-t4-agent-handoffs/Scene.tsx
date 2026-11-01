import React from "react";
import { useCurrentFrame } from "remotion";
import {
  C,
  EASE_IN,
  EASE_IN_OUT,
  EASE_OUT,
  Highlight,
  Reveal,
  SourcePill,
  TipOpener,
  VelourCanvas,
  geoZoom,
  mix,
  prog,
} from "../../../styles/velour/src";
import { LocalBrowserCard, LocalTweetCard } from "./LocalCards";
import {
  BOT_ARROW_Y,
  ChipStem,
  DottedLink,
  HArrow,
  LEFT_CX,
  LINK_L,
  LINK_R,
  ME,
  MeNode,
  NODE_W,
  NODE_Y,
  NodeBox,
  Packet,
  RIGHT_CX,
  Seg,
  SkillChip,
  TOP_ARROW_Y,
  along,
  packetX,
} from "./Handoff";

// anim-mvas6eqq: TIP 4 "Agent Handoffs" (scene e846, BEHIND, project 248520 to 309021 ms, 1816 frames f0-f1815).
// Stage 1920x1080 scaled x0.768 into the left panel by VelourCanvas (talking head only: the whole stage is ours).
// Word times (project ms): tips exports/_work/writer/words/e846.json (final). at(ms) = round((ms - 248520) * 30 / 1000) - 3,
// so each beat lands 3 frames before its word. Section brief: receipts/briefs/T4.md (rev 2); the designer brief wins on
// the shared TipOpener (96 px, its own fallback card rect) and the locked Boris label.
// Every change is sequenced: the old element leaves before the new one enters in that space; no two texts share a spot.
// Proof images (copied to public/opus-tips/proof/anim-mvas6eqq/): lingxi-article.png (= 07-lingxi-grokbot-tweet.png
// rows 0-1860), boris-created.png (= 10-boris-created-claude-code-tweet.png), boris-agents.png
// (= 09-boris-agents-kicking-off-agents-tweet.png), grokbot-x.png (= 90-gaps/09-grokbot-x-access-tweet.png),
// tb4-top.png (= 90-gaps/10-terminal-bench-4-crop.png crop 1376:295:0:505). Highlight boxes are measured ink + ~4 px.
//
// BEATS (frame, word, what shows, proof)
//   f0     "Okay" (f3)           TipOpener 'TIP 4 / Agent Handoffs' (shared velour component, rises from nothing f0-f12)
//                                 + Lingxi Li's X Article card rises with it (x 305, y 236, w 1310, view rows 0-1100:
//                                 header, orange cover, title). Slow push 1.00 to 1.04 f12-f596, focus on the cover.
//                                 proof 02-memory-handoff-prompts/07-lingxi-grokbot-tweet.png
//   f64    "out" (f67)           Opener exits up (gone f72); the card glides to x 267, y 86, w 1386 over f66-f84;
//                                 pill 'x.com/lingxi · captured Oct 16, 2026' f82; push focus eases to the title.
//   f132   "honestly"            Push focus eases toward the header (arrives f200).
//   f241   "Ling" (f244)         Box on 'Lingxi Li' + badge [203,81,435,132]. Out f280.
//   f280   "And" (f283)          Scroll f280-f300 to rows 760-1860 (title, counts row, two paragraphs).
//   f308   "SpaceX" (f311)       Box on 'SpaceXAI engineer building Grok Bot with Grok Bot.' [166,1347,1232,1398]
//                                 (the capture's own word SpaceXAI; never 'SpaceX'). Out f358.
//   f424   "engineering" (f427)  Box on 'highly capable engineering intern,' [523,1470,1216,1521]. Out f468.
//   f468   "manage" (f471)       Box on 'that can manage coding agents' [56,1542,707,1593]. Out f519.
//   f539   "no" (f542)           Marker on 'no more context switching' [1106,1689,1655,1740].
//   f567   "between" (f570)      Marker on 'between multiple agents,' [59,1761,580,1812]. Hold to f596.
//   f596                         Lingxi card + pill out (gone f605).
//   f605   "And Boris" (f614)    Boris post 1 rises (x 139, y 108, w 1642, view rows 0-880 incl. '2:58 PM · Jan 2, 2026 ·
//                                 8.2M Views'); pill 'x.com/bcherny · captured Oct 16, 2026' (right, y 916). Push 1.03.
//                                 proof 02-memory-handoff-prompts/10-boris-created-claude-code-tweet.png
//   f617   "Boris" (f620)        Box on 'Boris Cherny' [203,81,485,132]. Out f668.
//   f668   "created" (f671)      Box on "I'm Boris and I created Claude Code." [60,237,804,280].
//   f694   "Code," (f697)        Label chip 'Boris Cherny · creator of Claude Code (his own words)' slides in (left, y 916).
//   f703                         Post 1 out (gone f711).
//   f711   "is" (f714)           Boris post 2 rises (x 139, y 270, w 1642, view rows 0-700 incl. '8:42 AM · Jun 9, 2026 ·
//                                 489K Views'). Push 1.03. proof 02-memory-handoff-prompts/09-boris-agents-kicking-off-agents-tweet.png
//   f728   "experimenting"       Underline under 'Starting to experiment more' [57,357,640,408]. Out f808.
//   f808   "agents" (f811)       Box on 'agents kicking off agents' [747,357,1265,408], holds through 'other agents'.
//   f859                         Post 2, label chip and pill out (gone f868).
//   f869   "So" (f872)           Kicker 'THE TIP' + left node 'One agent' (WORKING: pulsing dots + bar filling). Concept.
//   f942   "one" (f945)          Left node border lights accent.
//   f963   "hand" (f966)         Headline 'Hand off.'; top arrow draws; a 'work' packet slides out of the left node.
//   f996   "another" (f999)      Right node 'Another agent' rises (IDLE); the packet slides in under it (~f1010).
//   f1024  "wait" (f1027)        Headline adds 'Wait.'; left node WAITING (pause glyph, bar frozen + dimmed 40%); right
//                                 node WORKING. Bottom arrow draws f1028-f1038.
//   f1040  "answer" (f1043)     'answer' packet rides the bottom arrow back (f1038-f1058); right bar completes, IDLE f1056.
//   f1061  "keep" (f1064)        Headline completes 'Hand off. Wait. Keep going.'; left node back to WORKING.
//   f1086                        Headline out; f1094 'Good at something different' (his words) builds in.
//   f1117  "good" (f1120)        Accent ring on the left node, mint ring on the right node.
//   f1143  "different" (f1146)   The nodes ease 40 px further apart. f1154 diagram + headline + kicker out (gone f1163).
//   f1164  "Grok Bot" (f1167)    Row 1: @bot post (x 124, y 95, w 1672, view rows 0-255: header + 'Grok Bot can now
//                                 search, read, and monitor X.'), pill 'x.com/bot · captured Oct 16, 2026'.
//                                 proof 90-gaps/09-grokbot-x-access-tweet.png. Whole stack pushes 1.00 to 1.02 f1164-f1318.
//   f1190  "full" (f1193)        Box on 'Grok Bot can now search, read, and monitor X.' [48,196,849,238] (never 'full access').
//   f1238  "Claude" (f1241)      Row 2: browser card tbench.ai/leaderboard (Terminal-Bench 4.0 header + rows 1 and 2, text
//                                 only, no bars); pill 'tbench.ai · Terminal-Bench 4.0 · captured Oct 16, 2026'.
//                                 proof 90-gaps/10-terminal-bench-4-crop.png
//   f1263  "better" (f1266)      Box on row 1 '1 · Opus 5.5 (max) · Claude Code · 64.8% ± 3.1%' (image px x123 y163 w1600 h54).
//   f1309                        Stack holds through 'So instead of', then out (gone f1318).
//   f1319  "me" (f1324)          Named diagram returns with the Me node: 'Claude Code' (accent ring), 'Grok Bot' (mint
//                                 ring), both IDLE, no arrows; Me node (clipboard + 'Me') at (960, 860). Dotted links
//                                 Me -> both nodes draw f1324-f1334. Concept.
//   f1338  "copy" (f1341)        Snippet Claude Code -> Me along the dotted link; 'copy paste' label under Me.
//   f1348  "paste" (f1351)       Snippet Me -> Grok Bot.
//   f1356  "between" (f1358)     Snippet Grok Bot -> Me -> Claude Code (to f1380).
//   f1384  "installed" (f1387)   Me group dims to 40%; top arrow draws f1384-f1400.
//   f1399  "skill" (f1402)       Skill chip pops at (960, 432) with Kicker 'SKILL' and a blinking cursor; stem draws.
//   f1424  "Grok Bot" (f1427)    Chip types 'grokbot-webhook' (to f1445). f1445-f1460 bottom arrow draws. Diagram only:
//                                 no repo, URL, install command or star count (no public source).
//   f1485  "Claude" (f1488)      Claude Code border lights; Claude Code WORKING.
//   f1502  "hands" (f1505)       'Twitter research' packet slides out of Claude Code, under the chip by f1526 (chip pulses),
//                                 creeps to f1540, slides in under Grok Bot by f1560.
//   f1560  "Grok Bot" (f1563)    Grok Bot WORKING.
//   f1586  "waits" (f1589)       Claude Code WAITING (pause glyph, bar frozen + dimmed); 'waits' tag above it (y 409-455).
//   f1606  "picks" (f1609)       'result' packet leaves Grok Bot on the bottom arrow; Grok Bot bar completes, IDLE.
//   f1632  "result" (f1635)      Packet in under Claude Code; mint check f1632-f1650, then WORKING; 'waits' tag out.
//   f1649  "because" (f1649)     Reprise inset: the @bot post (x 191, y 40, w 1459, view rows 0-255) + pill (right x 1650,
//                                 y 297); push 1.00 to 1.02. proof 90-gaps/09-grokbot-x-access-tweet.png (no 'better').
//   f1661  "better" (f1664)      Box on 'Grok Bot can now search, read, and monitor X.'; Grok Bot gives one mint pulse.
//   f1689  "And" (f1692)         The loop cycles every 60 frames (f1689, f1749, f1809): Claude Code WAITING + 'Twitter
//                                 research' out on the top arrow; +24 Grok Bot WORKING; +30 'result' back on the
//                                 bottom arrow, Grok Bot IDLE; +54 Claude Code WORKING. The chip pulses as packets pass.
//   f1728                        Reprise inset + pill out (gone f1737).
//   f1742  "stop" (f1745)        Me node, its label and links slide down 60 px and fade (gone f1758).
//   f1762  "middleman" (f1765)   Caption "I've stopped being the middleman" rises at y ~880 (pack's honest line).
//   f1798  "agents" (f1801)      Accent marker swipes under 'middleman' (done f1806), loop running.
//   f1806                        END EXIT: the whole named diagram, the chip and the caption fade up and out over
//                                 f1806-f1815 (fully gone on f1815, only the dot grid left) so T5's opener rises on a clean panel.
// Numbers on screen: only those printed inside the captures (Lingxi counts row 166 / 387 / 3.6K / 8K; '2:58 PM · Jan 2,
// 2026 · 8.2M Views'; '8:42 AM · Jun 9, 2026 · 489K Views'; Terminal-Bench rows 64.8% ± 3.1% and 61.8% ± 2.9%; the
// pill dates). Not shown: 'full access', 'SpaceX' alone, 'first time', 'better' captions, any Grok score, Lingxi views,
// row 10, the @bot views row, any *-tweet-page / *-highlight / -full image, anything below row 1860 of the Article.

const JOB_START = 248520;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;
const P = "opus-tips/proof/anim-mvas6eqq/";
const CAPTURED = "Oct 16, 2026";
const FONT = "'Inter', system-ui, -apple-system, 'Helvetica Neue', Arial, sans-serif";

const T = {
  openOut: at(250761), // "out" 64
  glide: 66,
  lxPill: 82,
  honestly: at(252923, 0), // 132
  nameBox: at(256667), // "Ling" 241
  scroll: at(257948), // "And" 280
  spaceBox: at(258889), // "SpaceX" 308
  spaceOut: at(260538), // "And" 358
  intern: at(262753), // "engineering" 424
  manage: at(264235), // "manage" 468
  manageOut: at(265934), // "So" 519
  noMarker: at(266597), // "no" 539
  between: at(267518), // "between" 567
  lxOut: 596,
  b1In: 605,
  borisBox: at(269180), // "Boris" 617
  created: at(270882), // "created" 668
  label: at(271743), // "Code," 694
  b1Out: 703,
  b2In: 711,
  experiment: at(272884), // "experimenting" 728
  akoa: at(275567), // "agents" 808
  b2Out: 859,
  tipIn: 869,
  one: at(280031), // 942
  hand: at(280734), // 963
  another: at(281813), // 996
  wait: at(282764), // 1024
  answer: at(283275), // 1040
  keep: at(283975), // 1061
  hOut: 1086,
  h2In: 1094,
  good: at(285837), // 1117
  different: at(286718), // 1143
  dOut: 1154,
  r1In: at(287408), // "Grok Bot" 1164
  full: at(288288), // "full" 1190
  r2In: at(289901), // "Claude" 1238
  better1: at(290722), // "better" 1263
  stackOut: 1309,
  d2In: 1319,
  links: 1324,
  copy: at(293225), // 1338
  paste: at(293545), // 1348
  installed: at(294766), // 1384
  skill: at(295247), // 1399
  typeFrom: at(296088), // "Grok Bot" 1424
  typeTo: 1445,
  claude: at(298130), // 1485
  hands: at(298688), // 1502
  grok: at(300632), // 1560
  waits: at(301478), // 1586
  picks: at(302154), // 1606
  result: at(303035), // 1632
  insetIn: 1649,
  better2: at(303994), // "better" 1661
  insetOut: 1728,
  loop: at(304928), // "And" 1689
  stop: at(306699), // 1742
  middleman: at(307359), // 1762
  agentsEnd: at(308561), // 1798
  endOut: 1806, // final exit f1806-f1815 (opacity 0 on f1815)
};

const LOOPS = [T.loop, T.loop + 60, T.loop + 120]; // 1689, 1749, 1809

/* ------------------------------------------------------------------ small local pieces */

/** One line of words that each rise in at their own frame, left-aligned at x (words keep their final positions, so the
 * sentence builds left to right without shifting). */
const WordLine: React.FC<{ words: [string, number][]; x: number; y: number; size: number; out?: number; outDur?: number }> = ({
  words,
  x,
  y,
  size,
  out,
  outDur = 8,
}) => {
  const f = useCurrentFrame();
  const o = out === undefined ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y - size * 0.6,
        height: size * 1.2,
        display: "flex",
        alignItems: "center",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 800,
        letterSpacing: "-0.03em",
        color: C.ink,
        opacity: o,
        transform: `translateY(${-(1 - o) * 12}px)`,
      }}
    >
      {words.map(([w, d], i) => {
        const p = prog(f, d, 12);
        return (
          <span
            key={i}
            style={{
              display: "inline-block",
              whiteSpace: "pre",
              opacity: p,
              transform: `translateY(${(1 - p) * 0.4}em)`,
              filter: `blur(${(1 - p) * 8}px)`,
            }}
          >
            {w}
            {i < words.length - 1 ? " " : ""}
          </span>
        );
      })}
    </div>
  );
};

/** Accent kicker with a fixed leading rule (no width animation, so the text never shifts), left-aligned at x. */
const LeftKicker: React.FC<{ text: string; x: number; y: number; delay: number; out: number }> = ({ text, x, y, delay, out }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 12);
  const o = 1 - prog(f, out, 9, EASE_IN_OUT);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y - 20,
        height: 40,
        display: "flex",
        alignItems: "center",
        gap: 18,
        fontFamily: FONT,
        opacity: p * o,
        transform: `translateY(${(1 - p) * 16 - (1 - o) * 12}px)`,
      }}
    >
      <div style={{ width: 44, height: 4, borderRadius: 2, background: C.accent }} />
      <div style={{ fontSize: 30, fontWeight: 750, letterSpacing: "0.16em", color: C.accent }}>{text}</div>
    </div>
  );
};

/** Person label chip (Boris): his name, then the locked label text. Slides in from the left. */
const PersonLabel: React.FC<{ name: string; rest: string; x: number; y: number; delay: number; out: number }> = ({
  name,
  rest,
  x,
  y,
  delay,
  out,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 12);
  const o = 1 - prog(f, out, 8, EASE_IN_OUT);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        display: "inline-flex",
        alignItems: "center",
        gap: 10,
        padding: "9px 24px",
        borderRadius: 999,
        background: C.dark,
        border: `2px solid ${C.darkLine}`,
        fontFamily: FONT,
        fontSize: 30,
        whiteSpace: "nowrap",
        opacity: p * o,
        transform: `translateX(${(1 - p) * -28}px)`,
      }}
    >
      <span style={{ fontWeight: 700, color: C.ink }}>{name}</span>
      <span style={{ fontWeight: 500, color: C.inkSoft }}>{rest}</span>
    </div>
  );
};

/* ------------------------------------------------------------------ beats 1-6: Lingxi Li */

const LingxiBeat: React.FC = () => {
  const f = useCurrentFrame();
  const e = prog(f, T.glide, 18, EASE_IN_OUT);
  return (
    <>
      <LocalTweetCard
        src={`${P}lingxi-article.png`}
        srcW={1824}
        srcH={1860}
        x={mix(e, 305, 267)}
        y={mix(e, 236, 86)}
        w={mix(e, 1310, 1386)}
        view={[0, 1100]}
        scroll={{ from: T.scroll, to: T.scroll + 20, toY: 760 }}
        delay={0}
        out={T.lxOut}
        outDur={9}
        push={{ from: 12, to: T.lxOut, s1: 1.04 }}
        focus={[
          { at: 0, p: [912, 607] }, // the orange cover
          { at: T.glide, dur: 30, p: [607, 1045] }, // the article title
          { at: T.honestly, dur: 68, p: [300, 130] }, // the header (arrives f200)
          { at: T.scroll + 5, dur: 28, p: [700, 1372] }, // the SpaceXAI line
          { at: T.spaceOut, dur: 32, p: [880, 1640] }, // the 'Think of Grok Bot...' paragraph
        ]}
        highlights={[
          { box: [203, 81, 435, 132], at: T.nameBox, out: T.scroll },
          { box: [166, 1347, 1232, 1398], at: T.spaceBox, out: T.spaceOut },
          { box: [523, 1470, 1216, 1521], at: T.intern, out: T.manage },
          { box: [56, 1542, 707, 1593], at: T.manage, out: T.manageOut },
          { box: [1106, 1689, 1655, 1740], at: T.noMarker, kind: "marker" },
          { box: [59, 1761, 580, 1812], at: T.between, kind: "marker" },
        ]}
      />
      <SourcePill domain="x.com/lingxi" captured={CAPTURED} delay={T.lxPill} out={T.lxOut} style={{ position: "absolute", right: 1920 - 1653, top: 938 }} />
    </>
  );
};

/* ------------------------------------------------------------------ beats 7-8: Boris Cherny */

const BorisBeat: React.FC = () => {
  const f = useCurrentFrame();
  return (
    <>
      {f < T.b1Out + 10 ? (
        <LocalTweetCard
          src={`${P}boris-created.png`}
          srcW={1824}
          srcH={1071}
          x={139}
          y={108}
          w={1642}
          view={[0, 880]}
          delay={T.b1In}
          out={T.b1Out}
          outDur={8}
          push={{ from: T.b1In, to: T.b2In, s1: 1.03 }}
          focus={[{ at: 0, p: [432, 258] }]}
          highlights={[
            { box: [203, 81, 485, 132], at: T.borisBox, out: T.created },
            { box: [60, 237, 804, 280], at: T.created },
          ]}
        />
      ) : null}
      {f >= T.b2In - 1 ? (
        <LocalTweetCard
          src={`${P}boris-agents.png`}
          srcW={1824}
          srcH={891}
          x={139}
          y={270}
          w={1642}
          view={[0, 700]}
          delay={T.b2In}
          out={T.b2Out}
          outDur={9}
          push={{ from: T.b2In, to: T.b2Out + 9, s1: 1.03 }}
          focus={[{ at: 0, p: [850, 383] }]}
          highlights={[
            { box: [57, 357, 640, 408], at: T.experiment, kind: "underline", out: T.akoa },
            { box: [747, 357, 1265, 408], at: T.akoa },
          ]}
        />
      ) : null}
      <PersonLabel name="Boris Cherny" rest="· creator of Claude Code (his own words)" x={139} y={916} delay={T.label} out={T.b2Out} />
      <SourcePill domain="x.com/bcherny" captured={CAPTURED} delay={T.b1In} out={T.b2Out} style={{ position: "absolute", right: 139, top: 916 }} />
    </>
  );
};

/* ------------------------------------------------------------------ beats 9-10: the tip (generic agents) */

const L1: Seg[] = [
  [0, "working"],
  [T.wait, "waiting"],
  [T.keep, "working"],
];
const L1_BAR: [number, number][] = [
  [T.tipIn, 0.04],
  [T.wait, 0.42],
  [T.keep, 0.42],
  [T.dOut + 9, 0.78],
];
const R1: Seg[] = [
  [0, "idle"],
  [T.wait, "working"],
  [1056, "idle"],
];
const R1_BAR: [number, number][] = [
  [T.wait, 0],
  [T.answer, 1],
];

const TipBeat: React.FC = () => {
  const f = useCurrentFrame();
  const sp = prog(f, T.different, 12, EASE_IN_OUT) * 20;
  const lx = LEFT_CX - sp;
  const rx = RIGHT_CX + sp;
  const gapL = lx + NODE_W / 2;
  const gapR = rx - NODE_W / 2;
  const o = 1 - prog(f, T.dOut, 9, EASE_IN_OUT);
  // 'work' (130 wide): out of the left node, waits short of the right node until it has risen, then slides in under it.
  const workX = packetX(f, [
    [T.hand, gapL - 65],
    [T.hand + 14, 800, EASE_OUT],
    [T.another + 6, 1110, (t) => t],
    [T.another + 16, 1230 + 65, EASE_IN],
  ]);
  // 'answer' (170 wide) back along the bottom arrow.
  const ansX = packetX(f, [
    [T.answer - 2, 1230 + 85],
    [1058, 690 - 85],
  ]);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, transform: `translateY(${-(1 - o) * 14}px)`, filter: `blur(${(1 - o) * 6}px)` }}>
      <LeftKicker text="THE TIP" x={LEFT_CX - NODE_W / 2} y={112} delay={T.tipIn} out={T.dOut} />
      <WordLine
        words={[
          ["Hand", T.hand],
          ["off.", T.hand + 3],
          ["Wait.", T.wait],
          ["Keep", T.keep],
          ["going.", T.keep + 4],
        ]}
        x={LEFT_CX - NODE_W / 2 - 4}
        y={205}
        size={76}
        out={T.hOut}
      />
      {f >= T.h2In ? (
        <WordLine
          words={[
            ["Good", T.h2In],
            ["at", T.h2In + 2],
            ["something", T.h2In + 4],
            ["different", T.h2In + 7],
          ]}
          x={LEFT_CX - NODE_W / 2 - 4}
          y={205}
          size={76}
        />
      ) : null}
      {/* arrows */}
      <HArrow x1={gapL} x2={gapR} y={TOP_ARROW_Y} at={T.hand} dur={12} />
      <HArrow x1={gapR} x2={gapL} y={BOT_ARROW_Y} at={T.wait + 4} dur={10} />
      {/* packets, clipped to the gap between the nodes (they slide out from under one and in under the other) */}
      <div style={{ position: "absolute", left: gapL, top: 480, width: gapR - gapL, height: 200, overflow: "hidden" }}>
        {f >= T.hand && f <= T.another + 16 ? <Packet x={workX - gapL} y={TOP_ARROW_Y - 480} w={130} text="work" /> : null}
        {f >= T.answer - 2 && f <= 1058 ? <Packet x={ansX - gapL} y={BOT_ARROW_Y - 480} w={170} text="answer" /> : null}
      </div>
      {/* nodes */}
      <Reveal delay={T.tipIn} dur={12} style={{ position: "absolute", inset: 0 }}>
        <NodeBox cx={lx} label="One agent" segs={L1} bar={L1_BAR} lightAt={T.one} ring={{ color: C.accent, at: T.good }} />
      </Reveal>
      {f >= T.another ? (
        <Reveal delay={T.another} dur={12} style={{ position: "absolute", inset: 0 }}>
          <NodeBox cx={rx} label="Another agent" segs={R1} bar={R1_BAR} ring={{ color: C.mint, at: T.good }} />
        </Reveal>
      ) : null}
    </div>
  );
};

/* ------------------------------------------------------------------ beat 11: who is good at what (two proofs) */

const StackBeat: React.FC = () => {
  const f = useCurrentFrame();
  const z = geoZoom(prog(f, T.r1In, T.stackOut + 9 - T.r1In, EASE_IN_OUT), 1, 1.02);
  return (
    <div style={{ position: "absolute", inset: 0, transform: `scale(${z})`, transformOrigin: "960px 520px" }}>
      <LocalTweetCard
        src={`${P}grokbot-x.png`}
        srcW={1520}
        srcH={1323}
        x={124}
        y={95}
        w={1672}
        view={[0, 255]}
        delay={T.r1In}
        out={T.stackOut}
        outDur={9}
        push={{ from: T.r1In, to: T.stackOut, s1: 1 }}
        focus={[{ at: 0, p: [448, 217] }]}
        highlights={[{ box: [48, 196, 849, 238], at: T.full }]}
      />
      <SourcePill domain="x.com/bot" captured={CAPTURED} delay={T.r1In + 2} out={T.stackOut} style={{ position: "absolute", right: 1920 - 1796, top: 388 }} />
      {f >= T.r2In ? (
        <LocalBrowserCard url="tbench.ai/leaderboard" src={`${P}tb4-top.png`} x={72} y={475} w={1775} h={380} delay={T.r2In} out={T.stackOut} outDur={9}>
          <Highlight x={123} y={163} w={1600} h={54} delay={T.better1} dur={10} kind="box" pad={10} stroke={6} />
        </LocalBrowserCard>
      ) : null}
      {f >= T.r2In ? (
        <SourcePill
          domain="tbench.ai · Terminal-Bench 4.0"
          captured={CAPTURED}
          delay={T.r2In + 2}
          out={T.stackOut}
          style={{ position: "absolute", right: 1920 - 1847, top: 931 }}
        />
      ) : null}
    </div>
  );
};

/* ------------------------------------------------------------------ beats 12-15: the named handoff */

const CC: Seg[] = [
  [0, "idle"],
  [T.claude, "working"],
  [T.waits, "waiting"],
  [T.result, "check"],
  [T.result + 18, "working"],
  ...LOOPS.flatMap((c): Seg[] => [
    [c, "waiting"],
    [c + 54, "working"],
  ]),
];
const CC_BAR: [number, number][] = [
  [T.claude, 0],
  [T.waits, 0.5],
  [T.result + 18, 0.5],
  [T.loop, 0.62],
  [T.loop + 54, 0.62],
  [T.loop + 60, 0.68],
  [T.loop + 114, 0.68],
  [T.loop + 120, 0.74],
];
const GB: Seg[] = [
  [0, "idle"],
  [T.grok, "working"],
  [T.picks, "idle"],
  ...LOOPS.flatMap((c): Seg[] => [
    [c + 24, "working"],
    [c + 30, "idle"],
  ]),
];
const GB_BAR: [number, number][] = [
  [T.grok, 0],
  [T.picks, 1],
  ...LOOPS.flatMap((c): [number, number][] => [
    [c + 12, 1],
    [c + 13, 0],
    [c + 24, 0],
    [c + 30, 1],
  ]),
];

const HandoffBeat: React.FC = () => {
  const f = useCurrentFrame();
  const gapL = LEFT_CX + NODE_W / 2; // 690
  const gapR = RIGHT_CX - NODE_W / 2; // 1230
  // Me group: dims at 'installed', slides down + fades at 'stop'
  const dim = 1 - 0.6 * prog(f, T.installed, 12, EASE_IN_OUT);
  const meOut = prog(f, T.stop, 16, EASE_IN_OUT);
  // the first 'Twitter research' run (300 wide) and 'result' run (150 wide)
  const tr0 = packetX(f, [
    [T.hands, 540],
    [T.hands + 24, 960, EASE_OUT],
    [T.hands + 38, 990, (t) => t],
    [T.grok, 1380, EASE_IN],
  ]);
  const rs0 = packetX(f, [
    [T.picks, 1305],
    [T.result, 615],
  ]);
  const packets: React.ReactNode[] = [];
  if (f >= T.hands && f <= T.grok) packets.push(<Packet key="tr0" x={tr0 - gapL} y={TOP_ARROW_Y - 480} w={300} text="Twitter research" />);
  if (f >= T.picks && f <= T.result) packets.push(<Packet key="rs0" x={rs0 - gapL} y={BOT_ARROW_Y - 480} w={150} text="result" />);
  for (const c of LOOPS) {
    if (f >= c && f <= c + 24) {
      const x = packetX(f, [
        [c, 540],
        [c + 24, 1380],
      ]);
      packets.push(<Packet key={`tr${c}`} x={x - gapL} y={TOP_ARROW_Y - 480} w={300} text="Twitter research" />);
    }
    if (f >= c + 30 && f <= c + 54) {
      const x = packetX(f, [
        [c + 30, 1305],
        [c + 54, 615],
      ]);
      packets.push(<Packet key={`rs${c}`} x={x - gapL} y={BOT_ARROW_Y - 480} w={150} text="result" />);
    }
  }
  // snippets on the dotted links (no text): CC -> Me, Me -> GB, GB -> Me -> CC
  const snip = (from: number, dur: number, a: [number, number], b: [number, number], key: string) => {
    if (f < from || f > from + dur) return null;
    const [x, y] = along(a, b, prog(f, from, dur, EASE_IN_OUT));
    return <Packet key={key} x={x} y={y} w={40} />;
  };
  const waitsP = prog(f, T.waits, 10) * (1 - prog(f, T.result, 8, EASE_IN_OUT));
  return (
    <Reveal delay={T.d2In} dur={12} style={{ position: "absolute", inset: 0 }}>
      {/* Me group (links, snippets, node, label): under the main nodes */}
      <div style={{ position: "absolute", inset: 0, opacity: dim * (1 - meOut), transform: `translateY(${60 * meOut}px)` }}>
        <DottedLink a={LINK_L.a} b={LINK_L.b} at={T.links} />
        <DottedLink a={LINK_R.a} b={LINK_R.b} at={T.links} />
        {snip(T.copy, 12, LINK_L.a, LINK_L.b, "s1")}
        {snip(T.paste, 9, LINK_R.b, LINK_R.a, "s2")}
        {snip(T.paste + 8, 12, LINK_R.a, LINK_R.b, "s3")}
        {snip(T.paste + 20, 12, LINK_L.b, LINK_L.a, "s4")}
        <MeNode />
        <div
          style={{
            position: "absolute",
            left: ME.cx - 200,
            width: 400,
            top: 960 - 22,
            height: 44,
            textAlign: "center",
            fontFamily: FONT,
            fontSize: 34,
            fontWeight: 600,
            color: C.muted,
            opacity: prog(f, T.copy, 10),
            transform: `translateY(${(1 - prog(f, T.copy, 10)) * 10}px)`,
          }}
        >
          copy paste
        </div>
      </div>
      {/* arrows + stem */}
      <HArrow x1={gapL} x2={gapR} y={TOP_ARROW_Y} at={T.installed} dur={16} />
      <HArrow x1={gapR} x2={gapL} y={BOT_ARROW_Y} at={T.typeTo} dur={15} />
      {f >= T.skill ? <ChipStem at={T.skill + 4} /> : null}
      {/* packets, clipped to the gap between the nodes */}
      <div style={{ position: "absolute", left: gapL, top: 480, width: gapR - gapL, height: 200, overflow: "hidden" }}>{packets}</div>
      {/* nodes */}
      <NodeBox cx={LEFT_CX} label="Claude Code" segs={CC} bar={CC_BAR} lightAt={T.claude} ring={{ color: C.accent, at: T.d2In }} />
      <NodeBox cx={RIGHT_CX} label="Grok Bot" segs={GB} bar={GB_BAR} ring={{ color: C.mint, at: T.d2In }} pulse={{ color: C.mint, at: T.better2 }} />
      {/* 'waits' tag above Claude Code */}
      {waitsP > 0 ? (
        <div
          style={{
            position: "absolute",
            left: LEFT_CX - 80,
            width: 160,
            top: 432 - 23, // clears the node's outer ring (y 468) by ~13 px
            height: 46,
            display: "flex",
            justifyContent: "center",
            opacity: waitsP,
            transform: `translateY(${(1 - prog(f, T.waits, 10)) * 10}px)`,
          }}
        >
          <div
            style={{
              padding: "0 22px",
              height: 46,
              borderRadius: 23,
              background: C.dark,
              border: `2px solid ${C.darkLine}`,
              display: "flex",
              alignItems: "center",
              fontFamily: FONT,
              fontSize: 30,
              fontWeight: 600,
              color: C.inkSoft,
            }}
          >
            waits
          </div>
        </div>
      ) : null}
      {/* skill chip on top */}
      {f >= T.skill ? (
        <SkillChip at={T.skill} typeFrom={T.typeFrom} typeTo={T.typeTo} text="grokbot-webhook" pulses={[T.hands + 22, ...LOOPS.map((c) => c + 10)]} />
      ) : null}
    </Reveal>
  );
};

/** Closing caption: the pack's honest line in his own experience, with a marker under 'middleman'. */
const Caption: React.FC = () => {
  const f = useCurrentFrame();
  const p = prog(f, T.middleman, 12);
  const m = prog(f, T.agentsEnd, T.endOut - T.agentsEnd, EASE_IN_OUT); // done by f1806, before the end exit
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        width: 1920,
        top: 880 - 40,
        height: 80,
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        fontFamily: FONT,
        fontSize: 62,
        fontWeight: 800,
        letterSpacing: "-0.03em",
        color: C.ink,
        opacity: p,
        transform: `translateY(${(1 - p) * 28}px)`,
        filter: `blur(${(1 - p) * 8}px)`,
        whiteSpace: "pre",
      }}
    >
      <span>{"I've stopped being the "}</span>
      <span style={{ position: "relative", display: "inline-block" }}>
        <span
          style={{
            position: "absolute",
            left: -6,
            bottom: 2,
            height: 22,
            width: `calc((100% + 12px) * ${m})`,
            borderRadius: 6,
            background: C.accent,
            opacity: 0.55,
          }}
        />
        <span style={{ position: "relative" }}>middleman</span>
      </span>
    </div>
  );
};

/* ------------------------------------------------------------------ scene */

const Scene: React.FC = () => {
  const f = useCurrentFrame();
  const openUp = prog(f, T.openOut, 8, EASE_IN);
  // final exit: everything left on screen (diagram, chip, caption) is gone on f1815, the job's last frame
  const endO = 1 - prog(f, T.endOut, 1815 - T.endOut, EASE_IN_OUT);
  return (
    <VelourCanvas>
      {f < T.openOut + 10 ? (
        <div style={{ position: "absolute", inset: 0, transform: `translateY(${-30 * openUp}px)` }}>
          <TipOpener n={4} name="Agent Handoffs" delay={0} out={T.openOut} />
        </div>
      ) : null}
      {f < T.lxOut + 10 ? <LingxiBeat /> : null}
      {f >= T.b1In - 1 && f < T.b2Out + 10 ? <BorisBeat /> : null}
      {f >= T.tipIn - 1 && f < T.dOut + 10 ? <TipBeat /> : null}
      {f >= T.r1In - 1 && f < T.stackOut + 10 ? <StackBeat /> : null}
      {f >= T.d2In - 1 ? (
        <div style={{ position: "absolute", inset: 0, opacity: endO, transform: `translateY(${-(1 - endO) * 14}px)`, filter: endO < 1 ? `blur(${(1 - endO) * 6}px)` : undefined }}>
          <HandoffBeat />
        </div>
      ) : null}
      {f >= T.insetIn - 1 && f < T.insetOut + 10 ? (
        <>
          <LocalTweetCard
            src={`${P}grokbot-x.png`}
            srcW={1520}
            srcH={1323}
            x={191}
            y={40}
            w={1459}
            view={[0, 255]}
            delay={T.insetIn}
            out={T.insetOut}
            outDur={9}
            push={{ from: T.insetIn, to: T.insetOut + 9, s1: 1.02 }}
            focus={[{ at: 0, p: [448, 217] }]}
            highlights={[{ box: [48, 196, 849, 238], at: T.better2 }]}
          />
          <SourcePill domain="x.com/bot" captured={CAPTURED} delay={T.insetIn + 2} out={T.insetOut} style={{ position: "absolute", right: 1920 - 1650, top: 297 }} />
        </>
      ) : null}
      {f >= T.middleman - 1 ? (
        <div style={{ position: "absolute", inset: 0, opacity: endO, transform: `translateY(${-(1 - endO) * 14}px)`, filter: endO < 1 ? `blur(${(1 - endO) * 6}px)` : undefined }}>
          <Caption />
        </div>
      ) : null}
    </VelourCanvas>
  );
};

export default Scene;
