/**
 * Velour proof components (added 2026-10-16 for the "10 Claude Opus 5.5 Tips" edit; additive, nothing else changed).
 * Ported from the proof-first-video-editor skill's velour template: BrowserCard, SourcePill, Highlight, CodeChip.
 * Import from the package index like every other velour component.
 */
import React from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import { C, EASE_IN_OUT, geoZoom, prog, velour } from "./theme";
import { MediaFill, Reveal } from "./components";

const FONT = "'Inter', system-ui, -apple-system, 'Helvetica Neue', Arial, sans-serif";
const MONO = "ui-monospace, 'SF Mono', Menlo, Consolas, monospace";

/** Opacity of an element that enters at `delay` and (optionally) leaves at `out`. */
const inOut = (frame: number, delay: number, dur: number, out?: number, outDur = 8): number =>
  prog(frame, delay, dur) * (out === undefined ? 1 : 1 - prog(frame, out, outDur, EASE_IN_OUT));

/** Slow geometric push over the rest of the job (max a few %). */
const usePush = (delay: number, push: [number, number]): number => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  return geoZoom(prog(frame, delay, Math.max(1, durationInFrames - delay), EASE_IN_OUT), push[0], push[1]);
};

/**
 * A real screenshot in a browser card: a title bar with the three window dots and a URL bar showing the real domain,
 * then the image. `children` are drawn over the IMAGE AREA (same pixel box), e.g. a <Highlight/> on the number the
 * creator says: measure the target in a still of the render, not in the source image.
 */
export const BrowserCard: React.FC<{
  url: string;
  src: string;
  x: number;
  y: number;
  w: number;
  /** height of the image area (default 16:10 of w) */
  h?: number;
  delay?: number;
  out?: number;
  push?: [number, number];
  fit?: "cover" | "contain";
  objectPosition?: string;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}> = ({ url, src, x, y, w, h, delay = 0, out, push = [1, 1.02], fit = "cover", objectPosition = "50% 0%", style, children }) => {
  const zoom = usePush(delay, push);
  const imgH = h ?? Math.round((w * 10) / 16);
  const bar = 64;
  return (
    <Reveal delay={delay} out={out} style={{ position: "absolute", left: x, top: y, width: w }}>
      <div style={{ borderRadius: 22, overflow: "hidden", background: C.paper, boxShadow: velour.shadow.card, ...style }}>
        <div style={{ height: bar, display: "flex", alignItems: "center", gap: 12, padding: "0 22px", background: C.dark }}>
          {["#FF5F57", "#FEBC2E", "#28C840"].map((c) => (
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
        <div style={{ position: "relative", width: w, height: imgH, overflow: "hidden", background: C.white }}>
          <MediaFill src={src} kind="image" fit={fit} objectPosition={objectPosition} zoom={zoom} origin="50% 0%" />
          <div style={{ position: "absolute", inset: 0, transform: `scale(${zoom})`, transformOrigin: "50% 0%" }}>{children}</div>
        </div>
      </div>
    </Reveal>
  );
};

/** The source line under every proof: "<domain> · captured <Mon D, YYYY>". */
export const SourcePill: React.FC<{ domain: string; captured: string; delay?: number; out?: number; style?: React.CSSProperties }> = ({
  domain,
  captured,
  delay = 0,
  out,
  style,
}) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 12,
        padding: "10px 22px",
        borderRadius: 999,
        background: C.paper,
        color: C.muted,
        fontFamily: FONT,
        fontSize: 28,
        fontWeight: 500,
        boxShadow: velour.shadow.soft,
        opacity: inOut(frame, delay, 10, out),
        ...style,
      }}
    >
      <div style={{ width: 10, height: 10, borderRadius: 5, background: C.accent }} />
      {domain} · captured {captured}
    </div>
  );
};

/**
 * ONE highlight on one spoken target, drawn on at `delay` (2-4 frames before the word).
 * ring = an ellipse around it, box = a rounded rectangle, underline = a line under it, marker = a translucent swipe.
 * x/y/w/h = the TARGET's box in the parent's pixels; `pad` leaves a margin so the mark never touches the text.
 */
export const Highlight: React.FC<{
  x: number;
  y: number;
  w: number;
  h: number;
  delay?: number;
  dur?: number;
  kind?: "ring" | "box" | "underline" | "marker";
  color?: string;
  stroke?: number;
  pad?: number;
  out?: number;
}> = ({ x, y, w, h, delay = 0, dur = 10, kind = "box", color = C.accent, stroke = 6, pad = 10, out }) => {
  const frame = useCurrentFrame();
  const t = prog(frame, delay, dur);
  const o = out === undefined ? 1 : 1 - prog(frame, out, 8, EASE_IN_OUT);
  const L = x - pad;
  const T = y - pad;
  const W = w + 2 * pad;
  const H = h + 2 * pad;
  if (kind === "marker") {
    return (
      <div
        style={{
          position: "absolute",
          left: L,
          top: T,
          width: W * t,
          height: H,
          borderRadius: 8,
          background: color,
          opacity: 0.3 * o * (t > 0 ? 1 : 0),
        }}
      />
    );
  }
  const svgBox = { left: L - stroke, top: T - stroke, width: W + 2 * stroke, height: H + 2 * stroke };
  let shape: React.ReactNode;
  let len: number;
  if (kind === "ring") {
    const rx = W / 2 + stroke;
    const ry = H / 2 + stroke;
    len = Math.PI * (3 * (rx + ry) - Math.sqrt((3 * rx + ry) * (rx + 3 * ry)));
    shape = <ellipse cx={svgBox.width / 2} cy={svgBox.height / 2} rx={rx - stroke} ry={ry - stroke} />;
  } else if (kind === "underline") {
    len = W;
    shape = <line x1={stroke} y1={svgBox.height - stroke} x2={stroke + W} y2={svgBox.height - stroke} />;
  } else {
    len = 2 * (W + H);
    shape = <rect x={stroke} y={stroke} width={W} height={H} rx={12} ry={12} />;
  }
  return (
    <svg style={{ position: "absolute", overflow: "visible", opacity: o, ...svgBox }} width={svgBox.width} height={svgBox.height}>
      <g
        fill="none"
        stroke={color}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={len}
        strokeDashoffset={len * (1 - t)}
      >
        {shape}
      </g>
    </svg>
  );
};

/** Monospace text (a command, a file name) on a raised card. */
export const CodeChip: React.FC<{ text: string; delay?: number; size?: number; out?: number; style?: React.CSSProperties }> = ({
  text,
  delay = 0,
  size = 36,
  out,
  style,
}) => (
  <Reveal delay={delay} out={out} style={{ display: "inline-block", ...style }}>
    <div
      style={{
        fontFamily: MONO,
        fontSize: size,
        color: C.ink,
        background: C.paper,
        borderRadius: 14,
        padding: `${Math.round(size * 0.4)}px ${Math.round(size * 0.6)}px`,
        boxShadow: velour.shadow.soft,
        whiteSpace: "pre",
      }}
    >
      {text}
    </div>
  </Reveal>
);

/* ------------------------------------------------------------------------------------------------ tweets */

type Box = [number, number, number, number];

/**
 * A REAL tweet screenshot (tweetshot.mjs: <name>.png + <name>.json) presented as itself: a rounded dark card with a thin
 * border, a rise-in entrance, a slow push and highlights placed from the json's exact phrase boxes. Shared by every job
 * of the tips video so all tweets look identical.
 * - srcW/srcH: the PNG's pixel size (json "size"); boxes are in those pixels (json boxes.highlights[].box, boxes.time_views).
 * - x/y/w: the card's box on the stage; the PNG is scaled to width w (scale = w / srcW).
 * - view: [y0, y1] source rows visible (default the whole PNG); the card's height = (y1 - y0) * scale.
 * - scroll: move the visible window's top from view[0] to `toY` (source px) between frames `from` and `to` (eased),
 *   e.g. to bring the quoted line of a long post (-full.png) under a highlight.
 * - highlights: one per spoken target, `at` = 2-4 frames before the word; they ride with the image (scroll and push).
 * - pushTo: the slow push (<= 1.04) is centred on this source point (default: centre of the first highlight).
 */
export const TweetCard: React.FC<{
  src: string;
  srcW: number;
  srcH: number;
  x: number;
  y: number;
  w: number;
  view?: [number, number];
  scroll?: { from: number; to: number; toY: number };
  delay?: number;
  out?: number;
  push?: [number, number];
  pushTo?: [number, number];
  highlights?: { box: Box; at: number; kind?: "ring" | "box" | "underline" | "marker"; out?: number; pad?: number; stroke?: number }[];
  children?: React.ReactNode;
}> = ({ src, srcW, srcH, x, y, w, view, scroll, delay = 0, out, push = [1, 1.03], pushTo, highlights = [], children }) => {
  const frame = useCurrentFrame();
  const s = w / srcW;
  const [v0, v1] = view ?? [0, srcH];
  const viewH = Math.round((v1 - v0) * s);
  const top = scroll ? v0 + (scroll.toY - v0) * prog(frame, scroll.from, Math.max(1, scroll.to - scroll.from), EASE_IN_OUT) : v0;
  const zoom = usePush(delay, push);
  const first = highlights[0]?.box;
  const focus: [number, number] = pushTo ?? (first ? [(first[0] + first[2]) / 2, (first[1] + first[3]) / 2] : [srcW / 2, v0 + (v1 - v0) / 2]);
  const ox = focus[0] * s;
  const oy = (focus[1] - top) * s;
  return (
    <Reveal delay={delay} out={out} style={{ position: "absolute", left: x, top: y, width: w }}>
      <div
        style={{
          position: "relative",
          width: w,
          height: viewH,
          borderRadius: 26,
          overflow: "hidden",
          background: C.paperEdge,
          boxShadow: velour.shadow.card,
          outline: `2px solid ${C.line}`,
        }}
      >
        <div style={{ position: "absolute", inset: 0, transform: `scale(${zoom})`, transformOrigin: `${ox}px ${oy}px` }}>
          <div style={{ position: "absolute", left: 0, top: -top * s, width: w, height: srcH * s }}>
            <MediaFill src={src} kind="image" fit="cover" />
            {highlights.map((hl, i) => (
              <Highlight
                key={i}
                x={hl.box[0] * s}
                y={hl.box[1] * s}
                w={(hl.box[2] - hl.box[0]) * s}
                h={(hl.box[3] - hl.box[1]) * s}
                delay={hl.at}
                kind={hl.kind ?? "box"}
                stroke={hl.stroke ?? 4}
                pad={hl.pad ?? 3}
                out={hl.out}
              />
            ))}
            {children}
          </div>
        </div>
      </div>
    </Reveal>
  );
};

/**
 * Tip opener (added 2026-10-16 for the "10 Claude Opus 5.5 Tips" edit; opt-in, nothing else uses it).
 * The FIRST beat of every tip section: a "TIP N" kicker (accent, letter-spaced, with a rule) over the tip name (96 px,
 * ink, weight 800), stage pixels. Enters with a 12-frame rise + de-blur at `delay`. Optional `dockAt`: from that frame it
 * shrinks (geometric, 12 frames) to `dockScale` and slides to (x, dockY), so it can stay as a small section header while
 * the proof beats play under it. Leaves with an 8-frame fade at `out`. Every job uses this same component so the ten
 * openers look identical.
 */
export const TipOpener: React.FC<{
  n: number;
  name: string;
  delay?: number;
  out?: number;
  x?: number;
  y?: number;
  dockAt?: number;
  dockScale?: number;
  dockY?: number;
}> = ({ n, name, delay = 0, out, x = 80, y = 60, dockAt, dockScale = 0.5, dockY = 28 }) => {
  const frame = useCurrentFrame();
  const p = prog(frame, delay, 12);
  const o = inOut(frame, delay, 12, out, 8);
  const d = dockAt === undefined ? 0 : prog(frame, dockAt, 12, EASE_IN_OUT);
  const s = geoZoom(d, 1, dockScale);
  const top = y + (dockY - y) * d + (1 - p) * 24;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top,
        transform: `scale(${s})`,
        transformOrigin: "0 0",
        opacity: o,
        filter: `blur(${(1 - p) * 8}px)`,
        fontFamily: FONT,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 20 }}>
        <div style={{ width: 54, height: 5, background: C.accent, borderRadius: 4 }} />
        <div style={{ fontSize: 34, fontWeight: 750, letterSpacing: "0.18em", color: C.accent }}>{`TIP ${n}`}</div>
      </div>
      <div style={{ marginTop: 14, fontSize: 96, fontWeight: 800, letterSpacing: "-0.02em", lineHeight: 1.02, color: C.ink, whiteSpace: "nowrap" }}>
        {name}
      </div>
    </div>
  );
};
