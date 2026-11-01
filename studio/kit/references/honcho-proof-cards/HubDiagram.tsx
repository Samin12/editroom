/**
 * HubDiagram: the "one shared brain" hub, built in H1 (anim-mvagv66m, scene 48b6) and reused EXACTLY by H2, H3, D1,
 * K1 and E1. Import it, do not fork it:
 *   import { HubDiagram, NodeBox, BoxRing, HUB_NODE, HUB_CHIPS } from "../anim-mvagv66m/HubDiagram";
 *
 * Geometry (1920x1080 stage px, the stage VelourCanvas scales x0.768 into the left panel):
 *   Honcho node 340x160 centred (960, 520), label "Honcho" 48 px, accent border.
 *   Chips 340x88, label 36 px: Claude Code (470, 300), Codex (1450, 300), OpenCode (300, 580), OpenClaw (1620, 580),
 *   Hermes Agent (960, 790). Lines run from the node edge to each chip edge (centre-to-centre ray).
 *   Unlit: line C.line, chip border C.line, chip text C.muted. Lit: line + chip border C.accent, text C.ink.
 *   Optional label above the node (centre y 364) and one bottom line (centre y 900). Nothing above y 120 or below 960.
 * Every value is a function of the frame (props are frame numbers in the host composition). Leave a prop undefined
 * to have that part already on screen (for example a job that opens on the finished hub passes no nodeAt/chipsAt and
 * lit values <= 0).
 */
import React from "react";
import { interpolateColors, useCurrentFrame } from "remotion";
import { C, EASE_IN_OUT, EASE_OUT, mix, prog, velour } from "../../../styles/velour/src";

const FONT = velour.font.display;

export type HubTool = "claudeCode" | "codex" | "opencode" | "openclaw" | "hermes";

export const HUB_NODE = { cx: 960, cy: 520, w: 340, h: 160, r: 32 } as const;
export const HUB_CHIP = { w: 340, h: 88 } as const;
export const HUB_CHIPS: readonly { id: HubTool; label: string; cx: number; cy: number }[] = [
  { id: "claudeCode", label: "Claude Code", cx: 470, cy: 300 },
  { id: "codex", label: "Codex", cx: 1450, cy: 300 },
  { id: "opencode", label: "OpenCode", cx: 300, cy: 580 },
  { id: "openclaw", label: "OpenClaw", cx: 1620, cy: 580 },
  { id: "hermes", label: "Hermes Agent", cx: 960, cy: 790 },
];
/** centre y of the optional label above the node, and of the optional bottom line */
export const HUB_TOP_LABEL_Y = 364;
export const HUB_BOTTOM_LABEL_Y = 900;

const hexA = (hex: string, a: number) => {
  const n = parseInt(hex.replace("#", ""), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
};
export const accentA = (a: number) => hexA(C.accent, a);
export const HUB_SHADOW = `${velour.shadow.card}, 0 0 0 1.5px rgba(255,255,255,0.06)`;
const blurOf = (v: number) => (v > 0.02 ? `blur(${v.toFixed(2)}px)` : undefined);

/** Where the centre-to-centre ray leaves a box of half-size hw x hh. */
const exitPt = (cx: number, cy: number, hw: number, hh: number, dx: number, dy: number): [number, number] => {
  const tx = dx === 0 ? Infinity : hw / Math.abs(dx);
  const ty = dy === 0 ? Infinity : hh / Math.abs(dy);
  const t = Math.min(tx, ty);
  return [cx + dx * t, cy + dy * t];
};

/** The line from the node edge to a chip edge. */
export const hubLine = (id: HubTool) => {
  const c = HUB_CHIPS.find((k) => k.id === id)!;
  const dx = c.cx - HUB_NODE.cx;
  const dy = c.cy - HUB_NODE.cy;
  const [x1, y1] = exitPt(HUB_NODE.cx, HUB_NODE.cy, HUB_NODE.w / 2, HUB_NODE.h / 2, dx, dy);
  const [x2, y2] = exitPt(c.cx, c.cy, HUB_CHIP.w / 2, HUB_CHIP.h / 2, -dx, -dy);
  return { x1, y1, x2, y2, len: Math.hypot(x2 - x1, y2 - y1) };
};

/* ------------------------------------------------------------------ node box */

/**
 * A rounded diagram box with a centred label (the Honcho node, a "You" node, "Chat 1" ...). Rises in (rise + de-blur)
 * from `at` over `dur` frames; `pulseAt` bumps it 1.00 to 1.03 and back over 33 frames; `out` fades it.
 */
export const NodeBox: React.FC<{
  cx: number;
  cy: number;
  w?: number;
  h?: number;
  r?: number;
  label: React.ReactNode;
  size?: number;
  tone?: "accent" | "plain";
  at?: number;
  dur?: number;
  rise?: number;
  out?: number;
  outDur?: number;
  pulseAt?: number;
  icon?: React.ReactNode;
}> = ({ cx, cy, w = HUB_NODE.w, h = HUB_NODE.h, r = HUB_NODE.r, label, size = 48, tone = "plain", at, dur = 12, rise = 36, out, outDur = 12, pulseAt, icon }) => {
  const f = useCurrentFrame();
  const p = at == null ? 1 : prog(f, at, dur, EASE_OUT);
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  const bump = pulseAt == null ? 0 : Math.sin(Math.PI * prog(f, pulseAt, 33, (t) => t)) * 0.03;
  const accent = tone === "accent";
  return (
    <div
      style={{
        position: "absolute",
        left: cx - w / 2,
        top: cy - h / 2,
        width: w,
        height: h,
        borderRadius: r,
        background: C.paper,
        border: `3px solid ${accent ? C.accent : C.darkLine}`,
        boxSizing: "border-box",
        boxShadow: accent ? `${HUB_SHADOW}, 0 0 60px -18px ${accentA(0.55)}` : HUB_SHADOW,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: size * 0.4,
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 800,
        letterSpacing: "-0.025em",
        color: C.ink,
        whiteSpace: "nowrap",
        opacity: p * o,
        transform: `translateY(${((1 - p) * rise - (1 - o) * 10).toFixed(2)}px) scale(${(mix(p, 0.96, 1) * mix(o, 0.97, 1) * (1 + bump)).toFixed(5)})`,
        filter: blurOf((1 - p) * 8 + (1 - o) * 6),
      }}
    >
      {icon}
      {label}
    </div>
  );
};

/* ------------------------------------------------------------------ ring */

/** Accent ring drawn on around a stage-px box (centre cx, cy), `m` px outside it. `out` fades it over 6 frames. */
export const BoxRing: React.FC<{
  cx: number;
  cy: number;
  w: number;
  h: number;
  r?: number;
  m?: number;
  at: number;
  dur?: number;
  out?: number;
  outDur?: number;
  stroke?: number;
}> = ({ cx, cy, w, h, r = 30, m = 14, at, dur = 12, out, outDur = 6, stroke = 6 }) => {
  const f = useCurrentFrame();
  const p = prog(f, at, dur, EASE_IN_OUT);
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  const glow = 0.45 + 0.55 * prog(f, at + 8, 10);
  const W = w + m * 2;
  const H = h + m * 2;
  return (
    <svg
      width={W + stroke * 2}
      height={H + stroke * 2}
      style={{
        position: "absolute",
        left: cx - W / 2 - stroke,
        top: cy - H / 2 - stroke,
        overflow: "visible",
        opacity: o,
        filter: `drop-shadow(0 0 ${(16 * glow).toFixed(2)}px ${accentA(0.3 + 0.35 * glow)})`,
      }}
    >
      <rect
        x={stroke}
        y={stroke}
        width={W}
        height={H}
        rx={r + m}
        fill="none"
        stroke={C.accent}
        strokeWidth={stroke}
        strokeLinecap="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1 - p}
      />
    </svg>
  );
};

/* ------------------------------------------------------------------ labels */

/** One centred line of text at centre y `cy` (rise + de-blur in from `at`, fade at `out`). */
export const HubText: React.FC<{
  text: React.ReactNode;
  cy: number;
  at?: number;
  dur?: number;
  out?: number;
  outDur?: number;
  size?: number;
  weight?: number;
  color?: string;
  cx?: number;
}> = ({ text, cy, at, dur = 12, out, outDur = 12, size = 40, weight = 750, color = C.ink, cx = 960 }) => {
  const f = useCurrentFrame();
  const p = at == null ? 1 : prog(f, at, dur, EASE_OUT);
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: cx - 940,
        width: 1880,
        top: cy - size * 0.7,
        height: size * 1.4,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        letterSpacing: "-0.015em",
        color,
        whiteSpace: "nowrap",
        opacity: p * o,
        transform: `translateY(${((1 - p) * 16 - (1 - o) * 8).toFixed(2)}px)`,
        filter: blurOf((1 - p) * 6 + (1 - o) * 5),
      }}
    >
      {text}
    </div>
  );
};

/* ------------------------------------------------------------------ the hub */

export const HubDiagram: React.FC<{
  /** node entrance frame (rises over 12 f); undefined = already on screen */
  nodeAt?: number;
  /** chips + unlit lines entrance frame; undefined = already on screen */
  chipsAt?: number;
  stagger?: number;
  /** frame each chip lights (border + text + its line drawn in accent from the node out) */
  lit?: Partial<Record<HubTool, number>>;
  /** node bump 1.00 to 1.03 and back */
  pulseAt?: number;
  /** accent ring around the node */
  ringAt?: number;
  ringOut?: number;
  topLabel?: { text: React.ReactNode; at?: number; out?: number };
  bottomLabel?: { text: React.ReactNode; at?: number; out?: number };
  out?: number;
  outDur?: number;
}> = ({ nodeAt, chipsAt, stagger = 1, lit = {}, pulseAt, ringAt, ringOut, topLabel, bottomLabel, out, outDur = 14 }) => {
  const f = useCurrentFrame();
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (o <= 0) return null;
  const lineP = chipsAt == null ? 1 : prog(f, chipsAt, 12, EASE_IN_OUT);
  return (
    <div style={{ position: "absolute", inset: 0, opacity: o, filter: blurOf((1 - o) * 6) }}>
      {/* lines (under the boxes) */}
      <svg style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, overflow: "visible" }} viewBox="0 0 1920 1080">
        {HUB_CHIPS.map((c) => {
          const L = hubLine(c.id);
          const litAt = lit[c.id];
          const lp = litAt == null ? 0 : prog(f, litAt, 14, EASE_IN_OUT);
          const d = `M ${L.x1.toFixed(1)} ${L.y1.toFixed(1)} L ${L.x2.toFixed(1)} ${L.y2.toFixed(1)}`;
          return (
            <g key={c.id}>
              {lineP > 0 ? (
                <path d={d} fill="none" stroke={C.line} strokeWidth={5} strokeLinecap="round" strokeDasharray={L.len} strokeDashoffset={L.len * (1 - lineP)} />
              ) : null}
              {lp > 0 ? (
                <>
                  <path d={d} fill="none" stroke={accentA(0.22)} strokeWidth={16} strokeLinecap="round" strokeDasharray={L.len} strokeDashoffset={L.len * (1 - lp)} />
                  <path d={d} fill="none" stroke={C.accent} strokeWidth={5} strokeLinecap="round" strokeDasharray={L.len} strokeDashoffset={L.len * (1 - lp)} />
                </>
              ) : null}
            </g>
          );
        })}
      </svg>
      {/* chips */}
      {HUB_CHIPS.map((c, i) => {
        const p = chipsAt == null ? 1 : prog(f, chipsAt + i * stagger, 11, EASE_OUT);
        if (p <= 0) return null;
        const litAt = lit[c.id];
        const lp = litAt == null ? 0 : prog(f, litAt + 4, 10, EASE_OUT);
        const bump = litAt == null ? 0 : Math.sin(Math.PI * prog(f, litAt + 4, 14, (t) => t)) * 0.05;
        return (
          <div
            key={c.id}
            style={{
              position: "absolute",
              left: c.cx - HUB_CHIP.w / 2,
              top: c.cy - HUB_CHIP.h / 2,
              width: HUB_CHIP.w,
              height: HUB_CHIP.h,
              borderRadius: HUB_CHIP.h / 2,
              background: C.paper,
              border: `3px solid ${interpolateColors(lp, [0, 1], [C.line, C.accent])}`,
              boxSizing: "border-box",
              boxShadow: lp > 0 ? `${HUB_SHADOW}, 0 0 44px -14px ${accentA(0.6 * lp)}` : HUB_SHADOW,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontFamily: FONT,
              fontSize: 36,
              fontWeight: 750,
              letterSpacing: "-0.015em",
              color: interpolateColors(lp, [0, 1], [C.muted, C.ink]),
              whiteSpace: "nowrap",
              opacity: p,
              transform: `translateY(${((1 - p) * 22).toFixed(2)}px) scale(${(mix(p, 0.94, 1) * (1 + bump)).toFixed(5)})`,
              filter: blurOf((1 - p) * 7),
            }}
          >
            {c.label}
          </div>
        );
      })}
      {/* the Honcho node */}
      <NodeBox cx={HUB_NODE.cx} cy={HUB_NODE.cy} label="Honcho" size={48} tone="accent" at={nodeAt} pulseAt={pulseAt} />
      {ringAt != null ? <BoxRing cx={HUB_NODE.cx} cy={HUB_NODE.cy} w={HUB_NODE.w} h={HUB_NODE.h} r={HUB_NODE.r} at={ringAt} out={ringOut} /> : null}
      {topLabel ? <HubText text={topLabel.text} cy={HUB_TOP_LABEL_Y} at={topLabel.at} out={topLabel.out} size={40} weight={750} /> : null}
      {bottomLabel ? <HubText text={bottomLabel.text} cy={HUB_BOTTOM_LABEL_Y} at={bottomLabel.at} out={bottomLabel.out} size={36} weight={650} color={C.inkSoft} /> : null}
    </div>
  );
};
