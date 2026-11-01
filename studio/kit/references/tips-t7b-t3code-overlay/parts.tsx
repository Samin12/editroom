/**
 * anim-mvas6eyk building blocks (FRONT overlay, every coordinate is a FINAL-FRAME pixel).
 * Rings on the real UI, proof cards that show a REGION of a real capture, source pills, diagram nodes.
 * Everything is a pure function of the frame. Colours from C.* only.
 */
import React from "react";
import { Img, interpolateColors, staticFile, useCurrentFrame } from "remotion";
import { C, EASE_IN_OUT, EASE_OUT, geoZoom, mix, prog, velour } from "../../../styles/velour/src";

export const FONT = velour.font.display;
export type Box = readonly [number, number, number, number];
export const CAPTURED = "Oct 16, 2026";
export const proofSrc = (f: string) => staticFile(`opus-tips/proof/anim-mvas6eyk/${f}`);

/** A soft drop shadow so cards lift off the footage (no colour token involved, pure black alpha). */
export const LIFT = "0 18px 36px -14px rgba(0,0,0,0.85), 0 6px 14px -6px rgba(0,0,0,0.6)";

/** A C.* hex colour with an alpha (0..1): "#E2774E" -> "#E2774E59". */
export const withAlpha = (c: string, a: number): string =>
  `${c}${Math.round(Math.min(1, Math.max(0, a)) * 255).toString(16).padStart(2, "0")}`;

/** Opacity of something that enters at `a` (dur frames) and leaves at `out` (outDur frames). */
export const life = (f: number, a: number, dur: number, out?: number, outDur = 8): number =>
  prog(f, a, dur) * (out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT));

/** Rise + de-blur entrance (<= 12 frames) and a short fade exit. Positioned absolutely at (x, y). */
export const Enter: React.FC<{
  at: number;
  out?: number;
  outDur?: number;
  dur?: number;
  x: number;
  y: number;
  rise?: number;
  blur?: number;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}> = ({ at, out, outDur = 8, dur = 12, x, y, rise = 14, blur = 6, style, children }) => {
  const f = useCurrentFrame();
  const p = prog(f, at, dur);
  const o = out == null ? 1 : 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        opacity: p * o,
        transform: `translateY(${(1 - p) * rise - (1 - o) * 8}px) scale(${mix(p, 0.985, 1)})`,
        transformOrigin: "50% 50%",
        filter: p < 1 || o < 1 ? `blur(${(1 - p) * blur + (1 - o) * 3}px)` : undefined,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

/**
 * An accent rounded-rectangle ring that draws on at keys[0].at, then MOVES (eased) to each next box at its frame,
 * with a faint opacity pulse while it holds. Boxes are the ring's own rectangle in the PARENT's pixels.
 * The stroke is centred on the box edge, so it reaches stroke/2 outside the box (no glow, no halo).
 */
export const RingBox: React.FC<{
  keys: { at: number; box: Box }[];
  out: number;
  outDur?: number;
  stroke?: number;
  radius?: number;
  pad?: number;
  draw?: number;
  morph?: number;
  color?: string;
}> = ({ keys, out, outDur = 7, stroke = 3.5, radius = 10, pad = 0, draw = 9, morph = 8, color = C.accent }) => {
  const f = useCurrentFrame();
  if (f < keys[0].at) return null;
  const o = 1 - prog(f, out, outDur, EASE_IN_OUT);
  if (o <= 0) return null;
  let b: number[] = [...keys[0].box];
  for (let i = 1; i < keys.length; i++) {
    const t = prog(f, keys[i].at, morph, EASE_IN_OUT);
    if (t <= 0) break;
    b = b.map((v, k) => mix(t, v, keys[i].box[k]));
  }
  const x = b[0] - pad;
  const y = b[1] - pad;
  const w = b[2] - b[0] + 2 * pad;
  const h = b[3] - b[1] + 2 * pad;
  const d = prog(f, keys[0].at, draw);
  const len = 2 * (w + h);
  const since = f - keys[0].at - draw;
  const pulse = since > 0 ? 0.87 + 0.13 * Math.cos((since / 40) * 2 * Math.PI) : 1;
  return (
    <svg
      style={{ position: "absolute", left: x - stroke, top: y - stroke, overflow: "visible", opacity: o * pulse, pointerEvents: "none" }}
      width={w + 2 * stroke}
      height={h + 2 * stroke}
    >
      <rect
        x={stroke}
        y={stroke}
        width={w}
        height={h}
        rx={radius}
        ry={radius}
        fill="none"
        stroke={color}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={len}
        strokeDashoffset={len * (1 - d)}
      />
    </svg>
  );
};

/** A translucent accent marker swipe over one or more text lines (source px * s), moving from key to key. */
export const MarkerSet: React.FC<{ keys: { at: number; rects: Box[]; out?: number }[]; s: number; out: number }> = ({ keys, s, out }) => {
  const f = useCurrentFrame();
  const endO = 1 - prog(f, out, 7, EASE_IN_OUT);
  return (
    <>
      {keys.map((k, i) => {
        const next = keys[i + 1]?.at;
        const leaveAt = k.out ?? (next != null ? next - 2 : undefined);
        const o = (leaveAt == null ? 1 : 1 - prog(f, leaveAt, 5, EASE_IN_OUT)) * endO;
        if (f < k.at || o <= 0) return null;
        return k.rects.map((r, j) => {
          const t = prog(f, k.at + j * 5, 9);
          const pad = 4;
          return (
            <div
              key={`${i}-${j}`}
              style={{
                position: "absolute",
                left: r[0] * s - pad,
                top: r[1] * s - pad,
                width: ((r[2] - r[0]) * s + 2 * pad) * t,
                height: (r[3] - r[1]) * s + 2 * pad,
                borderRadius: 6,
                background: C.accent,
                opacity: 0.34 * o * (t > 0 ? 1 : 0),
                mixBlendMode: "screen",
              }}
            />
          );
        });
      })}
    </>
  );
};

/**
 * A REGION of a real capture at `scale`, clipped to that region, with an optional eased pan to another region of the
 * same size, a slow geometric push (<= 3%) toward a source-px focus, and soft edge fades. Children are positioned in
 * SCALED SOURCE pixels (box * scale), so highlights ride with the image through pan and push.
 */
export const ImgRegion: React.FC<{
  src: string;
  srcW: number;
  scale: number;
  region: Box;
  pan?: { from: number; to: number; region: Box };
  push?: { from: number; to: number; z: number; focus: [number, number] };
  radius?: number | string;
  fadeL?: number;
  fadeR?: number;
  children?: React.ReactNode;
}> = ({ src, srcW, scale: s, region, pan, push, radius = 0, fadeL = 0, fadeR = 0, children }) => {
  const f = useCurrentFrame();
  const W = (region[2] - region[0]) * s;
  const H = (region[3] - region[1]) * s;
  const pt = pan ? prog(f, pan.from, pan.to - pan.from, EASE_IN_OUT) : 0;
  const ox = pan ? mix(pt, region[0], pan.region[0]) : region[0];
  const oy = pan ? mix(pt, region[1], pan.region[1]) : region[1];
  const z = push ? geoZoom(prog(f, push.from, push.to - push.from, EASE_IN_OUT), 1, push.z) : 1;
  const fx = push ? (push.focus[0] - ox) * s : W / 2;
  const fy = push ? (push.focus[1] - oy) * s : H / 2;
  const mask =
    fadeL > 0 || fadeR > 0
      ? `linear-gradient(to right, transparent 0px, black ${Math.max(0.01, fadeL)}px, black ${W - Math.max(0.01, fadeR)}px, transparent ${W}px)`
      : undefined;
  return (
    <div style={{ position: "relative", width: W, height: H, overflow: "hidden", borderRadius: radius, WebkitMaskImage: mask, maskImage: mask }}>
      <div style={{ position: "absolute", inset: 0, transform: `scale(${z})`, transformOrigin: `${fx}px ${fy}px` }}>
        <div style={{ position: "absolute", left: -ox * s, top: -oy * s, width: srcW * s }}>
          <Img src={src} style={{ width: srcW * s, display: "block" }} />
          {children}
        </div>
      </div>
    </div>
  );
};

/**
 * A BLANK strip of the same capture (no glyphs, rows identical to the neighbouring ImgRegion), squeezed or stretched
 * horizontally to `w` px at the vertical scale `s`. Used as inner padding beside an ImgRegion whose source has a
 * neighbouring word right past the region edge, so the margin is the capture's own background (never a hardcoded
 * colour). Positioned absolutely by the caller, UNDER the ImgRegion (overlapping it by a couple of px: no seam).
 */
export const Patch: React.FC<{ src: string; srcW: number; srcH: number; s: number; patch: Box; w: number; style?: React.CSSProperties }> = ({
  src,
  srcW,
  srcH,
  s,
  patch,
  w,
  style,
}) => {
  const sx = w / (patch[2] - patch[0]);
  return (
    <div style={{ position: "absolute", width: w, height: (patch[3] - patch[1]) * s, overflow: "hidden", ...style }}>
      <Img
        src={src}
        style={{ position: "absolute", left: -patch[0] * sx, top: -patch[1] * s, width: srcW * sx, height: srcH * s, maxWidth: "none", display: "block" }}
      />
    </div>
  );
};

/** Scale a source-px box into scaled-source px (for children of ImgRegion). */
export const sb = (b: Box, s: number): Box => [b[0] * s, b[1] * s, b[2] * s, b[3] * s];

/** Browser chrome bar: three dots and a URL pill with the real domain. */
export const BrowserBar: React.FC<{ url: string }> = ({ url }) => (
  <div style={{ height: 42, display: "flex", alignItems: "center", gap: 7, padding: "0 12px", background: C.dark }}>
    {[C.red, C.gold, C.green].map((c) => (
      <div key={c} style={{ width: 10, height: 10, borderRadius: 5, background: c, flex: "none" }} />
    ))}
    <div
      style={{
        marginLeft: 8,
        flex: 1,
        height: 28,
        borderRadius: 14,
        background: C.paper,
        color: C.inkSoft,
        fontFamily: FONT,
        fontSize: 22,
        fontWeight: 500,
        display: "flex",
        alignItems: "center",
        padding: "0 12px",
        whiteSpace: "nowrap",
        overflow: "hidden",
      }}
    >
      {url}
    </div>
  </div>
);

/** A small label row on a dark card (for GitHub strips and docs). */
export const LabelBar: React.FC<{ children: React.ReactNode; h?: number }> = ({ children, h = 36 }) => (
  <div
    style={{
      height: h,
      display: "flex",
      alignItems: "center",
      padding: "0 14px",
      background: C.dark,
      color: C.muted,
      fontFamily: FONT,
      fontSize: 22,
      fontWeight: 600,
      whiteSpace: "nowrap",
      overflow: "hidden",
    }}
  >
    {children}
  </div>
);

/** The card shell: C.paper, 1.5 px C.line border, rounded, lifted. `edge` overrides the border colour (pulses). */
export const Shell: React.FC<{ w: number; edge?: string; radius?: number; style?: React.CSSProperties; children?: React.ReactNode }> = ({
  w,
  edge = C.line,
  radius = 14,
  style,
  children,
}) => (
  <div
    style={{
      width: w,
      borderRadius: radius,
      overflow: "hidden",
      background: C.paper,
      border: `1.5px solid ${edge}`,
      boxShadow: LIFT,
      ...style,
    }}
  >
    {children}
  </div>
);

/** "<domain> · captured Oct 16, 2026" (22 px, final frame). */
export const SrcPill: React.FC<{ domain: string; style?: React.CSSProperties }> = ({ domain, style }) => (
  <div
    style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 9,
      height: 32,
      padding: "0 14px",
      borderRadius: 16,
      background: C.paper,
      border: `1.5px solid ${C.line}`,
      color: C.muted,
      fontFamily: FONT,
      fontSize: 22,
      fontWeight: 500,
      whiteSpace: "nowrap",
      boxShadow: LIFT,
      ...style,
    }}
  >
    <div style={{ width: 8, height: 8, borderRadius: 4, background: C.accent, flex: "none" }} />
    {domain} · captured {CAPTURED}
  </div>
);

/** A diagram node: C.paper card, 1.5 px border (C.line, lit to C.accent by `lit`), radius 12, centred text. */
export const NodeCard: React.FC<{
  box: Box;
  text: string;
  at: number;
  out?: number;
  size?: number;
  weight?: number;
  lit?: number;
  glow?: number;
  dur?: number;
}> = ({ box, text, at, out, size = 28, weight = 650, lit = 0, glow = 0, dur = 10 }) => {
  const f = useCurrentFrame();
  const p = prog(f, at, dur);
  const o = out == null ? 1 : 1 - prog(f, out, 7, EASE_IN_OUT);
  if (p <= 0 || o <= 0) return null;
  const edge = interpolateColors(Math.min(1, Math.max(0, lit)), [0, 1], [C.line, C.accent]);
  return (
    <div
      style={{
        position: "absolute",
        left: box[0],
        top: box[1],
        width: box[2] - box[0],
        height: box[3] - box[1],
        boxSizing: "border-box",
        borderRadius: 12,
        background: C.paper,
        border: `${lit > 0.01 ? 2 : 1.5}px solid ${edge}`,
        boxShadow: `${LIFT}${glow > 0 ? `, 0 0 ${18 * glow}px ${4 * glow}px ${withAlpha(C.accent, 0.35 * glow)}` : ""}`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        textAlign: "center",
        padding: "0 16px",
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        lineHeight: 1.12,
        letterSpacing: "-0.01em",
        color: C.ink,
        opacity: p * o,
        transform: `translateY(${(1 - p) * 10}px)`,
      }}
    >
      {text}
    </div>
  );
};

/** Straight connector with an arrow head, drawn on from `at` over `dur`; `flow` (0..1) paints it accent from the start. */
export const Arrow: React.FC<{
  from: [number, number];
  to: [number, number];
  at: number;
  out?: number;
  dur?: number;
  flow?: number;
  color?: string;
}> = ({ from, to, at, out, dur = 8, flow = 0, color = C.muted }) => {
  const f = useCurrentFrame();
  const d = prog(f, at, dur);
  const o = out == null ? 1 : 1 - prog(f, out, 7, EASE_IN_OUT);
  if (d <= 0 || o <= 0) return null;
  const [x1, y1] = from;
  const [x2, y2] = to;
  const L = Math.hypot(x2 - x1, y2 - y1);
  const ux = (x2 - x1) / L;
  const uy = (y2 - y1) / L;
  const hx = x2 - ux * 11;
  const hy = y2 - uy * 11;
  const head = `M ${hx - uy * 7} ${hy + ux * 7} L ${x2} ${y2} L ${hx + uy * 7} ${hy - ux * 7}`;
  const fx = x1 + (x2 - x1) * flow;
  const fy = y1 + (y2 - y1) * flow;
  return (
    <svg style={{ position: "absolute", left: 0, top: 0, overflow: "visible", opacity: o }} width={1} height={1}>
      <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={color} strokeWidth={3} strokeLinecap="round" strokeDasharray={L} strokeDashoffset={L * (1 - d)} />
      {flow > 0 ? <line x1={x1} y1={y1} x2={fx} y2={fy} stroke={C.accent} strokeWidth={3.5} strokeLinecap="round" /> : null}
      <path d={head} fill="none" stroke={flow >= 1 ? C.accent : color} strokeWidth={3} strokeLinecap="round" strokeLinejoin="round" opacity={prog(f, at + dur - 3, 4)} />
    </svg>
  );
};

export { EASE_OUT };
