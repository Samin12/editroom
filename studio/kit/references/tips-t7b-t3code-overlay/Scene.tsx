import React from "react";
import { interpolateColors, useCurrentFrame } from "remotion";
import { VelourCanvas, C, prog, mix, geoZoom, EASE_IN_OUT, PANEL } from "../../../styles/velour/src";
import {
  Arrow,
  Box,
  BrowserBar,
  Enter,
  FONT,
  ImgRegion,
  LabelBar,
  LIFT,
  MarkerSet,
  NodeCard,
  Patch,
  RingBox,
  Shell,
  SrcPill,
  life,
  proofSrc,
  sb,
  withAlpha,
} from "./parts";

// anim-mvas6eyk: T7b "Stack Your Subscriptions, live in T3 Code" (scene 57c4, FRONT, project 527011-628266 ms,
// 3038 frames). A TRANSPARENT overlay over his real T3 Code screen recording (Borumi media 3e28). Every coordinate is
// a FINAL-FRAME pixel (source px -> 19.2 + s * 0.42472); everything stays inside x 21-1492, y 21-1060.
// Word times: tips exports/_work/writer/words/57c4.json (final). at(ms) = round((ms - 527011) * 30 / 1000) - 3.
// Footage events (menus, cuts, blurs) are REAL frames (ms - 527011) * 30 / 1000; every exit ends before the event.
// Zones, UI boxes and private regions: receipts/briefs/T7b.md (every box re-checked on Borumi render frames).
// Proof crops copied unaltered to public/opus-tips/proof/anim-mvas6eyk/ (regions are shown, never edited).
// Shared look: the DemoChip is copied exactly from T8b (anim-mvas6f1h, built first); the T-strip kickers use T8b's
// closing-kicker type (C.accent 23 px, 750, 0.16em, uppercase, no rule). Measured on stills: LIVE DEMO chip
// [440,44,659,92] -> grown [440,44,781,92]; 'No context switching' [930,44,1226,92]; RIGHT-CLICK chip [470,802,1057,850].
//
// BEATS (frame, word, what shows, proof)
//   f0      (start)          DemoChip "LIVE DEMO" rises into TOP (x 440, y 44).                                  footage
//   f53     "T3"             Chip grows to "LIVE DEMO · T3 Code" (gone f160). T3 CODE INTRO CARD rises into LOW at
//                              [600,660]: t3.codes hero "The open-source control plane for coding agents." (region
//                              [540,56,2260,827] at 0.39), pill t3.codes; 2% push from f65; leaves f206-216.
//                              13-t3codes-hero-crop.png (tip7-11b)
//   f183    "right"          Ring on the composer model chip [416,564,653,617].                                    footage
//   f206    (picker opens)   Ring moves onto the C2 + C1 account icons [418,216,481,324]; gone by f227 (union blur
//                              b01 starts real f228.6).                                                              footage
//   f228    "have"           ACCOUNTS LOUPE rises into LOW [436,676] (in by f240): his real rail crop (C2/C1 icons only,
//                              1.1x); its header "Two Claude Max accounts" (his words) fades in f234-f242 on "two".
//                              rail-claude-accounts-c2-c1.png (brief asset, his UI, extracted at media 7.21 s)
//   f238    "two"            Ring around both icons in the loupe.
//   f256    "Claude"         Label "Account 1" beside C1.    f278 "accounts": "Account 2" beside C2.
//   f278    "accounts"       T3 CODE DOCS CARD rises right of LOW [803,690] (label "T3 Code docs · providers-claude.md",
//                              region [102,985,1386,1222] at 0.52 + an 8 px blank-page patch on the right), pill
//                              github.com. f290 box on "Separate accounts or configurations"; "subscriptions" (real
//                              f365.2): box moves f357-363 to "for each account."
//                              14-t3code-multi-account-crop.png (tip7-11a)
//   f337    "switch"         Two-way curved arrow draws between the two icons in the loupe.
//   f392    ("It's")         Loupe and docs card leave (gone f400).
//   f401    "It's one place" PROVIDERS CARD rises into LOW [428,690]: t3.codes "Orchestrate Claude Code, Codex,
//                              Antigravity, OpenCode, Cursor, and / Grok from one surface. Bring your own subscription.
//                              Fork the whole thing." (region [408,850,2391,1030] at 0.53), pill t3.codes. ONE box
//                              travels: f414 "place" -> "from one surface."; f494 "open" -> "Codex,"; f525
//                              "subscription" -> "Bring your own subscription."; f587 "claude" -> "Claude Code,";
//                              f634 "Grok" -> "Grok"; f660 "Hermes": nothing moves; f689 "one" -> "from one surface."
//                              13-t3codes-hero-crop.png (tip7-11b, ~: honest provider line, no Hermes of ours)
//   f454    "context"        Chip "No context switching" in TOP-RIGHT [930,44]; leaves f700-708.
//   f494    "open"           Ring on the real OpenAI rail icon [424,167,474,214]; gone by f537.                   footage
//   f634    "Grok"           Ring on the real GR icon [418,327,481,378]; f676 "all": grows to the OpenAI-to-GR column
//                              [418,163,481,378] (never HE). LOW + rings leave f728-736 (b14 starts real f737.6).
//   f779    "right"          Ring on the red toast [540,37,1308,111]; fades f805-810 (no sweep over the URL below).
//   f810    "hit"            Ring draws on his in-thread error "Claude usage limit reached." [445,385,703,418];
//                              leaves f870-878.                                                                       footage
//   f821    "Claude"         HELP CENTER STRIP rises into MID [436,478]: "Both Pro and Max plans have a five-hour
//                              session limit and a weekly limit." (10b region [18,150,1566,228] at 0.6, cream capture
//                              in a dark frame; 1% push anchored on the period so 'limit.' never moves), pill
//                              support.claude.com. f840 "limits.": box on "five-hour session limit". Leaves f940-950.
//                              10b-claude-code-5h-session-crop.png (tip7-10b)
//   f913    "right"          Ring around the real right-click menu [223,700,433,1014].                            footage
//   f962    "click"          DemoChip "RIGHT-CLICK · Hand off to another model" in STRIP-B [470,802].
//   f991    "hand"           Ring tightens onto "Hand off to another model..." [228,705,429,737]; leaves f1012-1019;
//                              chip leaves f1020-1028 (dialog at real f1029.6).
//   f1030   ("and then")     T3.CODES SWITCH CARD rises into DLG-LOW [440,890]: "✓ Switch models mid-thread." (13b
//                              region [702,1065,1312,1145] at 0.9), pill t3.codes. f1063 "choose": box on it.
//                              13b-t3codes-switch-crop.png (tip7-11c)
//   f1075   "model,"         Ring on the Codex CLI model group [436,394,1060,547] (stroke never below y 551); gone f1106.
//   f1120   "which"          Ring on the Grok group [442,608,1074,773]; f1152 "want.": moves to "Start new thread"
//                              [880,801,1068,851]; ring and card gone f1170.                                         footage
//   f1192   "new"            Ring on the composer chip "Grok 4.7" [427,985,567,1029]; fades f1213-1218; f1219 "keep":
//                              a new ring draws on the ticking timer [408,429,599,463]; leaves f1240-1245.          footage
//   f1219   "keep"           Node "New thread keeps going" (his words) rises in Q [1160,700,1478,790].
//   f1245   "And this"       RECAP: node "Hit the usage limit" [440,700,760,790], arrow, node "Hand off to another
//                              model" [800,700,1120,790], arrow (staggered to f1270).                                concept
//   f1291   "handy"          Kicker "SO HANDY" in the T strip (x 430, y 380); node 3 glows.
//   f1398   "tokens"         One accent pulse: node 1 (f1398), arrow 1, node 2 brightens, arrow 2 (to f1418); f1418
//                              "madman": node 3 flashes.
//                              Everything leaves f1440-1447. No counts, numbers or ratios.
//   f1447   "Basically"      README CARD rises into Q [424,560]: github.com/pingdotgg/t3code README (rows 770-920 of
//                              the crop at 0.68, window x 100-1629, soft right edge), pill github.com.
//                              12-t3code-readme-crop.png (tip7-11d)
//   f1573   "phone"          Box on "mobile app (iOS, Android),"; it leaves before the pan.
//   f1599   "while"          Window pans right to x 1150-2679 (to f1640).
//   f1677   "control"        Box on "control of the agents on your machine". Card leaves f1738-1745.
//   f1771   "this (thing)"   SMALL REPO CARD in the S strip [430,766]: real snippets "t3code Public" + "MIT license"
//                              (11 crop at 0.62), pill github.com/pingdotgg/t3code. f1834 "source": box on "MIT
//                              license". Leaves f1960-1968.   11-t3code-repo-crop.png (tip7-11e)
//   f1866   "actually"       README LINE CARD in Q [418,606]: "have everything you need to fork and build the editor
//                              that you want." (12 region [100,1680,1640,1750] at 0.68), pill github.com. f1916 "build":
//                              box on "fork and build the editor that you want." Leaves f1962-1970.
//   f1976   "example"        SAMRABBIT CARD in Q [430,484]: github.com/Samin12/SamRabbit, label "His OS: SamRabbit",
//                              the About text (15 crop x 2880-3790, rows 650-1100 at 0.5), pill github.com; push.
//                              f2047 "operating": marker on "SamRabbit: an always-on voice / orchestrator".
//                              f2094 "Hopefully": one soft accent pulse on the card frame.
//                              f2222 "chats": marker moves to "T3 Code threads,".
//                              f2393-2412 "even has": window pans down to rows 866-1316 (topics).
//                              f2420 "voice": marker on the topic chip "realtime-voice".
//                              15-samrabbit-repo-crop.png (tip7-11f)
//   f2192   "manage"         CHECKLIST card [910,500] (his words), rows tick on their words: "Manage all my chats"
//                              f2192, "See what's going on" f2313, "Provide context" f2344, "Little voice mode" f2420.
//                              Card + checklist leave f2515-2523.
//   f2523   "highly"         Kicker "HIGHLY RECOMMEND" in the T strip; FREE CARD rises into Q [420,567]: t3.codes
//                              "T3 Code is free, open source and fast. Install it, plug in the harness / you already pay
//                              for, and let your agents get to work." (13c region [626,545,2312,720] at 0.62), pill
//                              t3.codes. f2582 "free": box on "T3 Code is free, open source and fast." (line 2 stays
//                              readable). Card leaves f2626-2637, kicker f2640-2650.   13c-t3codes-free-crop.png (tip7-11g)
//   f2638   ("you're")       CLOSING DIAGRAM: opaque card [410,476,1484,848] + hub "T3 Code" [1080,520,1380,610]
//                              (breathing glow).                                                                     concept
//   f2662   "power"          Kicker "FOR POWER USERS" in the T strip.
//   f2745   "way"            Under the hub: real crop "Bring your own subscription." (13 region [1050,945,1805,1025] at
//                              0.5) + pill t3.codes.   13-t3codes-hero-crop.png
//   f2798   "multiple"       Chips "Claude Max", "Claude Max", "OpenAI", "Grok" rise (3 f stagger, x 440-760);
//                              connectors draw into the hub.
//   f2856   "way"            One flow pulse along every connector; the hub flashes.
//   f2917   "always"         Tag "Always being used".   f2967 "context": tag "No context switching". Hold to f3037.
// v2 (review round 1): Help strip period never clipped (push anchored on it, cream margins both sides); 18-27 px of
// each capture's own background inside every proof card (intro, docs, providers, switch, README, README line, free);
// docs box 2 lands by f362; ring cross-fades chip -> timer; recap pulse starts on 'tokens'; repo card in at f1771.
// Kept: 'realtime-voice' topic chip at 0.5 (text ~17 px); the 'Little voice mode' checklist row carries the word.
// Honesty: no numbers of ours; no Hermes / Settings text; no ring on HE or the Hermes row; no 'beat the limit', 'full
// context', 'cloud', 'v1', '$', ratios or star counts; 'His OS: SamRabbit' (never 'built from scratch' or 'MIT').
// Privacy: nothing of ours on any blurred box or P11/P12/timezone line; no ring near an email; the account icons are
// ringed only at f206-226 (before the union blur); every LOW element is gone by f736 (hover cards from real f744).

const JOB_START = 527011;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;

/* --------------------------------------------------------------------------------------------- DemoChip (shared) */

/**
 * DemoChip, copied EXACTLY from T8b (anim-mvas6f1h, built first, owns it); only the text differs here.
 * C.paper box, 1.5 px C.line border, radius 12, height 48, padding 0 18. Lead ('TIP 8', 'LIVE DEMO') in the velour Kicker
 * type (C.accent, 26 px, 750, 0.16em, uppercase, no rule), then ' · ' and the body in C.ink 26 px 600, sentence case.
 * Enters with a 10 px rise + fade (10 f). A swap step: old text out (4 f, ending 1 f before `at`), new text in (5 f from
 * `at`), box width eases between them. A grow step (`grow: true`, body = old body + appended words): the box width
 * grows while the appended words fade in (8 f).
 * One allowance for T7b's opening ('LIVE DEMO' alone, then it grows to 'LIVE DEMO · T3 Code'): a lead-only step (empty
 * body) draws no separator, and on the grow the separator fades in with the appended words. Nothing else differs.
 */
export type ChipStep = { at: number; lead?: string; body: string; grow?: boolean };
const SEP = " · ";
const CHIP = { h: 48, padX: 18, border: 1.5, radius: 12, size: 26 };
const CHIP_FONT = "'Inter', system-ui, -apple-system, sans-serif";

let measureCtx: CanvasRenderingContext2D | null = null;
const textWidth = (text: string, weight: number, size: number, trackingEm = 0): number => {
  if (typeof document !== "undefined") {
    if (!measureCtx) measureCtx = document.createElement("canvas").getContext("2d");
    if (measureCtx) {
      measureCtx.font = `${weight} ${size}px Inter`;
      return measureCtx.measureText(text).width + trackingEm * size * text.length;
    }
  }
  return text.length * size * 0.56;
};
const chipBoxW = (s: ChipStep, size: number): number => {
  const lead = s.lead ? textWidth(s.lead.toUpperCase(), 750, size, 0.16) + (s.body ? textWidth(SEP, 600, size) : 0) : 0;
  return Math.ceil(lead + textWidth(s.body, 600, size)) + 2 * CHIP.padX + 2 * CHIP.border + 2;
};

const BodyText: React.FC<{ text: string }> = ({ text }) => (
  <>
    {text.split(SEP).map((seg, i) => (
      <React.Fragment key={i}>
        {i > 0 ? <span style={{ color: C.muted }}>{SEP}</span> : null}
        <span style={{ color: C.ink }}>{seg}</span>
      </React.Fragment>
    ))}
  </>
);

export const DemoChip: React.FC<{ x: number; y: number; steps: ChipStep[]; out?: number; outDur?: number; size?: number }> = ({
  x,
  y,
  steps,
  out,
  outDur = 8,
  size = CHIP.size,
}) => {
  const f = useCurrentFrame();
  const widths = React.useMemo(() => steps.map((s) => chipBoxW(s, size)), [steps, size]);
  const t0 = steps[0].at;
  if (f < t0) return null;
  const pIn = prog(f, t0, 10);
  const pOut = out != null ? prog(f, out, outDur, EASE_IN_OUT) : 0;
  if (pOut >= 1) return null;
  let w = widths[0];
  let idx = 0;
  for (let k = 1; k < steps.length; k++) {
    const s = steps[k];
    const g = s.grow ? prog(f, s.at, 8, EASE_IN_OUT) : prog(f, s.at - 5, 10, EASE_IN_OUT);
    w += (widths[k] - widths[k - 1]) * g;
    if (f >= s.at) idx = k;
  }
  const cur = steps[idx];
  let textO = idx > 0 && !cur.grow ? prog(f, cur.at, 5) : 1;
  const next = steps[idx + 1];
  if (next && !next.grow) textO *= 1 - prog(f, next.at - 5, 4);
  const appendFrom = cur.grow ? steps[idx - 1].body.length : cur.body.length;
  const appendO = cur.grow ? prog(f, cur.at, 8) : 1;
  const sepO = cur.grow && appendFrom === 0 ? appendO : 1;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: CHIP.h,
        boxSizing: "border-box",
        border: `${CHIP.border}px solid ${C.line}`,
        borderRadius: CHIP.radius,
        background: C.paper,
        padding: `0 ${CHIP.padX}px`,
        display: "flex",
        alignItems: "center",
        overflow: "hidden",
        opacity: pIn * (1 - pOut),
        transform: `translateY(${(1 - pIn) * 10 + pOut * 4}px)`,
        fontFamily: CHIP_FONT,
      }}
    >
      <div style={{ whiteSpace: "pre", fontSize: size, lineHeight: 1, fontWeight: 600, opacity: textO }}>
        {cur.lead ? (
          <>
            <span style={{ color: C.accent, fontWeight: 750, letterSpacing: "0.16em", textTransform: "uppercase" }}>{cur.lead}</span>
            {cur.body ? <span style={{ color: C.muted, opacity: sepO }}>{SEP}</span> : null}
          </>
        ) : null}
        <BodyText text={cur.body.slice(0, appendFrom)} />
        {appendFrom < cur.body.length ? (
          <span style={{ opacity: appendO }}>
            <BodyText text={cur.body.slice(appendFrom)} />
          </span>
        ) : null}
      </div>
    </div>
  );
};

/* ---------------------------------------------------------------------------------------------- timing */

const T = {
  // A: live demo + T3 Code intro
  liveGrow: at(528873), // "T3" 53
  liveOut: 150, // gone 160 (10 f)
  introIn: at(528873), // 53
  introOut: 206, // gone 216
  chipRing: at(533198), // "right" 183
  railMorph: 206, // picker's first frame (real f205.4)
  railRingOut: 221, // gone 226 (5 f), union blur from real f228.6
  // B: accounts
  loupeIn: 228, // 'have': the header fades in from f234 while the card is still rising, so no empty header band
  two: at(535040), // 238
  acc1: at(535648), // "Claude" 256
  acc2: at(536362), // "accounts" 278
  docsIn: at(536362), // 278
  docsHl1: 290,
  sw: at(538344), // "switch" 337
  docsHl2: at(539185), // "subscriptions" 362 (real 365.2): the box moves f357-363, so it has landed by f362
  bOut: 392, // gone 400
  // C: providers
  provIn: 401,
  place: at(540907), // 414
  ctx: at(542228), // "context" 454
  open: at(543570), // 494
  sub: at(544608), // 525
  claude: at(546673), // 587
  grok: at(548235), // 634
  all: at(549648), // 676
  one: at(550077), // 689
  openRingOut: 530, // gone 537
  ctxOut: 700, // gone 708
  cOut: 728, // gone 736
  // D: usage limit
  toast: at(553061), // "right" 779
  toastOut: 805, // gone 810
  hit: at(554102), // 810
  errOut: 870, // gone 878
  help: at(554482), // "Claude" 821
  limits: at(555103), // 840
  helpOut: 940, // gone 950
  // E: right-click menu
  menu: 913,
  click: at(559168), // 962
  hand: at(560149), // 991
  menuOut: 1012, // gone 1019
  clickOut: 1020, // gone 1028
  // F: hand-off dialog
  swIn: 1030,
  choose: at(562551), // 1063
  model: at(562952), // 1075
  codexOut: 1098, // gone 1106
  which: at(564434), // 1120
  want: at(565495), // 1152
  fOut: 1162, // gone 1170
  // G: new thread
  newT: at(566856), // 1192
  keep: at(567757), // 1219
  timerOut: 1240, // gone 1245
  // H: recap
  rIn: 1245,
  handy: at(570140), // 1291
  tokens: at(573718), // 1398 (real 1401.2): the pulse starts on node 1 here
  madman: at(574385), // 1418
  rOut: 1440, // gone 1447
  // I: README / phone
  readmeIn: 1447,
  phone: at(579551), // 1573
  phoneOut: 1593,
  panFrom: at(580424), // "while" 1599
  panTo: 1640, // "active"
  control: at(582995), // 1677
  readmeOut: 1738, // gone 1745
  // J: open source
  repoIn: at(586138), // "this (thing)" 1771: shortens the quiet stretch after the README card (gone 1745)
  whole: at(586919), // 1794
  source: at(588241), // 1834
  repoOut: 1960, // gone 1968
  lineIn: at(589302), // "actually" 1866
  build: at(590978), // 1916
  lineOut: 1962, // gone 1970
  // K: SamRabbit
  srIn: at(592986), // "example" 1976
  operating: at(595329), // 2047
  hopefully: at(596908), // 2094
  manage: at(600194), // 2192
  chats: at(601175), // 2222
  see: at(604199), // 2313
  provide: at(605240), // 2344
  srPanFrom: at(606862), // "even" 2393
  srPanTo: 2412,
  voice: at(607783), // 2420
  srOut: 2515, // gone 2523
  // L: free
  highly: at(611204), // 2523
  free: at(613189), // 2582
  freeOut: 2626, // gone 2638
  highlyOut: 2640, // gone 2650
  // M: closing
  closeIn: 2638,
  power: at(615832), // 2662
  way: at(618595), // 2745
  multiple: at(620377), // 2798
  way2: at(622319), // 2856
  always: at(624342), // 2917
  context2: at(626003), // 2967
};

/* -------------------------------------------------------------------------------------------- sources */

const HERO = { src: proofSrc("13-t3codes-hero-crop.png"), w: 2800 };
const DOCS = { src: proofSrc("14-t3code-multi-account-crop.png"), w: 2830, h: 1700 };
const RAIL = { src: proofSrc("rail-claude-accounts-c2-c1.png"), w: 140 };
const HELP = { src: proofSrc("10b-claude-code-5h-session-crop.png"), w: 2580 };
const SWITCH = { src: proofSrc("13b-t3codes-switch-crop.png"), w: 3600 };
const README = { src: proofSrc("12-t3code-readme-crop.png"), w: 2830 };
const REPO = { src: proofSrc("11-t3code-repo-crop.png"), w: 3840 };
const SAMR = { src: proofSrc("15-samrabbit-repo-crop.png"), w: 3840 };
const FREE = { src: proofSrc("13c-t3codes-free-crop.png"), w: 2940 };

/** The card Shell is border-box (1.5 px border each side): an image card's Shell is the region width + 3, so no
 * column of the capture is hidden under the right border (v1 lost ~3 px there). */
const SHELL_B = 3;

/** A proof highlight inside an ImgRegion: an accent box on a measured source box, padded, moving key to key. */
const ProofBox: React.FC<{ s: number; keys: { at: number; box: Box }[]; out: number; outDur?: number; pad?: number; stroke?: number; morph?: number }> = ({
  s,
  keys,
  out,
  outDur = 7,
  pad = 5,
  stroke = 3.5,
  morph = 8,
}) => <RingBox keys={keys.map((k) => ({ at: k.at, box: sb(k.box, s) }))} out={out} outDur={outDur} pad={pad} stroke={stroke} radius={8} morph={morph} />;

/**
 * An opaque full-width sheet in the Q band (x 404-1484). Its top and bottom edges sit in the gaps between the agent's
 * text lines (lines at y 490-510, 535-590, 615-635, 660-715, 737-762), so no cut line or stray letter peeks out.
 * It can grow to a new rect (8 frames) before new content enters.
 */
const Sheet: React.FC<{ at: number; out?: number; outDur?: number; keys: { at: number; rect: Box }[] }> = ({ at: a, out, outDur = 7, keys }) => {
  const f = useCurrentFrame();
  const p = prog(f, a, 10);
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  let r: number[] = [...keys[0].rect];
  for (let i = 1; i < keys.length; i++) {
    const t = prog(f, keys[i].at, 8, EASE_IN_OUT);
    if (t <= 0) break;
    r = r.map((v, k) => mix(t, v, keys[i].rect[k]));
  }
  return (
    <div
      style={{
        position: "absolute",
        left: r[0],
        top: r[1],
        width: r[2] - r[0],
        height: r[3] - r[1],
        boxSizing: "border-box",
        borderRadius: 18,
        background: C.paper,
        border: `1.5px solid ${C.line}`,
        boxShadow: LIFT,
        opacity: p * o,
        transform: `translateY(${(1 - p) * 8}px)`,
      }}
    />
  );
};
const SHEET_I: Box = [404, 522, 1484, 733];
const SHEET_J: Box = [404, 598, 1484, 733];
const SHEET_Q: Box = [404, 476, 1484, 848];

/* ---------------------------------------------------------------------------------------------- beats */

/** A: T3 Code intro card (LOW), f53-216. */
const IntroCard: React.FC = () => {
  const s = 0.39;
  // ink x 597-2204, y 98-786 (scanned): ~22 px of the page's own background on each side and ~16 px above and below,
  // so after the 2% centred push the headline still sits ~15 px inside the card and clear of the URL bar.
  const region: Box = [540, 56, 2260, 827];
  const w = (region[2] - region[0]) * s;
  return (
    <Enter at={T.introIn} out={T.introOut} outDur={10} x={600} y={660}>
      <Shell w={w + SHELL_B}>
        <BrowserBar url="t3.codes" />
        <ImgRegion src={HERO.src} srcW={HERO.w} scale={s} region={region} push={{ from: 65, to: T.introOut + 10, z: 1.02, focus: [1400, 441] }} />
      </Shell>
      <SrcPill domain="t3.codes" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** B: accounts loupe (LOW left), f218-400. His real rail crop at 1.1x, a ring, two labels and a two-way arrow. */
const Loupe: React.FC = () => {
  const f = useCurrentFrame();
  const k = 1.1;
  const headO = prog(f, T.two - 4, 8);
  const cw = 140 * k;
  const ch = 250 * k;
  const l1 = prog(f, T.acc1, 8);
  const l2 = prog(f, T.acc2, 8);
  // icon centres in the crop (asset px): C2 [38,30,106,95], C1 [38,156,106,222]
  const c2y = 62.5 * k;
  const c1y = 189 * k;
  const ringBox: Box = [32 * k, 24 * k, 112 * k, 228 * k];
  // two-way arrow in the gap between the icons (inside the ring)
  const ax = 78 * k;
  const ay0 = 101 * k;
  const ay1 = 150 * k;
  const ad = prog(f, T.sw, 8);
  const apath = `M ${ax} ${ay0} C ${ax + 16} ${ay0 + 12}, ${ax + 16} ${ay1 - 12}, ${ax} ${ay1}`;
  const alen = 64;
  const tipVis = (t: number) => (ad >= t ? 1 : 0);
  // arrow heads from the curve's end tangents (start tangent (16, 12), end tangent (-16, 12)), wings at +-30 deg
  const wing = (tx: number, ty: number, dx: number, dy: number) => {
    const n = Math.hypot(dx, dy);
    const ux = dx / n;
    const uy = dy / n;
    const r = (a: number) => [ux * Math.cos(a) - uy * Math.sin(a), ux * Math.sin(a) + uy * Math.cos(a)];
    const [a1, b1] = r(Math.PI / 6);
    const [a2, b2] = r(-Math.PI / 6);
    return `M ${tx + 10 * a1} ${ty + 10 * b1} L ${tx} ${ty} L ${tx + 10 * a2} ${ty + 10 * b2}`;
  };
  const label = (o: number): React.CSSProperties => ({
    position: "absolute",
    left: cw + 14,
    fontSize: 26,
    fontWeight: 650,
    color: C.ink,
    whiteSpace: "nowrap",
    opacity: o,
    transform: `translateX(${(1 - o) * -10}px)`,
  });
  return (
    <Enter at={T.loupeIn} out={T.bOut} x={436} y={676}>
      <Shell w={352} style={{ padding: "12px 16px 14px", boxSizing: "border-box" }}>
        <div style={{ height: 34, fontFamily: FONT, fontSize: 25, fontWeight: 700, color: C.ink, whiteSpace: "nowrap", opacity: headO, letterSpacing: "-0.01em" }}>
          Two Claude Max accounts
        </div>
        <div style={{ position: "relative", height: ch, marginTop: 6, fontFamily: FONT }}>
          <div style={{ position: "absolute", left: 0, top: 0, width: cw, height: ch, borderRadius: 10, overflow: "hidden", border: `1.5px solid ${C.line}`, boxSizing: "border-box" }}>
            <ImgRegion src={RAIL.src} srcW={RAIL.w} scale={k} region={[0, 0, 140, 250]} />
          </div>
          <RingBox keys={[{ at: T.two, box: ringBox }]} out={T.bOut + 4} outDur={4} stroke={3.5} radius={14} />
          {ad > 0 ? (
            <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }} width={1} height={1}>
              <path d={apath} fill="none" stroke={C.accent} strokeWidth={3.5} strokeLinecap="round" strokeDasharray={alen} strokeDashoffset={alen * (1 - ad)} />
              <path d={wing(ax, ay0, 16, 12)} fill="none" stroke={C.accent} strokeWidth={3.5} strokeLinecap="round" strokeLinejoin="round" opacity={tipVis(0.3)} />
              <path d={wing(ax, ay1, 16, -12)} fill="none" stroke={C.accent} strokeWidth={3.5} strokeLinecap="round" strokeLinejoin="round" opacity={tipVis(0.95)} />
            </svg>
          ) : null}
          <div style={{ ...label(l2), top: c2y - 16 }}>Account 2</div>
          <div style={{ ...label(l1), top: c1y - 16 }}>Account 1</div>
        </div>
      </Shell>
    </Enter>
  );
};

/** B: T3 Code docs card (LOW right), f278-400. */
const DocsCard: React.FC = () => {
  const s = 0.52;
  // ink x 137-1371 (the hr under the heading starts at 136); the next word 'This' starts at x 1388, so the region's
  // right edge stays at 1386. Inner padding: 35 src px of blank page on the left (18 px), and on the right an 8 px
  // Patch of the blank columns 1373-1385 of the same rows (the hr continues through it), so the 'for each account.'
  // box sits ~11 px inside the card. The push is anchored on the right end of 'for each account.' so that edge never
  // moves; the left margin absorbs it (~8 px).
  const f = useCurrentFrame();
  const region: Box = [102, 985, 1386, 1222];
  const W = (region[2] - region[0]) * s;
  const H = (region[3] - region[1]) * s;
  const padR = 8;
  const push = { from: T.docsHl1, to: T.bOut + 8, z: 1.012, focus: [1371, 1186] as [number, number] };
  // the patch rides the same push (same scale, same focus) so the hr under the heading stays one continuous line
  const z = geoZoom(prog(f, push.from, push.to - push.from, EASE_IN_OUT), 1, push.z);
  const fx = (push.focus[0] - region[0]) * s;
  const fy = (push.focus[1] - region[1]) * s;
  return (
    <Enter at={T.docsIn} out={T.bOut} x={803} y={690}>
      <Shell w={W + padR + SHELL_B}>
        <LabelBar>
          <span style={{ color: C.ink }}>T3 Code docs</span>
          <span>&nbsp;· providers-claude.md</span>
        </LabelBar>
        <div style={{ position: "relative", width: W + padR, height: H }}>
          <div style={{ position: "absolute", left: W - 3, top: 0, width: padR + 3, height: H, overflow: "hidden" }}>
            <Patch
              src={DOCS.src}
              srcW={DOCS.w}
              srcH={DOCS.h}
              s={s}
              patch={[1373, 985, 1385, 1222]}
              w={padR + 3}
              style={{ left: 0, top: 0, transform: `scale(${z})`, transformOrigin: `${fx - (W - 3)}px ${fy}px` }}
            />
          </div>
          <ImgRegion src={DOCS.src} srcW={DOCS.w} scale={s} region={region} push={push}>
            <ProofBox
              s={s}
              pad={3}
              stroke={3}
              morph={6}
              keys={[
                { at: T.docsHl1, box: [137, 1003, 1318, 1069] },
                { at: T.docsHl2 - 5, box: [995, 1164, 1371, 1209] },
              ]}
              out={T.bOut + 2}
            />
          </ImgRegion>
        </div>
      </Shell>
      <SrcPill domain="github.com" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** C: providers card (LOW), f401-736, one box travelling on his words. */
const ProvidersCard: React.FC = () => {
  const s = 0.53;
  // ink-scanned line boxes: line 1 y 868-922, line 2 y 958-1012 (commas and periods included); ink x 453-2346.
  // 45 src px (24 px) of the page's own background on each side; with the 1% centred push 'Grok' and 'thing.' stay
  // ~19 px inside the card and the Grok box ~13 px.
  const region: Box = [408, 850, 2391, 1030];
  const w = (region[2] - region[0]) * s;
  const surface: Box = [591, 958, 1043, 1012];
  return (
    <Enter at={T.provIn} out={T.cOut} x={428} y={690}>
      <Shell w={w + SHELL_B}>
        <BrowserBar url="t3.codes" />
        <ImgRegion src={HERO.src} srcW={HERO.w} scale={s} region={region} push={{ from: T.place, to: T.cOut + 8, z: 1.01, focus: [1400, 940] }}>
          <ProofBox
            s={s}
            pad={4}
            keys={[
              { at: T.place, box: surface },
              { at: T.open, box: [1196, 868, 1370, 922] },
              { at: T.sub, box: [1064, 958, 1794, 1012] },
              { at: T.claude, box: [831, 868, 1177, 922] },
              { at: T.grok, box: [454, 958, 574, 1012] },
              { at: T.one, box: surface },
            ]}
            out={T.cOut + 2}
          />
        </ImgRegion>
      </Shell>
      <SrcPill domain="t3.codes" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** D: Claude Help Center strip (MID), f821-950. Light capture in a dark frame. */
const HelpStrip: React.FC = () => {
  const s = 0.6;
  // ink x 49-1551 ('Both' ... 'limit.'); columns 0-48 and 1552-1567 are blank cream, 'Max' starts at 1568.
  // Region 18-1566: 9 px of cream right of the period, 19 px left of 'Both'. The push is anchored on the period's
  // right end (focus x 1551), so the period NEVER moves; the 1% push spends ~9 px of the left margin (ends ~10 px).
  const region: Box = [18, 150, 1566, 228];
  const w = (region[2] - region[0]) * s;
  return (
    <Enter at={T.help} out={T.helpOut} outDur={10} x={436} y={478}>
      <Shell w={w + SHELL_B}>
        <LabelBar h={32}>
          <span style={{ color: C.ink }}>Claude Help Center</span>
        </LabelBar>
        <ImgRegion src={HELP.src} srcW={HELP.w} scale={s} region={region} push={{ from: T.limits, to: T.helpOut + 10, z: 1.01, focus: [1551, 189] }}>
          <ProofBox s={s} pad={4} keys={[{ at: T.limits, box: [696, 167, 1157, 211] }]} out={T.helpOut + 3} />
        </ImgRegion>
      </Shell>
      <SrcPill domain="support.claude.com" style={{ marginTop: 6 }} />
    </Enter>
  );
};

/** F: t3.codes "Switch models mid-thread." (DLG-LOW), f1030-1170. */
const SwitchCard: React.FC = () => {
  const s = 0.9;
  // ink: the check icon from x 732, 'Switch models mid-thread.' to 1282 (other page ink ends 652 and starts 1362):
  // ~27 px of blank page each side.
  const region: Box = [702, 1065, 1312, 1145];
  const w = (region[2] - region[0]) * s;
  return (
    <Enter at={T.swIn} out={T.fOut} dur={10} x={440} y={890}>
      <Shell w={w + SHELL_B}>
        <LabelBar h={32}>
          <span style={{ color: C.ink }}>t3.codes</span>
        </LabelBar>
        <ImgRegion src={SWITCH.src} srcW={SWITCH.w} scale={s} region={region} push={{ from: T.choose, to: T.fOut + 8, z: 1.02, focus: [1038, 1105] }}>
          <ProofBox s={s} pad={4} keys={[{ at: T.choose, box: [795, 1087, 1282, 1123] }]} out={T.fOut + 2} />
        </ImgRegion>
      </Shell>
      <SrcPill domain="t3.codes" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** H: the recap diagram on his own demo (lower Q), f1219-1447. */
const Recap: React.FC = () => {
  const f = useCurrentFrame();
  const out = T.rOut;
  // node 3 glows from "handy", flashes at "madman"
  const n3lit = prog(f, T.handy, 10);
  const flash3 = prog(f, T.madman, 4) * (1 - prog(f, T.madman + 4, 12, EASE_IN_OUT));
  const glow3 = n3lit * (0.55 + 0.25 * Math.sin(((f - T.handy) / 45) * 2 * Math.PI)) + flash3 * 0.9;
  // the pulse (keyed to 'tokens', real f1401.2): node 1 lights (f1398), arrow 1 (f1401-1407), node 2 brightens
  // (f1406-1416), arrow 2 (f1412-1418), node 3 flashes on 'madman' (f1418)
  const n1 = prog(f, T.tokens, 3) * (1 - prog(f, T.tokens + 5, 6, EASE_IN_OUT));
  const flow1 = prog(f, T.tokens + 3, 6, EASE_IN_OUT);
  const n2 = prog(f, T.tokens + 8, 3) * (1 - prog(f, T.tokens + 12, 6, EASE_IN_OUT));
  const flow2 = prog(f, T.tokens + 14, 6, EASE_IN_OUT);
  const fade1 = 1 - prog(f, T.madman + 6, 10, EASE_IN_OUT);
  return (
    <>
      <NodeCard box={[440, 700, 760, 790]} text="Hit the usage limit" at={T.rIn} out={out} lit={n1} glow={n1 * 0.8} />
      <Arrow from={[766, 745]} to={[796, 745]} at={T.rIn + 8} dur={6} out={out} flow={flow1 * fade1} />
      <NodeCard box={[800, 700, 1120, 790]} text="Hand off to another model" at={T.rIn + 12} out={out} lit={n2} glow={n2 * 0.8} />
      <Arrow from={[1126, 745]} to={[1156, 745]} at={T.rIn + 20} dur={6} out={out} flow={flow2 * fade1} />
      <NodeCard box={[1160, 700, 1478, 790]} text="New thread keeps going" at={T.keep} out={out} lit={n3lit} glow={glow3} />
    </>
  );
};

/** T-strip kicker: the same look as T8b's closing kicker (C.accent, 23 px, 750, 0.16em, uppercase, no rule), with a
 * 6 px rise + fade in (8 f) and a fade out (10 f). Sits at y 383-409 in the empty strip above the timer. */
const StripKicker: React.FC<{ text: string; at: number; out?: number }> = ({ text, at: a, out }) => {
  const f = useCurrentFrame();
  const p = prog(f, a, 8);
  const o = out == null ? 1 : 1 - prog(f, out, 10, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: 430,
        top: 383,
        fontFamily: CHIP_FONT,
        fontSize: 23,
        lineHeight: "26px",
        fontWeight: 750,
        letterSpacing: "0.16em",
        textTransform: "uppercase",
        color: C.accent,
        whiteSpace: "nowrap",
        opacity: p * o,
        transform: `translateY(${(1 - p) * 6}px)`,
      }}
    >
      {text}
    </div>
  );
};

/** I: README card (Q), f1447-1745, with a pan from the phone line to the control line. */
const ReadmeCard: React.FC = () => {
  const f = useCurrentFrame();
  const s = 0.68;
  // ink starts at x 137: 25 px of blank page left of 'mobile app' (the box sits ~19 px inside the card)
  const region: Box = [100, 770, 1629, 920];
  const region2: Box = [1150, 770, 2679, 920];
  const w = (region[2] - region[0]) * s;
  const pt = prog(f, T.panFrom, T.panTo - T.panFrom, EASE_IN_OUT);
  return (
    <Enter at={T.readmeIn} out={T.readmeOut} outDur={7} x={424} y={536}>
      <Shell w={w + SHELL_B}>
        <BrowserBar url="github.com/pingdotgg/t3code/blob/main/README.md" />
        <div style={{ background: C.paperEdge }}>
          <ImgRegion
            src={README.src}
            srcW={README.w}
            scale={s}
            region={region}
            pan={{ from: T.panFrom, to: T.panTo, region: region2 }}
            fadeL={56 * pt}
            fadeR={56}
          >
            <ProofBox s={s} pad={4} keys={[{ at: T.phone, box: [138, 858, 705, 906] }]} out={T.phoneOut} outDur={6} />
            <ProofBox s={s} pad={4} keys={[{ at: T.control, box: [1387, 786, 2207, 831] }]} out={T.readmeOut + 1} />
          </ImgRegion>
        </div>
      </Shell>
      <SrcPill domain="github.com" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** J: small repo card (S strip), f1771-1968: two real snippets of the repo page, pill inside. */
const RepoCard: React.FC = () => {
  const s = 0.62;
  const head: Box = [95, 340, 560, 418];
  const about: Box = [2920, 846, 3232, 912];
  return (
    <Enter at={T.repoIn} out={T.repoOut} dur={10} x={430} y={766}>
      <Shell w={640} style={{ padding: "4px 12px 5px", boxSizing: "border-box" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 14, height: 48 }}>
          <div style={{ borderRadius: 6, overflow: "hidden" }}>
            <ImgRegion src={REPO.src} srcW={REPO.w} scale={s} region={head} />
          </div>
          <div style={{ width: 1.5, height: 36, background: C.line }} />
          <div style={{ borderRadius: 6, overflow: "hidden" }}>
            <ImgRegion src={REPO.src} srcW={REPO.w} scale={s} region={about}>
              <ProofBox s={s} pad={3} stroke={3} keys={[{ at: T.source, box: [3003, 858, 3218, 901] }]} out={T.repoOut + 2} />
            </ImgRegion>
          </div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 9, height: 26, marginTop: 1, fontFamily: FONT, fontSize: 22, fontWeight: 500, color: C.muted, whiteSpace: "nowrap" }}>
          <div style={{ width: 8, height: 8, borderRadius: 4, background: C.accent }} />
          github.com/pingdotgg/t3code · captured Oct 16, 2026
        </div>
      </Shell>
    </Enter>
  );
};

/** J: README line card (Q), f1866-1970. */
const ReadmeLine: React.FC = () => {
  const s = 0.68;
  // ink x 139-1597 (blank page from 60 to 1900 on these rows): ~27 px each side; the 1.2% push toward the phrase
  // leaves 'have' ~18 px and the box ~20 px inside the card.
  const region: Box = [100, 1680, 1640, 1750];
  const w = (region[2] - region[0]) * s;
  return (
    <Enter at={T.lineIn} out={T.lineOut} x={418} y={606}>
      <Shell w={w + SHELL_B}>
        <LabelBar h={32}>
          <span style={{ color: C.ink }}>T3 Code README</span>
        </LabelBar>
        <ImgRegion src={README.src} srcW={README.w} scale={s} region={region} push={{ from: T.lineIn + 12, to: T.lineOut + 8, z: 1.012, focus: [1180, 1714] }}>
          <ProofBox s={s} pad={4} keys={[{ at: T.build, box: [762, 1692, 1598, 1738] }]} out={T.lineOut + 2} />
        </ImgRegion>
      </Shell>
      <SrcPill domain="github.com" style={{ marginTop: 6 }} />
    </Enter>
  );
};

/** K: SamRabbit card (Q left), f1976-2523. */
const SamRabbitCard: React.FC = () => {
  const f = useCurrentFrame();
  const s = 0.5;
  const region: Box = [2880, 650, 3790, 1100];
  const region2: Box = [2880, 866, 3790, 1316];
  const w = (region[2] - region[0]) * s;
  const pulse = prog(f, T.hopefully, 6) * (1 - prog(f, T.hopefully + 8, 14, EASE_IN_OUT));
  const edge = interpolateColors(pulse, [0, 1], [C.line, C.accent]);
  return (
    <Enter at={T.srIn} out={T.srOut} x={424} y={506}>
      <Shell w={w + SHELL_B} edge={edge} style={{ boxShadow: `${LIFT}, 0 0 ${22 * pulse}px ${5 * pulse}px ${withAlpha(C.accent, 0.4 * pulse)}` }}>
        <BrowserBar url="github.com/Samin12/SamRabbit" />
        <ImgRegion
          src={SAMR.src}
          srcW={SAMR.w}
          scale={s}
          region={region}
          pan={{ from: T.srPanFrom, to: T.srPanTo, region: region2 }}
          push={{ from: T.srIn + 12, to: T.srOut + 8, z: 1.025, focus: [3250, 900] }}
        >
          <MarkerSet
            s={s}
            out={T.srOut}
            keys={[
              {
                at: T.operating,
                rects: [
                  [2930, 670, 3603, 715],
                  [2930, 742, 3195, 787],
                ],
              },
              { at: T.chats, rects: [[2930, 815, 3302, 859]], out: T.srPanFrom - 5 },
              { at: T.voice, rects: [[2950, 1232, 3240, 1288]] },
            ]}
          />
        </ImgRegion>
      </Shell>
      <SrcPill domain="github.com" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** K: checklist (Q right), his words, the card grows row by row, f2192-2523. */
const CHECKS: { text: string; at: number }[] = [
  { text: "Manage all my chats", at: T.manage },
  { text: "See what's going on", at: T.see },
  { text: "Provide context", at: T.provide },
  { text: "Little voice mode", at: T.voice },
];
const Checks: React.FC = () => {
  const f = useCurrentFrame();
  const pitch = 60;
  const top = 578;
  return (
    <Enter at={T.srIn} out={T.srOut} x={912} y={508}>
      <div style={{ fontFamily: FONT, fontSize: 36, fontWeight: 800, letterSpacing: "-0.02em", color: C.ink, whiteSpace: "nowrap" }}>
        His OS: <span style={{ color: C.accent }}>SamRabbit</span>
      </div>
      <div style={{ width: 120, height: 3, borderRadius: 2, background: C.line, marginTop: 14 }} />
      {CHECKS.map((c, i) => {
        const p = prog(f, c.at, 8);
        const tick = prog(f, c.at + 5, 9);
        if (p <= 0) return null;
        return (
          <div
            key={c.text}
            style={{
              position: "absolute",
              left: 0,
              top: top - 508 + i * pitch,
              height: pitch,
              display: "flex",
              alignItems: "center",
              gap: 16,
              opacity: p,
              transform: `translateX(${(1 - p) * -14}px)`,
              fontFamily: FONT,
            }}
          >
            <svg width={32} height={32} viewBox="0 0 48 48">
              <circle cx={24} cy={24} r={22} fill={C.accent} />
              <path
                d="M14 24.5 21 31.5 34.5 17"
                fill="none"
                stroke={C.white}
                strokeWidth={5}
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeDasharray={32}
                strokeDashoffset={32 * (1 - tick)}
              />
            </svg>
            <div style={{ fontSize: 30, fontWeight: 650, color: C.ink, letterSpacing: "-0.01em", whiteSpace: "nowrap" }}>{c.text}</div>
          </div>
        );
      })}
    </Enter>
  );
};

/** L: free card (Q), f2523-2638. */
const FreeCard: React.FC = () => {
  const s = 0.62;
  // ink x 658-2280 (scanned): ~20 px of the page's own background each side; a 1.2% push focused mid-line keeps
  // 'T3' and 'harness' ~13 px inside the card to the end. The card sits centred on the Q sheet (insets 16/16).
  const region: Box = [626, 545, 2312, 720];
  const w = (region[2] - region[0]) * s;
  return (
    <Enter at={T.highly} out={T.freeOut} outDur={12} x={420} y={567}>
      <Shell w={w + SHELL_B}>
        <BrowserBar url="t3.codes" />
        <ImgRegion src={FREE.src} srcW={FREE.w} scale={s} region={region} push={{ from: T.highly + 12, to: T.freeOut + 12, z: 1.012, focus: [1400, 590] }}>
          <ProofBox s={s} pad={4} keys={[{ at: T.free, box: [657, 565, 1584, 616] }]} out={T.freeOut + 4} outDur={8} />
        </ImgRegion>
      </Shell>
      <SrcPill domain="t3.codes" style={{ marginTop: 8 }} />
    </Enter>
  );
};

/** M: closing diagram (Q, one opaque card), f2638 to the last frame. */
const SUBS = ["Claude Max", "Claude Max", "OpenAI", "Grok"];
const HUB: Box = [1080, 520, 1380, 610];
const bez = (t: number, p0: number[], p1: number[], p2: number[], p3: number[]) =>
  [0, 1].map((k) => (1 - t) ** 3 * p0[k] + 3 * (1 - t) ** 2 * t * p1[k] + 3 * (1 - t) * t * t * p2[k] + t ** 3 * p3[k]);

const Closing: React.FC = () => {
  const f = useCurrentFrame();
  const hubT = f - (T.closeIn + 12);
  const breathe = hubT > 0 ? 0.5 + 0.3 * Math.sin((hubT / 60) * 2 * Math.PI) : 0;
  const flowFlash = prog(f, T.way2 + 16, 4) * (1 - prog(f, T.way2 + 20, 12, EASE_IN_OUT));
  const hubIn = prog(f, T.closeIn, 12);
  // the hub starts centred on the sheet and moves to its spot (12 f) before the crop chip enters at "way"
  const hm = prog(f, T.way - 12, 12, EASE_IN_OUT);
  const HUB0: Box = [794, 617, 1094, 707];
  const hub = HUB0.map((v, k) => mix(hm, v, HUB[k]));
  const end: [number, number] = [HUB[0] - 4, (HUB[1] + HUB[3]) / 2];
  const cropS = 0.5;
  const cropRegion: Box = [1050, 945, 1805, 1025];
  return (
    <>
      {/* connectors (behind the chips and the hub) */}
      <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }} width={1} height={1}>
        {SUBS.map((_, i) => {
          const y = 490 + i * 90 + 32;
          const p0 = [768, y];
          const p1 = [930, y];
          const p2 = [940, end[1]];
          const p3 = [end[0], end[1]];
          const a = T.multiple + i * 3 + 6;
          const d = prog(f, a, 14, EASE_IN_OUT);
          if (d <= 0) return null;
          const path = `M ${p0[0]} ${p0[1]} C ${p1[0]} ${p1[1]}, ${p2[0]} ${p2[1]}, ${p3[0]} ${p3[1]}`;
          const L = 420;
          const ft = prog(f, T.way2 + i * 2, 16, EASE_IN_OUT);
          const fo = ft > 0 && ft < 1 ? 1 : 0;
          const [dx, dy] = bez(ft, p0, p1, p2, p3);
          return (
            <g key={i}>
              <path d={path} fill="none" stroke={C.muted} strokeWidth={3} strokeLinecap="round" strokeDasharray={L} strokeDashoffset={L * (1 - d)} />
              {fo ? <circle cx={dx} cy={dy} r={7} fill={C.accent} /> : null}
            </g>
          );
        })}
      </svg>
      {/* hub */}
      {hubIn > 0 ? (
        <div
          style={{
            position: "absolute",
            left: hub[0],
            top: hub[1],
            width: hub[2] - hub[0],
            height: hub[3] - hub[1],
            boxSizing: "border-box",
            borderRadius: 14,
            background: C.dark,
            border: `2px solid ${C.accent}`,
            boxShadow: `${LIFT}, 0 0 ${20 + 14 * (breathe + flowFlash)}px ${3 + 4 * (breathe + flowFlash)}px ${withAlpha(C.accent, 0.22 + 0.25 * (breathe + flowFlash))}`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontFamily: FONT,
            fontSize: 40,
            fontWeight: 800,
            letterSpacing: "-0.02em",
            color: C.ink,
            opacity: hubIn,
            transform: `translateY(${(1 - hubIn) * 10}px) scale(${1 + 0.02 * flowFlash})`,
            filter: hubIn < 1 ? `blur(${(1 - hubIn) * 6}px)` : undefined,
          }}
        >
          T3 Code
        </div>
      ) : null}
      {/* real crop under the hub + pill */}
      <Enter at={T.way} x={1041} y={624} dur={10}>
        <div style={{ borderRadius: 8, overflow: "hidden", border: `1.5px solid ${C.line}` }}>
          <ImgRegion src={HERO.src} srcW={HERO.w} scale={cropS} region={cropRegion} />
        </div>
        <SrcPill domain="t3.codes" style={{ marginTop: 6, height: 28, boxShadow: "none" }} />
      </Enter>
      {/* subscription chips */}
      {SUBS.map((t, i) => (
        <NodeCard key={i} box={[440, 490 + i * 90, 760, 554 + i * 90]} text={t} at={T.multiple + i * 3} lit={prog(f, T.way2 + i * 2, 4) * (1 - prog(f, T.way2 + i * 2 + 14, 10, EASE_IN_OUT))} />
      ))}
      {/* tags (his words) */}
      <DemoChip x={1040} y={714} steps={ALWAYS_CHIP} />
      <DemoChip x={1040} y={772} steps={CTX2_CHIP} />
    </>
  );
};

/* ---------------------------------------------------------------------------------------------- chips */

const LIVE_CHIP: ChipStep[] = [
  { at: 0, lead: "LIVE DEMO", body: "" },
  { at: T.liveGrow, lead: "LIVE DEMO", body: "T3 Code", grow: true },
];
const CTX_CHIP: ChipStep[] = [{ at: T.ctx, body: "No context switching" }];
const CLICK_CHIP: ChipStep[] = [{ at: T.click, lead: "Right-click", body: "Hand off to another model" }];
const ALWAYS_CHIP: ChipStep[] = [{ at: T.always, body: "Always being used" }];
const CTX2_CHIP: ChipStep[] = [{ at: T.context2, body: "No context switching" }];

/* ---------------------------------------------------------------------------------------------- scene */

const Scene: React.FC = () => {
  return (
    <VelourCanvas transparent>
      {/* Everything is clipped to the screen panel (x 19.2-1492.95, y 19.2-1060.8, radius 19.2): soft shadows can never
          reach the camera tile or the badge (x >= 1500), and the frame stays fully transparent outside the panel. */}
      <div style={{ position: "absolute", left: PANEL.x, top: PANEL.y, width: PANEL.w, height: PANEL.h, borderRadius: PANEL.r, overflow: "hidden" }}>
        <div style={{ position: "absolute", left: -PANEL.x, top: -PANEL.y, width: 1920, height: 1080 }}>
      {/* A */}
      <DemoChip x={440} y={44} steps={LIVE_CHIP} out={T.liveOut} outDur={10} />
      <IntroCard />
      <RingBox
        keys={[
          { at: T.chipRing, box: [416, 564, 653, 617] },
          { at: T.railMorph, box: [418, 216, 481, 324] },
        ]}
        morph={6}
        out={T.railRingOut}
        outDur={5}
      />
      {/* B */}
      <Loupe />
      <DocsCard />
      {/* C */}
      <ProvidersCard />
      <DemoChip x={930} y={44} steps={CTX_CHIP} out={T.ctxOut} />
      <RingBox keys={[{ at: T.open, box: [424, 167, 474, 214] }]} out={T.openRingOut} stroke={3} />
      <RingBox
        keys={[
          { at: T.grok, box: [418, 327, 481, 378] },
          { at: T.all, box: [418, 163, 481, 378] },
        ]}
        out={T.cOut}
        outDur={8}
        stroke={3}
      />
      {/* D */}
      <RingBox keys={[{ at: T.toast, box: [540, 37, 1308, 111] }]} out={T.toastOut} outDur={5} stroke={3} radius={12} />
      <RingBox keys={[{ at: T.hit, box: [447, 387, 700, 417] }]} out={T.errOut} outDur={8} stroke={2.5} radius={7} />
      <HelpStrip />
      {/* E */}
      <RingBox
        keys={[
          { at: T.menu, box: [223, 700, 433, 1014] },
          { at: T.hand, box: [228, 705, 429, 737] },
        ]}
        out={T.menuOut}
        outDur={7}
        stroke={3}
        radius={9}
      />
      <DemoChip x={470} y={802} steps={CLICK_CHIP} out={T.clickOut} />
      {/* F */}
      <SwitchCard />
      <RingBox keys={[{ at: T.model, box: [436, 394, 1060, 547] }]} out={T.codexOut} outDur={8} stroke={3} />
      <RingBox
        keys={[
          { at: T.which, box: [442, 608, 1074, 773] },
          { at: T.want, box: [880, 801, 1068, 851] },
        ]}
        out={T.fOut}
        outDur={8}
        stroke={3}
      />
      {/* G */}
      {/* the chip ring fades out (gone f1218), then the timer ring draws on at 'keep' (f1219): no travel across the
          agent's 'Thinking' line */}
      <RingBox keys={[{ at: T.newT, box: [427, 985, 567, 1029] }]} out={T.keep - 6} outDur={5} stroke={3} />
      <RingBox keys={[{ at: T.keep, box: [408, 429, 599, 463] }]} out={T.timerOut} outDur={5} stroke={3} draw={8} />
      {/* H */}
      <Recap />
      <StripKicker text="So handy" at={T.handy} out={T.rOut - 2} />
      {/* I */}
      <Sheet at={T.readmeIn} out={T.readmeOut} keys={[{ at: T.readmeIn, rect: SHEET_I }]} />
      <ReadmeCard />
      {/* J..M: one persistent sheet (J line card, then grows for K, L and M) */}
      <Sheet
        at={T.lineIn}
        keys={[
          { at: T.lineIn, rect: SHEET_J },
          { at: T.repoOut + 8, rect: SHEET_Q },
        ]}
      />
      <RepoCard />
      <ReadmeLine />
      {/* K */}
      <SamRabbitCard />
      <Checks />
      {/* L */}
      <StripKicker text="Highly recommend" at={T.highly} out={T.highlyOut} />
      <FreeCard />
      {/* M */}
      <Closing />
      <StripKicker text="For power users" at={T.power} />
        </div>
      </div>
    </VelourCanvas>
  );
};

export default Scene;
