/**
 * The HANDOFF DIAGRAM for Tip 4 (anim-mvas6eqq): built once, reused at identical positions in beats 9, 10 and 12 to 15.
 * Stage px (1920x1080). Nodes 420x200 centred at (480, 580) and (1440, 580); top arrow y 530 (left to right), bottom
 * arrow y 630 (right to left); skill chip 380x116 centred at (960, 432) with a 3 px accent stem down to the top arrow;
 * Me node 260x120 centred at (960, 860). Draw order (bottom to top): arrows + stem, packets, nodes, chip.
 * Node status: WORKING = three pulsing dots + an advancing accent bar; WAITING = a static pause glyph + the bar frozen
 * and dimmed to 40%; IDLE = empty status row, bar hidden; CHECK = a mint check (briefly, when the result lands).
 */
import React from "react";
import { interpolate, interpolateColors, useCurrentFrame } from "remotion";
import { C, EASE_IN_OUT, EASE_OUT, Kicker, mix, prog } from "../../../styles/velour/src";

const FONT = "'Inter', system-ui, -apple-system, 'Helvetica Neue', Arial, sans-serif";
const MONO = "ui-monospace, 'SF Mono', Menlo, Consolas, monospace";

export const NODE_W = 420;
export const NODE_H = 200;
export const NODE_Y = 580; // centre y
export const LEFT_CX = 480;
export const RIGHT_CX = 1440;
export const TOP_ARROW_Y = 530;
export const BOT_ARROW_Y = 630;
export const ME = { cx: 960, cy: 860, w: 260, h: 120 };
export const CHIP = { cx: 960, cy: 432, w: 380, h: 116 };

export type Status = "idle" | "working" | "waiting" | "check";
export type Seg = [number, Status];

/** Opacity of the runs of `segs` whose status passes `pred` (consecutive passing segments merge into one run). */
export const segOp = (
  frame: number,
  segs: Seg[],
  pred: (s: Status) => boolean,
  lead = 2,
  inDur = 5,
  outDur = 4,
): number => {
  let op = 0;
  let i = 0;
  while (i < segs.length) {
    if (!pred(segs[i][1])) {
      i++;
      continue;
    }
    const s = segs[i][0];
    let j = i;
    while (j + 1 < segs.length && pred(segs[j + 1][1])) j++;
    const e = j + 1 < segs.length ? segs[j + 1][0] : Infinity;
    op += prog(frame, s + lead, inDur) * (e === Infinity ? 1 : 1 - prog(frame, e, outDur, EASE_IN_OUT));
    i = j + 1;
  }
  return Math.min(1, op);
};

/** Piecewise-linear value over [frame, value] keys (clamped). */
export const keyed = (frame: number, keys: [number, number][]): number =>
  interpolate(
    frame,
    keys.map((k) => k[0]),
    keys.map((k) => k[1]),
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" },
  );

/* ------------------------------------------------------------------ node */

const Dots: React.FC<{ op: number }> = ({ op }) => {
  const f = useCurrentFrame();
  return (
    <div style={{ display: "flex", gap: 12, opacity: op }}>
      {[0, 1, 2].map((i) => {
        const w = 0.5 + 0.5 * Math.sin(((f - i * 6) / 24) * Math.PI * 2);
        return (
          <div
            key={i}
            style={{
              width: 12,
              height: 12,
              borderRadius: 6,
              background: C.muted,
              opacity: 0.3 + 0.7 * w,
              transform: `translateY(${-3 * w}px)`,
            }}
          />
        );
      })}
    </div>
  );
};

const PauseGlyph: React.FC<{ op: number }> = ({ op }) => (
  <div style={{ display: "flex", gap: 10, opacity: op }}>
    <div style={{ width: 8, height: 26, borderRadius: 2, background: C.muted }} />
    <div style={{ width: 8, height: 26, borderRadius: 2, background: C.muted }} />
  </div>
);

const CheckGlyph: React.FC<{ op: number }> = ({ op }) => (
  <svg width={34} height={30} viewBox="0 0 34 30" style={{ opacity: op }}>
    <path d="M4 16 L13 25 L30 5" fill="none" stroke={C.mint} strokeWidth={5} strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

export const NodeBox: React.FC<{
  cx: number;
  label: string;
  segs: Seg[];
  bar: [number, number][];
  /** frame the border lights C.accent (optional) */
  lightAt?: number;
  ring?: { color: string; at: number };
  pulse?: { color: string; at: number };
}> = ({ cx, label, segs, bar, lightAt, ring, pulse }) => {
  const f = useCurrentFrame();
  const left = cx - NODE_W / 2;
  const top = NODE_Y - NODE_H / 2;
  const border = lightAt === undefined ? C.line : interpolateColors(prog(f, lightAt, 8), [0, 1], [C.line, C.accent]);
  // glyph swaps are sequenced: the old glyph is gone (3 frames) before the new one fades in (4 frames)
  const working = segOp(f, segs, (s) => s === "working", 3, 4, 3);
  const waiting = segOp(f, segs, (s) => s === "waiting", 3, 4, 3);
  const check = segOp(f, segs, (s) => s === "check", 3, 4, 3);
  const barVis = segOp(f, segs, (s) => s !== "idle", 0, 6, 6);
  const dim = 1 - 0.6 * segOp(f, segs, (s) => s === "waiting", 0, 5, 5);
  const fill = keyed(f, bar);
  const ringP = ring ? prog(f, ring.at, 10) : 0;
  const pulseP = pulse ? prog(f, pulse.at, 26, EASE_OUT) : 0;
  return (
    <div style={{ position: "absolute", left, top, width: NODE_W, height: NODE_H }}>
      {ring ? (
        <div
          style={{
            position: "absolute",
            inset: -12,
            borderRadius: 40,
            border: `4px solid ${ring.color}`,
            opacity: ringP,
            transform: `scale(${mix(ringP, 1.05, 1)})`,
          }}
        />
      ) : null}
      {pulse && pulseP > 0 && pulseP < 1 ? (
        <div
          style={{
            position: "absolute",
            inset: -12 - 22 * pulseP,
            borderRadius: 40 + 22 * pulseP,
            border: `4px solid ${pulse.color}`,
            opacity: 0.85 * (1 - pulseP),
          }}
        />
      ) : null}
      <div
        style={{
          position: "absolute",
          inset: 0,
          borderRadius: 28,
          background: C.paper,
          border: `2px solid ${border}`,
          boxShadow: "0 24px 48px -24px rgba(0,0,0,0.8)",
        }}
      />
      {/* label centred at y 560 (node-local 80) */}
      <div
        style={{
          position: "absolute",
          left: 0,
          width: NODE_W,
          top: 80 - 32,
          height: 64,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: FONT,
          fontSize: 52,
          fontWeight: 700,
          letterSpacing: "-0.02em",
          color: C.ink,
          whiteSpace: "nowrap",
        }}
      >
        {label}
      </div>
      {/* status row centred at y 618 (node-local 138) */}
      <div style={{ position: "absolute", left: 0, width: NODE_W, top: 138 - 15, height: 30 }}>
        <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center" }}>
          <Dots op={working} />
        </div>
        <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center" }}>
          <PauseGlyph op={waiting} />
        </div>
        <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center" }}>
          <CheckGlyph op={check} />
        </div>
      </div>
      {/* progress bar y 647-653 (node-local 167-173), x cx-170..cx+170 */}
      <div style={{ position: "absolute", left: 40, top: 167, width: 340, height: 6, borderRadius: 3, background: C.line, opacity: barVis * dim }}>
        <div style={{ width: `${fill * 100}%`, height: 6, borderRadius: 3, background: C.accent }} />
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------ arrows, packets */

/** A horizontal arrow with a head at `x2`, drawn on from x1 over [at, at+dur]. */
export const HArrow: React.FC<{ x1: number; x2: number; y: number; at: number; dur?: number; color?: string }> = ({
  x1,
  x2,
  y,
  at,
  dur = 14,
  color = C.faint,
}) => {
  const f = useCurrentFrame();
  const p = prog(f, at, dur, EASE_IN_OUT);
  if (p <= 0) return null;
  const dir = Math.sign(x2 - x1);
  const xe = x1 + (x2 - x1) * p;
  const head = prog(f, at + dur * 0.7, dur * 0.3);
  return (
    <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }} width={1920} height={1080}>
      <line x1={x1} y1={y} x2={xe - dir * 4} y2={y} stroke={color} strokeWidth={3} strokeLinecap="round" />
      <polyline
        points={`${x2 - dir * 16},${y - 11} ${x2},${y} ${x2 - dir * 16},${y + 11}`}
        fill="none"
        stroke={color}
        strokeWidth={3}
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity={head}
      />
    </svg>
  );
};

/** A packet pill centred at (x, y) (accent fill, white text); no text = a small snippet. */
export const Packet: React.FC<{ x: number; y: number; w: number; text?: string; op?: number }> = ({ x, y, w, text, op = 1 }) => {
  const h = text ? 50 : 24;
  return (
    <div
      style={{
        position: "absolute",
        left: x - w / 2,
        top: y - h / 2,
        width: w,
        height: h,
        borderRadius: h / 2,
        background: C.accent,
        color: C.white,
        fontFamily: FONT,
        fontSize: 30,
        fontWeight: 700,
        letterSpacing: "-0.01em",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        whiteSpace: "nowrap",
        opacity: op,
        boxShadow: "0 10px 24px -12px rgba(0,0,0,0.8)",
      }}
    >
      {text}
    </div>
  );
};

/** Centre x of a packet moving along keys [[frame, x], ...] with an easing per segment (default ease-in-out). */
export const packetX = (frame: number, keys: [number, number, ((t: number) => number)?][]): number => {
  if (frame <= keys[0][0]) return keys[0][1];
  for (let i = 1; i < keys.length; i++) {
    const [f1, x1, e] = keys[i];
    const [f0, x0] = keys[i - 1];
    if (frame <= f1) return x0 + (x1 - x0) * prog(frame, f0, f1 - f0, e ?? EASE_IN_OUT);
  }
  return keys[keys.length - 1][1];
};

/* ------------------------------------------------------------------ skill chip */

export const SkillChip: React.FC<{ at: number; typeFrom: number; typeTo: number; text: string; pulses: number[] }> = ({
  at,
  typeFrom,
  typeTo,
  text,
  pulses,
}) => {
  const f = useCurrentFrame();
  const t = prog(f, at, 14, (x) => x);
  const s0 = 1.70158 * 1.2;
  const back = t === 0 ? 0 : 1 + (s0 + 1) * Math.pow(t - 1, 3) + s0 * Math.pow(t - 1, 2);
  let pulse = 0;
  for (const p of pulses) {
    const k = prog(f, p, 14, (x) => x);
    if (k > 0 && k < 1) pulse = Math.max(pulse, Math.sin(k * Math.PI));
  }
  const n = Math.round(interpolate(f, [typeFrom, typeTo], [0, text.length], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));
  const typing = f >= typeFrom && f < typeTo;
  const cursorOn = typing || Math.floor((f - at) / 15) % 2 === 0;
  return (
    <div
      style={{
        position: "absolute",
        left: CHIP.cx - CHIP.w / 2,
        top: CHIP.cy - CHIP.h / 2,
        width: CHIP.w,
        height: CHIP.h,
        opacity: Math.min(1, t * 2),
        transform: `scale(${mix(back, 0.6, 1) * (1 + 0.045 * pulse)})`,
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          borderRadius: 22,
          background: C.paper,
          border: `2px solid ${interpolateColors(pulse, [0, 1], [C.line, C.accent])}`,
          boxShadow: "0 24px 48px -24px rgba(0,0,0,0.85)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: 8,
        }}
      >
        <Kicker text="Skill" size={28} delay={at + 2} />
        <div style={{ fontFamily: MONO, fontSize: 36, color: C.ink, height: 44, display: "flex", alignItems: "center", whiteSpace: "pre" }}>
          {text.slice(0, n)}
          <span style={{ display: "inline-block", width: 3, height: 38, marginLeft: 3, background: C.accent, opacity: cursorOn ? 1 : 0 }} />
        </div>
      </div>
    </div>
  );
};

/** The chip's stem: 3 px accent from the chip's bottom edge (y 490) down to the top arrow (y 530) at x 960. */
export const ChipStem: React.FC<{ at: number }> = ({ at }) => {
  const f = useCurrentFrame();
  const p = prog(f, at, 10, EASE_IN_OUT);
  const y0 = CHIP.cy + CHIP.h / 2;
  return <div style={{ position: "absolute", left: CHIP.cx - 1.5, top: y0, width: 3, height: (TOP_ARROW_Y - y0) * p, background: C.accent, borderRadius: 2 }} />;
};

/* ------------------------------------------------------------------ Me node and its links */

export const LINK_L = { a: [570, 680] as [number, number], b: [880, 800] as [number, number] };
export const LINK_R = { a: [1350, 680] as [number, number], b: [1040, 800] as [number, number] };

/** A dotted link drawn from a to b over [at, at+dur]. */
export const DottedLink: React.FC<{ a: [number, number]; b: [number, number]; at: number; dur?: number }> = ({ a, b, at, dur = 10 }) => {
  const f = useCurrentFrame();
  const p = prog(f, at, dur, EASE_IN_OUT);
  if (p <= 0) return null;
  const x2 = a[0] + (b[0] - a[0]) * p;
  const y2 = a[1] + (b[1] - a[1]) * p;
  return (
    <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }} width={1920} height={1080}>
      <line x1={a[0]} y1={a[1]} x2={x2} y2={y2} stroke={C.muted} strokeWidth={5} strokeLinecap="round" strokeDasharray="0 15" />
    </svg>
  );
};

/** Point on the line a->b extended by `ext` px past both ends, at t in [0, 1]. */
export const along = (a: [number, number], b: [number, number], t: number, ext = 40): [number, number] => {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const L = Math.hypot(dx, dy);
  const ux = dx / L;
  const uy = dy / L;
  const sx = a[0] - ux * ext;
  const sy = a[1] - uy * ext;
  const ex = b[0] + ux * ext;
  const ey = b[1] + uy * ext;
  return [sx + (ex - sx) * t, sy + (ey - sy) * t];
};

export const MeNode: React.FC = () => (
  <div
    style={{
      position: "absolute",
      left: ME.cx - ME.w / 2,
      top: ME.cy - ME.h / 2,
      width: ME.w,
      height: ME.h,
      borderRadius: 24,
      background: C.dark,
      border: `2px solid ${C.darkLine}`,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      gap: 16,
      fontFamily: FONT,
      boxShadow: "0 24px 48px -24px rgba(0,0,0,0.85)",
    }}
  >
    <svg width={34} height={42} viewBox="0 0 34 42">
      <rect x={3} y={6} width={28} height={33} rx={5} fill="none" stroke={C.muted} strokeWidth={3} />
      <rect x={10} y={2} width={14} height={9} rx={3} fill={C.dark} stroke={C.muted} strokeWidth={3} />
      <line x1={10} y1={21} x2={24} y2={21} stroke={C.muted} strokeWidth={3} strokeLinecap="round" />
      <line x1={10} y1={29} x2={20} y2={29} stroke={C.muted} strokeWidth={3} strokeLinecap="round" />
    </svg>
    <div style={{ fontSize: 46, fontWeight: 700, color: C.onDark, letterSpacing: "-0.02em" }}>Me</div>
  </div>
);

