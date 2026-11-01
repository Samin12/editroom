import React from "react";
import { Img, staticFile, useCurrentFrame } from "remotion";
import { VelourCanvas, C, prog, mix, velour, geoZoom, EASE_OUT, EASE_IN_OUT } from "../../../styles/velour/src";

// anim-mvagv60a: I1 "The hook" (scene ffbb, BEHIND, project 0 to 54.370 s, 1632 frames, JOB_START 0).
// Stage 1920x1080 scaled x0.768 into the left panel by VelourCanvas (camera tile and step badge sit outside the render).
// Word times (project ms): honcho exports/_work/writer/words/ffbb.json. at(ms) = round(ms * 30 / 1000) - 3.
// Every change is sequenced: the old element leaves (or moves aside) before the new one enters; no two texts share a
// spot at any frame. Proof PNGs are built by prep-proof.py (real crops; two are re-wrapped at measured word gaps).
//
// BEATS (frame, word, what shows, proof)
//   f0..27   ("99%")              Canvas only (drifting dot grid). The spoken 99% is never printed, nor any quantifier.
//   f28      "AI" (f31)           Headline "AI second brain setups" builds word by word.                       concept
//   f76      "one"                Second line "one big problem"; f86 "big": accent highlighter on "big problem".
//   f103     ("problem." ends)    Both lines leave (gone f110).
//   f110     "They"               Four identical note cards (grey placeholder bars only) cascade in, 3 f apart.  concept
//   f139     "same"               Highlighter across the first bar of all four cards (the repeated line).
//   f168     ("but")              Stack dims to 35% and slides left; f172 "but": the brain outline (12 nodes, thin
//                                  lines, a few accent "active" nodes breathing) fades in on the right.
//   f178     "real"               Label "YOUR REAL BRAIN" under the brain; leaves f234.
//   f267     "process"            Chip "process" under the brain; f330 "every" (month) chip "every month" joins it in
//                                  the same row (two chips, centred under the brain). Both leave f350. (v2: the
//                                  "every way" chip is gone: "every way" is a slip for "every day", not shown.)
//   f360     "change"             Brain grows 1.0 to 1.2 and gains 3 accent nodes with new links.
//   f375     "taste"              Chip "taste"; f413 "better": chip "better" with an up arrow, same row.
//   f426     "So"                 The stack comes back to 60%, greyscale, frozen. Brain holds right.
//   f479     "only"               The stack drifts up 36 px (its only motion). f500: taste/better chips leave.
//   f522     "remember"           Chip "remember" above the stack.
//   f561     "stuck"              Outline lock lands on the stack.
//   f593     "old"                Label "OLD VERSION" under the stack; f629 "your": "YOUR CURRENT BRAIN" under the brain.
//   f652     ("brain." ends)      Everything leaves (gone f658).
//   f658     "And"                Three generic file cards (tabs 01.md, 02.md, 03.md, grey bars) rise in.        concept
//   f712     ("start")            Files leave; f720 a blank dashed card with a blinking caret + label "ALMOST FROM
//                                  SCRATCH" (f722, lands on "almost" f724).
//   f770     ("or go")            Blank card leaves; f776 the files return with label "FILE BY FILE"; a caret edits
//                                  01.md at f778 "file", 02.md at f793 "file", 03.md at f803 "and" (bar flashes accent).
//   f828     ("it." ends)         Files leave (gone f834).
//   f834     ("But")              The brain fades in on the right, slides to the centre on "with" (f846-866) and fills:
//                                  nodes take the accent (f849). NO TEXT here (no "free", no "local", no "setup").
//   f881     "setup"              Hub links draw one by one: f881, 893, 905, 917, 928 ("you"). Signals pulse on them.
//   f977     "with" (this setup)  Hub slides left and shrinks (f977-990).
//   f1034    "whatever" (f1037)   Headline "“Whatever tool I want”" (his words, in quotes) right of the hub.
//   f1064    ("want," ends)       Headline leaves (f1068-1074); hub tucks into the top-left corner (f1064-1078).
//   f1080    ("whether it's")     Browser card honcho.dev: the real "ONE MEMORY FOR ALL YOUR AGENTS" strip (Agent Skill,
//                                  Claude, Codex, Hermes Agent) at 1.3x; row under it: source pill + honest note (f1083,
//                                  "ChatGPT" f1088) "ChatGPT isn't on Honcho's list. / Its desktop app connects through
//                                  MCP." (v2: names ChatGPT, shown only while he says it). The note leaves f1101-1107,
//                                  gone before the Claude ring starts; its slot stays reserved so the pill never moves.
//                                  No ChatGPT logo or chip; ChatGPT appears only as the subject of that honest line.
//                                  proof 01-memory-today/04-honcho-tools-crop.png -> tools-strip.png; the ChatGPT line:
//                                  proof README #4 honest line + 01-memory-today/08-chatgpt-desktop-mcp (not shown)
//   f1107    "Claude"             Ring on the Claude tile + arrow from the hub to it. No negative caption on screen.
//   (f1136-1176 "the best AI tool in the market": nothing new is printed.)
//   f1187    "Hermes"             Ring on the Hermes Agent tile + arrow from the hub to it.
//   f1222    ("and")              Tiles card, pill and note leave (gone f1228).
//   f1230    "and it will"        Browser card honcho.dev/docs/v3/guides/overview: the OpenClaw card (1.5x). Row: pill +
//                                  honest note "Plugins capture automatically. / Other tools call Honcho when they choose to."
//                                  f1247 "automatically": highlighter on "automatic message capture."
//                                  proof 01-memory-today/05-honcho-docs-cards-crop.png -> openclaw-card.png
//   f1294    ("context")          Card + row leave; f1300 card (same page): "All integrations share one Honcho workspace,
//                                  so memory formed in / one carries over to the rest." (1.25x, re-wrapped at a word gap;
//                                  the em-dash line above it is not used). f1315 "update": highlighter on "carries over".
//                                  proof 05-honcho-concepts/02-integrations-docs-crop.png -> integrations-sentence.png
//   f1338    ("whenever")         Card leaves; f1344 card honcho.dev/docs/v3/documentation/features/advanced/dreaming:
//                                  the yellow warning "Dreaming is an experimental feature under active development."
//                                  above the "Knowledge updates" bullet (page order, re-wrapped into 4 rows, 1.15x, light
//                                  captures dimmed 10%). Row: pill + honest note "New facts come from every message. /
//                                  Dreaming, still experimental, replaces outdated ones." f1359 "changes": highlighter on
//                                  "the same fact has changed over time"; f1380 "But": highlighter on "experimental feature".
//                                  proof 90-gaps/15-...-experimental-warning-full.png -> dreaming-warning.png,
//                                  90-gaps/14-...-delete-outdated-crop.png -> dreaming-bullet.png
//   f1425    ("good job")         Dreaming card leaves; the hub returns to the centre (f1428-1446).
//   f1450    "set" (f1453)        Headline "Set it up correctly" under the hub; leaves f1480.
//   f1519    "showing"            Hub link 6 draws; f1543 "build": link 7. Hub fades f1550-1557.
//   f1556    "AI" (f1558)         Closing line 1 "An AI second brain".
//   f1598    "evolving" (f1600)   Line 2 "that evolves with you" (accent on "evolves with you"); 2% push to the end.
// Deviations from the section brief (reasons in render.json notes): the kicker "But it will only do a good job" is dropped
// so the dreaming proof holds 2.9 s instead of 0.7 s; the hub tucks into the corner during the proofs (a 1.3x tile strip
// cannot sit beside it); the stack slides 380 px (not 60) so the brain has room on the right.
// Do not show (respected): 99% or any number/ranking; "free", "local", "hosted", "self-hosted"; "best"; ChatGPT logo/
// chip/label; "10x"; star counts; the docs "25 available"; the LongMem banner; "Vibecoding setup"; real notes or names.

const JOB_START = 0;
/** Render frame of a project time (ms), led by `lead` frames. */
const at = (ms: number, lead = 3) => Math.round(((ms - JOB_START) * 30) / 1000) - lead;

const T = {
  hIn: at(1141) - 3, // 28 ("AI" 31)
  subIn: at(2682) - 1, // 76 "one"
  subMark: at(2963), // 86 "big"
  hOut: 103, // "problem." ends f113; gone by 110
  stackIn: at(3763), // 110 "They"
  sameMark: at(4744), // 139 "same"
  split: at(5825) - 4, // 168: stack dims + slides left
  brainIn: at(5825), // 172 "but"
  realLabel: at(6146) - 3, // 178 ("real" 181)
  realLabelOut: 234,
  chip1: at(8988), // 267 "process"
  chip3: at(11110), // 330 "every" month (v2: no chip on the "every way" slip)
  chipsOut: 350,
  grow: at(12091), // 360 "change"
  taste: at(12592), // 375 "taste"
  better: at(13873), // 413 "better"
  stackBack: at(14313), // 426 "So"
  drift: at(16055), // 479 "only"
  tbOut: 500,
  remember: at(17516), // 522 "remember"
  lock: at(18797), // 561 "stuck"
  oldLabel: at(19858), // 593 "old"
  curLabel: at(21079), // 629 "your" current brain
  allOut: 652, // "brain." ends f661
  filesIn: at(22040), // 658 "And"
  filesOut: 712,
  blankIn: 720, // "start" f710 .. "almost" f727
  blankLabel: 722,
  blankOut: 764,
  files2In: 770,
  caret: [at(26024), at(26544), at(26865)], // 778 "file", 793 "file", 803 "and"
  files2Out: 828, // "it." ends f834
  hubIn: 834,
  hubCenter: at(28286), // 846 "with"
  fill: at(28286) + 3, // 849
  links: [at(29467), 893, 905, 917, at(31049)], // 881 "setup" .. 928 "you"
  side: at(32670), // 977 "with" this setup
  wtHead: at(34672) - 3, // 1034 ("whatever" 1037)
  wtOut: 1068,
  corner: 1064,
  tilesIn: 1080,
  tilesNote: at(36253) - 2, // 1083 ("ChatGPT" 1088)
  tilesNoteOut: at(36994) - 6, // 1101: gone (6 f) at 1107, the frame the Claude ring starts
  ringClaude: at(36994), // 1107 "Claude"
  ringHermes: at(39676), // 1187 "Hermes"
  tilesOut: 1222, // "and" 1225
  a1In: 1230,
  a1Mark: at(41678), // 1247 "automatically"
  a1Out: 1294,
  a2In: 1300,
  a2Mark: at(43920), // 1315 "update"
  a2Out: 1338,
  bIn: 1344, // "whenever" 1342
  bMark: at(45322) + 2, // 1359 ("changes" 1357, after the card settles)
  bRing: at(46102), // 1380 "But"
  bOut: 1425,
  hubBack: 1428,
  setHead: at(48545) - 3, // 1450 ("set" 1453)
  setOut: 1480,
  link6: at(50727), // 1519 "showing"
  link7: at(51547), // 1543 "build"
  hubOut: 1550,
  end1: at(52028) - 2, // 1556 ("AI" 1558)
  end2: at(53449) - 2, // 1598 ("evolving" 1600)
};

const FONT = velour.font.display;
const P = "honcho/proof/anim-mvagv60a/";
const BAR = 72;

const hexA = (hex: string, a: number) => {
  const n = parseInt(hex.replace("#", ""), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
};
const accentA = (a: number) => hexA(C.accent, a);
const CARD_SHADOW = `${velour.shadow.card}, 0 0 0 1.5px rgba(255,255,255,0.10)`;
const blurOf = (v: number) => (v > 0.02 ? `blur(${v.toFixed(2)}px)` : undefined);
const outOf = (f: number, start: number, dur = 6) => 1 - prog(f, start, dur, EASE_IN_OUT);

type Rect = { x: number; y: number; w: number; h: number };

/* ------------------------------------------------------------------ glyphs */

const UrlLock: React.FC<{ size: number; color: string }> = ({ size, color }) => (
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

const UpArrow: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" style={{ flex: "none" }}>
    <path d="M12 20V5M5.5 11.5 12 5l6.5 6.5" stroke={color} strokeWidth={3} strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/* ------------------------------------------------------------------ type */

/** Headline that builds word by word (11 f per word). "\n" breaks a line; `accent` words take the accent colour. */
const Words: React.FC<{
  text: string;
  at: number;
  size: number;
  accent?: string[];
  stagger?: number;
  weight?: number;
  color?: string;
  align?: "left" | "center";
  out?: number;
  outDur?: number;
  style?: React.CSSProperties;
}> = ({ text, at: start, size, accent = [], stagger = 3, weight = 800, color = C.ink, align = "left", out, outDur = 6, style }) => {
  const f = useCurrentFrame();
  const clean = (w: string) => w.toLowerCase().replace(/[^a-z0-9]/g, "");
  const acc = new Set(accent.map(clean));
  const o = out != null ? outOf(f, out, outDur) : 1;
  if (f < start - 1 || o <= 0) return null;
  let k = 0;
  return (
    <div
      style={{
        position: "absolute",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        lineHeight: 1.06,
        letterSpacing: "-0.035em",
        color,
        textAlign: align,
        whiteSpace: "nowrap",
        opacity: o,
        filter: blurOf((1 - o) * 6),
        ...style,
      }}
    >
      {text.split("\n").map((line, li) => (
        <div key={li}>
          {line.split(" ").map((w, wi, arr) => {
            const p = prog(f, start + k++ * stagger, 11);
            return (
              <span
                key={wi}
                style={{
                  display: "inline-block",
                  whiteSpace: "pre",
                  opacity: p,
                  transform: `translateY(${((1 - p) * 0.4).toFixed(3)}em)`,
                  filter: blurOf((1 - p) * 8),
                  color: acc.has(clean(w)) ? C.accent : undefined,
                }}
              >
                {w}
                {wi < arr.length - 1 ? " " : ""}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
};

/** Uppercase label with a leading accent rule (kicker style), centred on cx. */
const Tag: React.FC<{ text: string; at: number; cx: number; top: number; out?: number; color?: string; size?: number }> = ({
  text,
  at: start,
  cx,
  top,
  out,
  color = C.accent,
  size = 32,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 10, EASE_OUT);
  const o = out != null ? outOf(f, out) : 1;
  if (p <= 0 || o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: cx,
        top,
        transform: `translate(-50%, ${((1 - p) * 12).toFixed(2)}px)`,
        opacity: p * o,
        filter: blurOf((1 - p) * 6 + (1 - o) * 5),
        display: "flex",
        alignItems: "center",
        gap: size * 0.6,
        fontFamily: FONT,
        whiteSpace: "nowrap",
      }}
    >
      <div style={{ width: size * 1.4, height: Math.max(4, size * 0.13), borderRadius: 4, background: color }} />
      <div style={{ fontSize: size, fontWeight: 750, letterSpacing: "0.16em", textTransform: "uppercase", color }}>{text}</div>
    </div>
  );
};

/** Dark pill chip (C.dark / C.onDark), pops in (11 f). Inline: lay several out in a flex row. */
const Chip: React.FC<{ text: string; at: number; out?: number; icon?: React.ReactNode; size?: number }> = ({ text, at: start, out, icon, size = 38 }) => {
  const f = useCurrentFrame();
  const t = prog(f, start, 11, (x) => x);
  const s = 1.70158;
  const back = t === 0 ? 0 : 1 + (s + 1) * Math.pow(t - 1, 3) + s * Math.pow(t - 1, 2);
  const o = out != null ? outOf(f, out) : 1;
  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 14,
        height: size * 2,
        padding: `0 ${size * 0.85}px`,
        boxSizing: "border-box",
        borderRadius: 999,
        background: C.dark,
        color: C.onDark,
        boxShadow: `${velour.shadow.soft}, inset 0 0 0 1.5px ${C.darkLine}`,
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 700,
        letterSpacing: "-0.01em",
        whiteSpace: "nowrap",
        opacity: Math.min(1, t * 2) * o,
        transform: `translateY(${((1 - t) * 18).toFixed(2)}px) scale(${mix(back, 0.82, 1).toFixed(4)})`,
        filter: blurOf((1 - o) * 5),
      }}
    >
      {text}
      {icon}
    </div>
  );
};

/* -------------------------------------------------------------- proof bits (copied from anim-mva2k5iy) */

/** Source pill (inline; place inside a Row). */
const SourcePill: React.FC<{ text: string; tail: string; at: number }> = ({ text, tail, at: start }) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 10, EASE_OUT);
  return (
    <div
      style={{
        height: 62,
        flex: "none",
        transform: `translateY(${((1 - p) * 10).toFixed(2)}px)`,
        opacity: p,
        display: "flex",
        alignItems: "center",
        gap: 14,
        padding: "0 28px",
        borderRadius: 999,
        background: C.paper,
        border: `1.5px solid ${C.darkLine}`,
        boxSizing: "border-box",
        fontFamily: FONT,
        fontSize: 33,
        fontWeight: 600,
        whiteSpace: "nowrap",
        color: C.muted,
      }}
    >
      <LinkGlyph size={30} color={C.accent} />
      <span style={{ color: C.inkSoft }}>{text}</span>
      <span>·</span>
      <span>{tail}</span>
    </div>
  );
};

/** Honest note: dashed outline, muted text (inline; place inside a Row). With `out` it fades (6 f) but keeps its slot. */
const HonestNote: React.FC<{ lines: string[]; at: number; out?: number; dur?: number }> = ({ lines, at: start, out, dur = 10 }) => {
  const f = useCurrentFrame();
  const p = prog(f, start, dur, EASE_OUT) * (out != null ? outOf(f, out) : 1);
  return (
    <div
      style={{
        flex: "none",
        padding: "12px 28px",
        borderRadius: 18,
        border: `2.5px dashed ${C.faint}`,
        boxSizing: "border-box",
        fontFamily: FONT,
        fontSize: 36,
        fontWeight: 550,
        lineHeight: 1.3,
        color: C.muted,
        whiteSpace: "nowrap",
        opacity: p,
        transform: `translateY(${((1 - p) * 10).toFixed(2)}px)`,
        filter: blurOf((1 - p) * 5),
      }}
    >
      {lines.map((l, i) => (
        <div key={i}>{l}</div>
      ))}
    </div>
  );
};

const Row: React.FC<{ cx: number; top: number; children?: React.ReactNode }> = ({ cx, top, children }) => (
  <div style={{ position: "absolute", left: cx, top, transform: "translateX(-50%)", display: "flex", alignItems: "flex-start", gap: 24 }}>{children}</div>
);

/** Accent ring drawn on, in SOURCE px inside a scaled screenshot; `s` keeps the stroke in stage px. */
const Ring: React.FC<{ r: Rect; at: number; s: number; radius: number; stroke?: number }> = ({ r, at: start, s, radius, stroke = 6 }) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 10, EASE_IN_OUT);
  if (p <= 0) return null;
  const sw = stroke / s;
  const glow = 0.45 + 0.55 * prog(f, start + 7, 10);
  return (
    <svg
      width={r.w + sw * 2}
      height={r.h + sw * 2}
      style={{ position: "absolute", left: r.x - sw, top: r.y - sw, overflow: "visible", filter: `drop-shadow(0 0 ${(14 * glow) / s}px ${accentA(0.35 + 0.35 * glow)})` }}
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

/** Highlighter band sweeping over a phrase (multiply on light pages), with an accent underline. SOURCE px. */
const Highlighter: React.FC<{ r: Rect; at: number; s: number; blend?: "multiply" | "screen" | "normal"; thick?: number }> = ({
  r,
  at: start,
  s,
  blend = "multiply",
  thick = 5,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 11, EASE_IN_OUT);
  if (p <= 0) return null;
  const glow = prog(f, start + 6, 10);
  const th = thick / s;
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: r.x,
          top: r.y,
          width: r.w * p,
          height: r.h,
          borderRadius: 6 / s,
          background: accentA(blend === "multiply" ? 0.34 : 0.3),
          mixBlendMode: blend === "normal" ? undefined : blend,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: r.x,
          top: r.y + r.h + 1 / s,
          width: r.w * p,
          height: th,
          borderRadius: th,
          background: C.accent,
          boxShadow: `0 0 ${(10 * glow) / s}px ${accentA(0.5 * glow)}`,
        }}
      />
    </>
  );
};

/** Browser chrome bar: traffic lights + URL pill with lock, domain and muted path. */
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
      <UrlLock size={24} color={C.muted} />
      <span style={{ flex: "none" }}>{domain}</span>
      <span style={{ color: C.muted, fontWeight: 500, marginLeft: -12, overflow: "hidden", textOverflow: "ellipsis", minWidth: 0 }}>{path}</span>
    </div>
  </div>
);

/** A screenshot at scale `s` (children use SOURCE px), with a geometric push `z` around source point `o`. */
const Shot: React.FC<{ src: string; iw: number; ih: number; s: number; z?: number; o?: [number, number]; filter?: string; children?: React.ReactNode }> = ({
  src,
  iw,
  ih,
  s,
  z = 1,
  o = [iw / 2, ih / 2],
  filter,
  children,
}) => (
  <div style={{ position: "absolute", left: 0, top: 0, width: iw * s, height: ih * s, transform: `scale(${z.toFixed(5)})`, transformOrigin: `${o[0] * s}px ${o[1] * s}px` }}>
    <div style={{ position: "absolute", left: 0, top: 0, width: iw, height: ih, transform: `scale(${s})`, transformOrigin: "0 0" }}>
      <Img src={staticFile(src)} style={{ position: "absolute", left: 0, top: 0, width: iw, height: ih, maxWidth: "none", filter }} />
      {children}
    </div>
  </div>
);

/** Browser card frame with entrance (10 f). Content area below the bar is `children`. */
const BrowserCard: React.FC<{ x: number; y: number; w: number; h: number; at: number; domain: string; path: string; page?: string; children?: React.ReactNode }> = ({
  x,
  y,
  w,
  h,
  at: start,
  domain,
  path,
  page = C.white,
  children,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, 10, EASE_OUT);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: h,
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
      <div style={{ position: "absolute", left: 0, top: BAR, width: w, height: h - BAR, overflow: "hidden", background: page }}>{children}</div>
    </div>
  );
};

/** Accent arrow drawn on along a cubic curve, head from the end tangent. Stage px. */
const CurveArrow: React.FC<{ p0: [number, number]; c1: [number, number]; c2: [number, number]; p1: [number, number]; at: number; dur?: number; width?: number }> = ({
  p0,
  c1,
  c2,
  p1,
  at: start,
  dur = 12,
  width = 5,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, start, dur, EASE_IN_OUT);
  if (p <= 0) return null;
  const ang = Math.atan2(p1[1] - c2[1], p1[0] - c2[0]);
  const hs = width * 3.6;
  const head = `M ${p1[0] - hs * Math.cos(ang - 0.5)} ${p1[1] - hs * Math.sin(ang - 0.5)} L ${p1[0]} ${p1[1]} L ${p1[0] - hs * Math.cos(ang + 0.5)} ${p1[1] - hs * Math.sin(ang + 0.5)}`;
  return (
    <svg style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, overflow: "visible", filter: `drop-shadow(0 0 8px ${accentA(0.35)})` }} viewBox="0 0 1920 1080">
      <path
        d={`M ${p0[0]} ${p0[1]} C ${c1[0]} ${c1[1]} ${c2[0]} ${c2[1]} ${p1[0]} ${p1[1]}`}
        fill="none"
        stroke={C.accent}
        strokeWidth={width}
        strokeLinecap="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1 - p}
      />
      <path d={head} fill="none" stroke={C.accent} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" opacity={prog(f, start + dur - 3, 4)} />
    </svg>
  );
};

/* ------------------------------------------------------------------ the brain / memory hub */

// Brain glyph in local px around its centre (about 364 x 306): 12 outline nodes joined by a smooth outline, a stem
// node, a few sulci, and 3 extra nodes that appear on "change".
const OUTLINE: [number, number][] = [
  [-142, 44],
  [-176, -28],
  [-146, -100],
  [-76, -146],
  [14, -158],
  [100, -136],
  [162, -80],
  [184, -4],
  [152, 58],
  [82, 84],
  [-8, 80],
  [-84, 74],
];
const STEM_NODE: [number, number] = [52, 150];
const SULCI = [
  "M -128 30 Q -40 -8 72 12",
  "M 16 -154 Q -6 -92 28 -34",
  "M -106 -118 Q -84 -68 -118 -14",
  "M 84 82 Q 132 46 178 18",
  "M 112 -124 Q 92 -82 124 -40",
  "M 30 80 Q 44 112 52 150",
];
const EXTRA: [number, number][] = [
  [-62, -60],
  [62, -76],
  [44, 28],
];
// extra-node links: [extra index, outline index or -1-extra index]
const EXTRA_EDGES: [number, number][] = [
  [0, 2],
  [0, 11],
  [0, -2],
  [1, 4],
  [1, 6],
  [1, -3],
  [2, 9],
  [2, 7],
  [2, -1],
];
const ACTIVE = [2, 5, 8, 11]; // nodes that breathe in the accent while the brain is "alive"

const closedSpline = (pts: [number, number][]) => {
  const n = pts.length;
  let d = `M ${pts[0][0]} ${pts[0][1]}`;
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n];
    const p1 = pts[i];
    const p2 = pts[(i + 1) % n];
    const p3 = pts[(i + 2) % n];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += ` C ${c1[0].toFixed(1)} ${c1[1].toFixed(1)} ${c2[0].toFixed(1)} ${c2[1].toFixed(1)} ${p2[0]} ${p2[1]}`;
  }
  return `${d} Z`;
};
const OUTLINE_D = closedSpline(OUTLINE);

// Hub links: spoke angles (deg, y down) and the frame each one draws. Satellites carry no labels.
const SPOKES: { ang: number; at: number }[] = [
  { ang: 206, at: T.links[0] },
  { ang: 0, at: T.links[1] },
  { ang: 154, at: T.links[2] },
  { ang: 309, at: T.links[3] },
  { ang: 103, at: T.links[4] },
  { ang: 257, at: T.link6 },
  { ang: 51, at: T.link7 },
];
const SAT_R = [390, 312];
const EDGE_R = [196, 160];
const spokeEnds = (ang: number) => {
  const a = (ang * Math.PI) / 180;
  const c = Math.cos(a);
  const s = Math.sin(a);
  return { a: [EDGE_R[0] * c, EDGE_R[1] * s] as [number, number], b: [SAT_R[0] * c, SAT_R[1] * s] as [number, number] };
};

type HubPose = { x: number; y: number; s: number };
const P_RIGHT: HubPose = { x: 1390, y: 470, s: 1.3 };
const P_CENTER: HubPose = { x: 960, y: 480, s: 1.15 };
const P_SIDE: HubPose = { x: 560, y: 520, s: 0.66 };
const P_CORNER: HubPose = { x: 232, y: 196, s: 0.44 };
const P_FINAL: HubPose = { x: 960, y: 400, s: 0.82 };

const hubPose = (f: number): HubPose => {
  let pose: HubPose = { ...P_RIGHT };
  const moves: [number, number, HubPose][] = [
    [T.grow, 18, { ...P_RIGHT, s: 1.55 }],
    [T.hubCenter, 20, P_CENTER],
    [T.side, 14, P_SIDE],
    [T.corner, 14, P_CORNER],
    [T.hubBack, 18, P_FINAL],
  ];
  for (const [start, dur, to] of moves) {
    const t = prog(f, start, dur, EASE_IN_OUT);
    pose = { x: mix(t, pose.x, to.x), y: mix(t, pose.y, to.y), s: mix(t, pose.s, to.s) };
  }
  return pose;
};

const hubOpacity = (f: number) => {
  const a = prog(f, T.brainIn, 12, EASE_OUT) * outOf(f, T.allOut);
  const b = prog(f, T.hubIn, 12, EASE_OUT) * outOf(f, T.hubOut, 7);
  return f < T.hubIn ? a : b;
};

const BrainHub: React.FC = () => {
  const f = useCurrentFrame();
  const o = hubOpacity(f);
  if (o <= 0.001) return null;
  const { x, y, s } = hubPose(f);
  const k = 1 / Math.sqrt(Math.min(1, s)); // keep strokes readable when the hub is small
  const fill = prog(f, T.fill, 20, EASE_IN_OUT);
  const pulsesOn = f >= T.fill + 20;
  const nodeR = 11;
  const breathe = (i: number) => 0.5 + 0.5 * Math.sin((f + i * 17) / 9);
  const node = (pt: [number, number], key: string, active: number, appear = 1) => {
    const r = nodeR * mix(appear, 0.2, 1) * (1 + 0.12 * active * breathe(pt[0] + pt[1]));
    return (
      <g key={key} opacity={Math.min(1, appear * 1.6)}>
        {active > 0.02 ? <circle cx={pt[0]} cy={pt[1]} r={r * 2.1} fill={accentA(0.16 * active)} /> : null}
        <circle cx={pt[0]} cy={pt[1]} r={r} fill={C.paper} stroke={C.muted} strokeWidth={2.5 * k} />
        <circle cx={pt[0]} cy={pt[1]} r={r} fill={C.accent} opacity={active} />
      </g>
    );
  };
  return (
    <svg
      width={1000}
      height={800}
      viewBox="-500 -400 1000 800"
      style={{
        position: "absolute",
        left: x - 500,
        top: y - 400,
        overflow: "visible",
        opacity: o,
        transform: `scale(${s.toFixed(4)})`,
        transformOrigin: "500px 400px",
      }}
    >
      {/* hub links + satellites */}
      {SPOKES.map((sp, i) => {
        const dp = prog(f, sp.at, 12, EASE_IN_OUT);
        if (dp <= 0) return null;
        const { a, b } = spokeEnds(sp.ang);
        const sat = prog(f, sp.at + 8, 10, EASE_OUT);
        const ex = mix(dp, a[0], b[0]);
        const ey = mix(dp, a[1], b[1]);
        const cyc = ((f - sp.at - 12) % 48) / 48;
        const dot = pulsesOn && f > sp.at + 12 ? cyc : -1;
        return (
          <g key={i}>
            <line x1={a[0]} y1={a[1]} x2={ex} y2={ey} stroke={accentA(0.55)} strokeWidth={3.5 * k} strokeLinecap="round" />
            {dot >= 0 ? (
              <circle cx={mix(dot, a[0], b[0])} cy={mix(dot, a[1], b[1])} r={6 * k} fill={C.accent} opacity={Math.sin(Math.PI * dot) * 0.9} />
            ) : null}
            {sat > 0 ? (
              <g opacity={sat} transform={`translate(${b[0]} ${b[1]}) scale(${mix(sat, 0.4, 1).toFixed(3)})`}>
                <circle r={34} fill={accentA(0.1)} />
                <circle r={20} fill={C.paper} stroke={C.accent} strokeWidth={4 * k} />
                <circle r={7} fill={C.accent} />
              </g>
            ) : null}
          </g>
        );
      })}
      {/* brain body */}
      <path d={OUTLINE_D} fill={accentA(0.1 * fill)} stroke={C.muted} strokeWidth={3 * k} strokeLinejoin="round" />
      <path d={OUTLINE_D} fill="none" stroke={C.accent} strokeWidth={3 * k} strokeLinejoin="round" opacity={fill} />
      {SULCI.map((d, i) => (
        <g key={i}>
          <path d={d} fill="none" stroke={C.muted} strokeWidth={2.4 * k} strokeLinecap="round" opacity={0.85} />
          <path d={d} fill="none" stroke={C.accent} strokeWidth={2.4 * k} strokeLinecap="round" opacity={fill * 0.85} />
        </g>
      ))}
      {/* the 3 nodes it gains on "change", with their links */}
      {EXTRA_EDGES.map(([e, t], i) => {
        const lp = prog(f, T.grow + e * 4 + 2, 10, EASE_IN_OUT);
        if (lp <= 0) return null;
        const a = EXTRA[e];
        const b = t >= 0 ? OUTLINE[t] : EXTRA[-t - 1];
        return (
          <line
            key={i}
            x1={a[0]}
            y1={a[1]}
            x2={mix(lp, a[0], b[0])}
            y2={mix(lp, a[1], b[1])}
            stroke={accentA(0.7)}
            strokeWidth={2.4 * k}
            strokeLinecap="round"
          />
        );
      })}
      {OUTLINE.map((pt, i) => {
        const base = ACTIVE.includes(i) ? 0.55 + 0.45 * breathe(i * 31) : 0;
        const ripple = prog(f, T.fill + Math.round(Math.hypot(pt[0], pt[1]) / 22), 10, EASE_OUT);
        return node(pt, `o${i}`, Math.max(base, ripple));
      })}
      {node(STEM_NODE, "stem", prog(f, T.fill + 9, 10, EASE_OUT))}
      {EXTRA.map((pt, i) => {
        const ap = prog(f, T.grow + i * 4, 10, EASE_OUT);
        return ap > 0 ? node(pt, `x${i}`, 1, ap) : null;
      })}
    </svg>
  );
};

/* ------------------------------------------------------------------ opening headline */

const OpeningBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f > T.hOut + 8) return null;
  const o = outOf(f, T.hOut, 6);
  const sp = prog(f, T.subIn, 11, EASE_OUT);
  const mk = prog(f, T.subMark, 11, EASE_IN_OUT);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <Words text="AI second brain setups" at={T.hIn} size={132} align="center" style={{ left: 0, width: 1920, top: 352 }} />
      {sp > 0 ? (
        <div
          style={{
            position: "absolute",
            left: 0,
            width: 1920,
            top: 532,
            textAlign: "center",
            fontFamily: FONT,
            fontSize: 100,
            fontWeight: 800,
            letterSpacing: "-0.035em",
            color: C.ink,
            whiteSpace: "nowrap",
            opacity: sp,
            transform: `translateY(${((1 - sp) * 24).toFixed(2)}px)`,
            filter: blurOf((1 - sp) * 8),
          }}
        >
          one{" "}
          <span style={{ position: "relative", display: "inline-block" }}>
            <span style={{ position: "absolute", left: -10, right: -10, top: "18%", bottom: "6%", borderRadius: 12, background: accentA(0.28), transformOrigin: "0 50%", transform: `scaleX(${mk.toFixed(4)})` }} />
            <span style={{ position: "absolute", left: -10, right: -10, bottom: "2%", height: 8, borderRadius: 8, background: C.accent, transformOrigin: "0 50%", transform: `scaleX(${mk.toFixed(4)})` }} />
            <span style={{ position: "relative" }}>big problem</span>
          </span>
        </div>
      ) : null}
    </div>
  );
};

/* ------------------------------------------------------------------ the notes stack (frozen second brain) */

const NOTE_W = 640;
const NOTE_H = 340;
const NOTE_DY = 84;
const STACK_H = NOTE_H + NOTE_DY * 3; // 592
const STACK_X0 = 960 - NOTE_W / 2; // 640, centred
const STACK_X1 = 220; // left (centre 540)
const STACK_Y = Math.round((1080 - STACK_H) / 2); // 244
const STACK_CX1 = STACK_X1 + NOTE_W / 2; // 540
const DRIFT = 36;

const NoteCard: React.FC<{ mark: number }> = ({ mark }) => (
  <div style={{ position: "absolute", inset: 0, borderRadius: 24, background: C.paper, border: `2px solid ${C.line}`, boxSizing: "border-box", boxShadow: velour.shadow.soft }}>
    <div style={{ position: "absolute", left: 40, top: 30, width: 330, height: 24, borderRadius: 12, background: C.darkLine }} />
    {mark > 0 ? (
      <>
        <div style={{ position: "absolute", left: 28, top: 20, width: 354 * mark, height: 44, borderRadius: 10, background: accentA(0.32) }} />
        <div style={{ position: "absolute", left: 28, top: 66, width: 354 * mark, height: 5, borderRadius: 5, background: C.accent }} />
      </>
    ) : null}
    {[540, 490, 515, 340].map((w, i) => (
      <div key={i} style={{ position: "absolute", left: 40, top: 116 + i * 48, width: w, height: 17, borderRadius: 9, background: C.line }} />
    ))}
  </div>
);

const StackBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.stackIn - 1 || f > T.allOut + 8) return null;
  const slide = prog(f, T.split, 14, EASE_IN_OUT);
  const back = prog(f, T.stackBack, 12, EASE_IN_OUT);
  const drift = prog(f, T.drift, 36, EASE_IN_OUT) * DRIFT;
  const x = mix(slide, STACK_X0, STACK_X1);
  const op = mix(back, mix(slide, 1, 0.35), 0.6);
  const grey = back;
  const o = outOf(f, T.allOut);
  const markFade = 1 - prog(f, T.split, 10, EASE_IN_OUT);
  const lp = prog(f, T.lock, 10, EASE_OUT);
  const top = STACK_Y - drift;
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <div style={{ position: "absolute", left: x, top, width: NOTE_W, height: STACK_H, opacity: op, filter: grey > 0.01 ? `grayscale(${grey.toFixed(3)})` : undefined }}>
        {[0, 1, 2, 3].map((i) => {
          const p = prog(f, T.stackIn + i * 3, 10, EASE_OUT);
          const mark = prog(f, T.sameMark + i * 2, 11, EASE_IN_OUT) * markFade;
          return (
            <div
              key={i}
              style={{
                position: "absolute",
                left: 0,
                top: i * NOTE_DY,
                width: NOTE_W,
                height: NOTE_H,
                opacity: p,
                transform: `translateY(${((1 - p) * 40).toFixed(2)}px) scale(${mix(p, 0.96, 1).toFixed(4)})`,
                filter: blurOf((1 - p) * 8),
              }}
            >
              <NoteCard mark={mark} />
            </div>
          );
        })}
      </div>
      {/* "remember" above the stack */}
      <div style={{ position: "absolute", left: STACK_CX1, top: top - 128, transform: "translateX(-50%)" }}>
        <Chip text="remember" at={T.remember} />
      </div>
      {/* the lock lands on "stuck" */}
      {lp > 0 ? (
        <div
          style={{
            position: "absolute",
            left: STACK_CX1 - 125,
            top: top + STACK_H / 2 - 125 + 24,
            width: 250,
            height: 250,
            opacity: Math.min(1, lp * 1.5),
            transform: `scale(${mix(lp, 1.45, 1).toFixed(4)})`,
          }}
        >
          <svg width={250} height={250} viewBox="-110 -110 220 220">
            <circle r={104} fill={hexA(C.bg, 0.86)} stroke={accentA(0.35)} strokeWidth={3} />
            <path d="M -36 -10 V -36 A 36 36 0 0 1 36 -36 V -10" fill="none" stroke={C.accent} strokeWidth={10} strokeLinecap="round" />
            <rect x={-56} y={-12} width={112} height={86} rx={18} fill="none" stroke={C.accent} strokeWidth={10} />
            <circle cx={0} cy={24} r={9} fill={C.accent} />
            <path d="M 0 28 V 46" stroke={C.accent} strokeWidth={8} strokeLinecap="round" />
          </svg>
        </div>
      ) : null}
      <Tag text="old version" at={T.oldLabel} cx={STACK_CX1} top={STACK_Y - DRIFT + STACK_H + 34} color={C.muted} />
      <Tag text="your current brain" at={T.curLabel} cx={P_RIGHT.x} top={STACK_Y - DRIFT + STACK_H + 34} />
    </div>
  );
};

/* ------------------------------------------------------------------ brain labels and chips (right side) */

const BrainNotes: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.realLabel - 1 || f > T.tbOut + 8) return null;
  return (
    <>
      <Tag text="your real brain" at={T.realLabel} cx={P_RIGHT.x} top={722} out={T.realLabelOut} />
      {f < T.chipsOut + 8 ? (
        <div style={{ position: "absolute", left: P_RIGHT.x, top: 714, transform: "translateX(-50%)", display: "flex", gap: 20 }}>
          <Chip text="process" at={T.chip1} out={T.chipsOut} />
          <Chip text="every month" at={T.chip3} out={T.chipsOut} />
        </div>
      ) : null}
      {f >= T.taste - 1 ? (
        <div style={{ position: "absolute", left: P_RIGHT.x, top: 744, transform: "translateX(-50%)", display: "flex", gap: 20 }}>
          <Chip text="taste" at={T.taste} out={T.tbOut} />
          <Chip text="better" at={T.better} out={T.tbOut} icon={<UpArrow size={38} color={C.accent} />} />
        </div>
      ) : null}
    </>
  );
};

/* ------------------------------------------------------------------ files (go file by file) */

const FILE_W = 420;
const FILE_H = 480;
const FILE_GAP = 70;
const FILES_X = Math.round(960 - (FILE_W * 3 + FILE_GAP * 2) / 2); // 356
const TAB_H = 60;
const FILES_Y = 262; // tab top
const BARS = [300, 340, 236, 322, 280, 180];
const EDIT = 2; // the bar the caret edits

const FileCard: React.FC<{ i: number; caretAt: number | null; nextAt: number | null }> = ({ i, caretAt, nextAt }) => {
  const f = useCurrentFrame();
  const visit = caretAt != null ? prog(f, caretAt, 8, EASE_OUT) : 0;
  const active = caretAt != null && f >= caretAt && (nextAt == null || f < nextAt) ? 1 : 0;
  const flash = caretAt != null ? Math.max(0, 1 - Math.max(0, f - caretAt - 4) / 14) * (f >= caretAt ? 1 : 0) : 0;
  const editW = mix(visit, BARS[EDIT], 330);
  const blink = Math.floor((f - (caretAt ?? 0)) / 8) % 2 === 0 ? 1 : 0;
  return (
    <div style={{ position: "absolute", left: FILES_X + i * (FILE_W + FILE_GAP), top: FILES_Y, width: FILE_W, height: TAB_H + FILE_H, transform: `translateY(${(-active * 10).toFixed(2)}px)` }}>
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          width: 190,
          height: TAB_H + 10,
          borderRadius: "16px 16px 0 0",
          background: C.dark,
          border: `2px solid ${active ? C.accent : C.line}`,
          borderBottom: "none",
          boxSizing: "border-box",
          display: "flex",
          alignItems: "flex-start",
          paddingTop: 10,
          justifyContent: "center",
          fontFamily: FONT,
          fontSize: 32,
          fontWeight: 650,
          color: active ? C.ink : C.inkSoft,
          letterSpacing: "0.01em",
        }}
      >
        {`0${i + 1}.md`}
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          top: TAB_H,
          width: FILE_W,
          height: FILE_H,
          borderRadius: "0 24px 24px 24px",
          background: C.paper,
          border: `2px solid ${active ? C.accent : C.line}`,
          boxSizing: "border-box",
          boxShadow: active ? `${velour.shadow.soft}, 0 0 40px -12px ${accentA(0.6)}` : velour.shadow.soft,
        }}
      >
        <div style={{ position: "absolute", left: 38, top: 40, width: 210, height: 24, borderRadius: 12, background: C.darkLine }} />
        {BARS.map((w, b) => {
          const isEdit = b === EDIT;
          const width = isEdit ? editW : w;
          return (
            <div key={b} style={{ position: "absolute", left: 38, top: 108 + b * 56, width, height: 18, borderRadius: 9, background: isEdit ? (visit > 0 ? C.inkSoft : C.line) : C.line, opacity: isEdit && visit > 0 ? 0.55 + 0.45 * flash : 1 }}>
              {isEdit && flash > 0 ? <div style={{ position: "absolute", inset: 0, borderRadius: 8, background: C.accent, opacity: flash }} /> : null}
            </div>
          );
        })}
        {active ? <div style={{ position: "absolute", left: 38 + editW + 10, top: 108 + EDIT * 56 - 13, width: 6, height: 44, borderRadius: 3, background: C.accent, opacity: blink }} /> : null}
      </div>
    </div>
  );
};

const FilesBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.filesIn - 1 || f > T.files2Out + 8) return null;
  const first = f < T.blankIn;
  const inAt = first ? T.filesIn : T.files2In;
  const outAt = first ? T.filesOut : T.files2Out;
  if (!first && f < T.files2In - 1) return null;
  const o = outOf(f, outAt, 7);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      {[0, 1, 2].map((i) => {
        const p = prog(f, inAt + (first ? i * 4 : i * 2), 10, EASE_OUT);
        const caretAt = first ? null : T.caret[i];
        const nextAt = first ? null : i < 2 ? T.caret[i + 1] : T.files2Out + 8;
        return (
          <div key={i} style={{ position: "absolute", inset: 0, opacity: p, transform: `translateY(${((1 - p) * 36).toFixed(2)}px)`, filter: blurOf((1 - p) * 8) }}>
            <FileCard i={i} caretAt={caretAt} nextAt={nextAt} />
          </div>
        );
      })}
      {!first ? <Tag text="file by file" at={T.caret[0]} cx={960} top={172} size={34} /> : null}
    </div>
  );
};

const BlankBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.blankIn - 1 || f > T.blankOut + 8) return null;
  const p = prog(f, T.blankIn, 10, EASE_OUT);
  const o = outOf(f, T.blankOut, 6);
  const blink = Math.floor((f - T.blankIn) / 9) % 2 === 0 ? 1 : 0;
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <div
        style={{
          position: "absolute",
          left: 960 - 250,
          top: 290,
          width: 500,
          height: 540,
          borderRadius: 26,
          border: `3px dashed ${C.muted}`,
          boxSizing: "border-box",
          background: hexA(C.paper, 0.35),
          opacity: p,
          transform: `translateY(${((1 - p) * 30).toFixed(2)}px) scale(${mix(p, 0.96, 1).toFixed(4)})`,
          filter: blurOf((1 - p) * 8),
        }}
      >
        <div style={{ position: "absolute", left: 50, top: 52, width: 6, height: 48, borderRadius: 3, background: C.accent, opacity: blink }} />
      </div>
      <Tag text="almost from scratch" at={T.blankLabel} cx={960} top={172} size={34} />
    </div>
  );
};

/* ------------------------------------------------------------------ hub headline beats */

const WhateverBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.wtHead - 1 || f > T.wtOut + 8) return null;
  return <Words text={"“Whatever tool\nI want”"} at={T.wtHead} size={110} accent={["whatever"]} out={T.wtOut} style={{ left: 920, top: 404 }} />;
};

const SetBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.setHead - 1 || f > T.setOut + 8) return null;
  return <Words text="Set it up correctly" at={T.setHead} size={112} accent={["correctly"]} align="center" out={T.setOut} style={{ left: 0, width: 1920, top: 728 }} />;
};

const ClosingBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.end1 - 1) return null;
  const z = geoZoom(prog(f, T.end1, 1632 - T.end1, EASE_IN_OUT), 1, 1.02);
  return (
    <div style={{ position: "absolute", inset: 0, transform: `scale(${z.toFixed(5)})`, transformOrigin: "960px 500px" }}>
      <Words text="An AI second brain" at={T.end1} size={128} align="center" style={{ left: 0, width: 1920, top: 362 }} />
      <Words text="that evolves with you" at={T.end2} size={128} align="center" accent={["evolves", "with", "you"]} style={{ left: 0, width: 1920, top: 506 }} />
    </div>
  );
};

/* ------------------------------------------------------------------ proof 1: honcho.dev tool strip */

const TL = { iw: 1368, ih: 222, s: 1.3 };
const TL_W = Math.round(TL.iw * TL.s); // 1778
const TL_CH = Math.round(TL.ih * TL.s); // 289
const TL_H = BAR + TL_CH; // 361
const TL_X = Math.round(960 - TL_W / 2); // 71
const TL_Y = 470;
const TL_ROW = TL_Y + TL_H + 20;
const TL_ROW_X = 196.4; // the v1 pill's left edge (stage px, measured in the v1 render to 0.1 px)
const RING_CLAUDE: Rect = { x: 400, y: 112, w: 160, h: 63 }; // icon x 412-451 + "Claude" x 470-548, y 131-156
const RING_HERMES: Rect = { x: 1053, y: 112, w: 243, h: 63 }; // icon x 1065-1102 + "Hermes Agent" x 1121-1284, y 124-163
const stageOf = (sx: number, sy: number): [number, number] => [TL_X + sx * TL.s, TL_Y + BAR + sy * TL.s];

const TilesBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.tilesIn - 1 || f > T.tilesOut + 8) return null;
  const o = outOf(f, T.tilesOut);
  const z = geoZoom(prog(f, T.tilesIn, T.tilesOut + 6 - T.tilesIn, EASE_IN_OUT), 1, 1.02);
  const cTop = stageOf(RING_CLAUDE.x + 130, RING_CLAUDE.y);
  const hTop = stageOf(RING_HERMES.x + 120, RING_HERMES.y);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <BrowserCard x={TL_X} y={TL_Y} w={TL_W} h={TL_H} at={T.tilesIn} domain="honcho.dev" path="" page={C.bg}>
        <Shot src={`${P}tools-strip.png`} iw={TL.iw} ih={TL.ih} s={TL.s} z={z} o={[700, 140]}>
          <Ring r={RING_CLAUDE} at={T.ringClaude} s={TL.s} radius={10} />
          <Ring r={RING_HERMES} at={T.ringHermes} s={TL.s} radius={10} />
        </Shot>
      </BrowserCard>
      {/* left-anchored where the v1 pill sat, so the pill does not move when the ChatGPT note leaves */}
      <div style={{ position: "absolute", left: TL_ROW_X, top: TL_ROW, display: "flex", alignItems: "flex-start", gap: 24 }}>
        <SourcePill text="honcho.dev" tail="captured Oct 15, 2026" at={T.tilesIn + 4} />
        <HonestNote lines={["ChatGPT isn't on Honcho's list.", "Its desktop app connects through MCP."]} at={T.tilesNote} dur={8} out={T.tilesNoteOut} />
      </div>
      {/* arrows from the hub (top-left corner) to the tiles he names */}
      <CurveArrow p0={[420, 206]} c1={[680, 220]} c2={[cTop[0] + 6, 430]} p1={[cTop[0], cTop[1] - 12]} at={T.ringClaude + 1} />
      <CurveArrow p0={[362, 84]} c1={[880, 56]} c2={[hTop[0] - 10, 210]} p1={[hTop[0], hTop[1] - 12]} at={T.ringHermes + 1} dur={14} />
    </div>
  );
};

/* ------------------------------------------------------------------ proof 2: docs Integrations (OpenClaw, then the shared-workspace sentence) */

const OC = { iw: 540, ih: 300, s: 1.5 };
const OC_W = Math.round(OC.iw * OC.s); // 810
const OC_CH = Math.round(OC.ih * OC.s); // 450
const OC_H = BAR + OC_CH; // 522
const OC_CX = 1010;
const OC_Y = 204;
const OC_MARK: Rect = { x: 45, y: 246, w: 382, h: 36 }; // "automatic message capture." (x 51-421, y 251-277)

const OpenClawBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.a1In - 1 || f > T.a1Out + 8) return null;
  const o = outOf(f, T.a1Out);
  const z = geoZoom(prog(f, T.a1In, T.a1Out + 6 - T.a1In, EASE_IN_OUT), 1, 1.03);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <BrowserCard x={OC_CX - OC_W / 2} y={OC_Y} w={OC_W} h={OC_H} at={T.a1In} domain="honcho.dev" path="/docs/v3/guides/overview">
        <Shot src={`${P}openclaw-card.png`} iw={OC.iw} ih={OC.ih} s={OC.s} z={z} o={[240, 260]} filter="brightness(0.9)">
          <Highlighter r={OC_MARK} at={T.a1Mark} s={OC.s} />
        </Shot>
      </BrowserCard>
      <Row cx={OC_CX} top={OC_Y + OC_H + 20}>
        <SourcePill text="honcho.dev" tail="captured Oct 15, 2026" at={T.a1In + 4} />
        <HonestNote lines={["Plugins capture automatically.", "Other tools call Honcho when they choose to."]} at={T.a1In + 3} />
      </Row>
    </div>
  );
};

const IN = { iw: 1137, ih: 158, s: 1.25 };
const IN_W = Math.round(IN.iw * IN.s); // 1421
const IN_CH = Math.round(IN.ih * IN.s); // 198
const IN_H = BAR + IN_CH; // 270
const IN_CX = 1000;
const IN_Y = 384;
const IN_MARK: Rect = { x: 115, y: 84, w: 192, h: 36 }; // "carries over" (x 122-299, y 90-113)

const SharedBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.a2In - 1 || f > T.a2Out + 8) return null;
  const o = outOf(f, T.a2Out);
  const z = geoZoom(prog(f, T.a2In, T.a2Out + 6 - T.a2In, EASE_IN_OUT), 1, 1.02);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <BrowserCard x={IN_CX - IN_W / 2} y={IN_Y} w={IN_W} h={IN_H} at={T.a2In} domain="honcho.dev" path="/docs/v3/guides/overview">
        <Shot src={`${P}integrations-sentence.png`} iw={IN.iw} ih={IN.ih} s={IN.s} z={z} o={[210, 100]} filter="brightness(0.9)">
          <Highlighter r={IN_MARK} at={T.a2Mark} s={IN.s} />
        </Shot>
      </BrowserCard>
      <Row cx={IN_CX} top={IN_Y + IN_H + 20}>
        <SourcePill text="honcho.dev" tail="captured Oct 15, 2026" at={T.a2In + 4} />
      </Row>
    </div>
  );
};

/* ------------------------------------------------------------------ proof 3: docs Dreaming (experimental warning + Knowledge updates) */

const DS = 1.15;
const DW = { iw: 1015, ih: 142 }; // dreaming-warning.png (band edge at x 16)
const DB = { iw: 1150, ih: 263 }; // dreaming-bullet.png (bullet dot at x 68)
const DW_X = Math.round((68 - 22 - 16) * DS); // band edge sits 22 source px left of the bullet dot, as on the page
const DW_Y = 8;
const DB_Y = DW_Y + Math.round(DW.ih * DS); // 171
const D_W = 1340;
const D_CH = DB_Y + Math.round(DB.ih * DS) + 4; // 477
const D_H = BAR + D_CH;
const D_CX = 1095;
const D_Y = 190;
const D_MARK: Rect = { x: 534, y: 22, w: 568, h: 42 }; // "the same fact has changed over time" (x 540-1096, y 28-59)
const D_MARK2: Rect = { x: 316, y: 51, w: 278, h: 38 }; // "experimental feature" (x 321-590, y 55-85; word gaps ~12 px, so a band, not a ring)

const DreamingBeat: React.FC = () => {
  const f = useCurrentFrame();
  if (f < T.bIn - 1 || f > T.bOut + 8) return null;
  const o = outOf(f, T.bOut);
  const z = geoZoom(prog(f, T.bIn, T.bOut + 6 - T.bIn, EASE_IN_OUT), 1, 1.025);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      <BrowserCard x={D_CX - D_W / 2} y={D_Y} w={D_W} h={D_H} at={T.bIn} domain="honcho.dev" path="/docs/v3/documentation/features/advanced/dreaming">
        <div style={{ position: "absolute", inset: 0, background: C.white, filter: "brightness(0.9)", transform: `scale(${z.toFixed(5)})`, transformOrigin: "40% 40%" }}>
          <div style={{ position: "absolute", left: DW_X, top: DW_Y }}>
            <Shot src={`${P}dreaming-warning.png`} iw={DW.iw} ih={DW.ih} s={DS}>
              <Highlighter r={D_MARK2} at={T.bRing} s={DS} />
            </Shot>
          </div>
          <div style={{ position: "absolute", left: 0, top: DB_Y }}>
            <Shot src={`${P}dreaming-bullet.png`} iw={DB.iw} ih={DB.ih} s={DS}>
              <Highlighter r={D_MARK} at={T.bMark} s={DS} />
            </Shot>
          </div>
        </div>
      </BrowserCard>
      <Row cx={D_CX} top={D_Y + D_H + 20}>
        <SourcePill text="honcho.dev" tail="captured Oct 15, 2026" at={T.bIn + 4} />
        <HonestNote lines={["New facts come from every message.", "Dreaming, still experimental, replaces outdated ones."]} at={T.bIn + 4} />
      </Row>
    </div>
  );
};

/* ------------------------------------------------------------------ scene */

const Scene: React.FC = () => (
  <VelourCanvas tone="dark">
    <OpeningBeat />
    <StackBeat />
    <BrainHub />
    <BrainNotes />
    <FilesBeat />
    <BlankBeat />
    <WhateverBeat />
    <TilesBeat />
    <OpenClawBeat />
    <SharedBeat />
    <DreamingBeat />
    <SetBeat />
    <ClosingBeat />
  </VelourCanvas>
);

export default Scene;
