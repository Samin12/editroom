/**
 * Local copies of velour's TweetCard and BrowserCard (styles/velour/src/proof.tsx), same look, with:
 * - a 12-frame entrance (Reveal dur 12) and an `outDur` exit passed through to Reveal;
 * - TweetCard: a per-card push WINDOW (pushFrom..pushTo, scale <= 1.04) and a focus that can be re-targeted over time
 *   (`focus` keyframes in SOURCE px, eased between them; each move >= 18 frames), so the push never stacks scale;
 * - TweetCard: x / y / w may change per frame (the Lingxi card glides from its opener rect to its standard rect);
 * - BrowserCard: no internal push (the stack wrapper carries it).
 */
import React from "react";
import { useCurrentFrame } from "remotion";
import { C, EASE_IN_OUT, Highlight, MediaFill, Reveal, geoZoom, prog, velour } from "../../../styles/velour/src";

const FONT = "'Inter', system-ui, -apple-system, 'Helvetica Neue', Arial, sans-serif";

export type Box = [number, number, number, number];
export type FocusKey = { at: number; dur?: number; p: [number, number] };

/** Eased focus point from a list of keyframes: each key starts easing from the previous point at `at` over `dur`. */
export const focusAt = (frame: number, keys: FocusKey[]): [number, number] => {
  let cur = keys[0].p;
  for (let i = 1; i < keys.length; i++) {
    const k = keys[i];
    const t = prog(frame, k.at, k.dur ?? 24, EASE_IN_OUT);
    cur = [cur[0] + (k.p[0] - cur[0]) * t, cur[1] + (k.p[1] - cur[1]) * t];
  }
  return cur;
};

export const LocalTweetCard: React.FC<{
  src: string;
  srcW: number;
  srcH: number;
  x: number;
  y: number;
  w: number;
  view: [number, number];
  scroll?: { from: number; to: number; toY: number };
  delay: number;
  out?: number;
  outDur?: number;
  push: { from: number; to: number; s1: number };
  focus: FocusKey[];
  highlights?: { box: Box; at: number; kind?: "ring" | "box" | "underline" | "marker"; out?: number; pad?: number; stroke?: number }[];
}> = ({ src, srcW, srcH, x, y, w, view, scroll, delay, out, outDur = 12, push, focus, highlights = [] }) => {
  const frame = useCurrentFrame();
  const s = w / srcW;
  const [v0, v1] = view;
  const viewH = Math.round((v1 - v0) * s);
  const top = scroll ? v0 + (scroll.toY - v0) * prog(frame, scroll.from, Math.max(1, scroll.to - scroll.from), EASE_IN_OUT) : v0;
  const zoom = geoZoom(prog(frame, push.from, Math.max(1, push.to - push.from), EASE_IN_OUT), 1, Math.min(1.04, push.s1));
  const fp = focusAt(frame, focus);
  const ox = fp[0] * s;
  const oy = (fp[1] - top) * s;
  return (
    <Reveal delay={delay} dur={12} out={out} outDur={outDur} style={{ position: "absolute", left: x, top: y, width: w }}>
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
          </div>
        </div>
      </div>
    </Reveal>
  );
};

export const LocalBrowserCard: React.FC<{
  url: string;
  src: string;
  x: number;
  y: number;
  w: number;
  h: number;
  delay: number;
  out?: number;
  outDur?: number;
  fit?: "cover" | "contain";
  objectPosition?: string;
  children?: React.ReactNode;
}> = ({ url, src, x, y, w, h, delay, out, outDur = 12, fit = "cover", objectPosition = "50% 0%", children }) => {
  const bar = 64;
  return (
    <Reveal delay={delay} dur={12} out={out} outDur={outDur} style={{ position: "absolute", left: x, top: y, width: w }}>
      <div style={{ borderRadius: 22, overflow: "hidden", background: C.paper, boxShadow: velour.shadow.card }}>
        <div style={{ height: bar, display: "flex", alignItems: "center", gap: 12, padding: "0 22px", background: C.dark }}>
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
        <div style={{ position: "relative", width: w, height: h, overflow: "hidden", background: C.paperEdge }}>
          <MediaFill src={src} kind="image" fit={fit} objectPosition={objectPosition} />
          <div style={{ position: "absolute", inset: 0 }}>{children}</div>
        </div>
      </div>
    </Reveal>
  );
};
