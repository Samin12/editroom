/**
 * Velour components: the building blocks every velour scene is assembled from.
 * All motion is frame-pure (a function of useCurrentFrame()); nothing uses
 * CSS transitions, Math.random or Date.
 *
 * Coordinates are canvas pixels at 1920x1080. Media `src` values are paths
 * relative to the kit's public/ folder (for example "aiinf/vid/sbs-jp1.mp4").
 */
import React from "react";
import {
  AbsoluteFill,
  Img,
  Loop,
  OffthreadVideo,
  Sequence,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { C, EASE_IN_OUT, EASE_OUT, compact, geoZoom, mix, money, prog, velour } from "./theme";
import { DOT_PITCH, JOB_START_MS, canvasFor, driftAt } from "./phase";

const FONT = velour.font.display;
const resolveSrc = (src: string) => (/^(https?:|data:)/.test(src) ? src : staticFile(src));

/* ------------------------------------------------------------------ canvas */

/**
 * PANEL MODE (Samin, 2026-10-09): every animation plays inside the left screen panel of his layout,
 * exactly where the screen recording sits (Borumi slot x 0.01, y 0.0178, w 0.7676, h 0.9644 of
 * 1920x1080, radius 0.0178 h, drop shadow), over the drifting dot canvas; his camera tile and the
 * step badge sit on the right, outside the render. Scenes are still authored at 1920x1080: the
 * stage is scaled uniformly into the panel. Nested VelourCanvas elements render plain.
 */
export const PANEL = { x: 19.2, y: 19.2, w: 1473.75, h: 1041.6, r: 19.2 } as const;
export const PANEL_MODE = true;
export const PANEL_SCALE = PANEL.w / 1920;
const InPanel = React.createContext(false);

/**
 * A dot grid: dots at multiples of `pitch` from the frame origin, shifted `off` px down-right.
 * `solid`/`fade` are the gradient stops (px from the dot centre): fully dot-coloured inside
 * `solid`, fading linearly to clear at `fade`. cutroom's canvas uses radius r with a 1 px edge,
 * i.e. solid = r - 0.5, fade = r + 0.5.
 */
const DotLayer: React.FC<{ off: number; color?: string; pitch?: number; solid?: number; fade?: number }> = ({
  off,
  color = "rgb(70,70,70)",
  pitch = DOT_PITCH,
  solid = 2.1,
  fade = 2.9,
}) => (
  <div
    style={{
      position: "absolute",
      left: -pitch,
      top: -pitch,
      right: -pitch,
      bottom: -pitch,
      backgroundImage: `radial-gradient(circle, ${color} ${solid}px, rgba(0,0,0,0) ${fade}px)`,
      backgroundSize: `${pitch}px ${pitch}px`,
      backgroundPosition: `${-pitch / 2}px ${-pitch / 2}px`,
      transform: `translate(${off % pitch}px, ${off % pitch}px)`,
    }}
  />
);

/** The outer canvas, exactly cutroom's (see phase.ts CUTROOM_CANVAS), at output time `ms`. */
const CutroomDots: React.FC<{ id: string; ms: number }> = ({ id, ms }) => {
  const cv = canvasFor(id);
  return <DotLayer off={driftAt(ms, cv)} color={cv.dot} pitch={cv.pitch} solid={Math.max(0, cv.radius - 0.5)} fade={cv.radius + 0.5} />;
};

export const VelourCanvas: React.FC<{
  tone?: "cream" | "dark";
  transparent?: boolean;
  /** false = full frame (no panel), for pieces with no camera beside them, like a chapter pause */
  panel?: boolean;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}> = ({ tone = "cream", transparent = false, panel = PANEL_MODE, children, style }) => {
  // The Borumi canvas in dark mode: #121212 with a grey dot grid (40 px pitch, dots on multiples of
  // 40 px) drifting 2.5 px/s in phase with PROJECT time (JOB_START_MS), matching the moving
  // background clips under the screen recordings, so cuts never jump.
  const nested = React.useContext(InPanel);
  const frame = useCurrentFrame();
  const { fps, id } = useVideoConfig();
  // Output time of this frame: the slot's start (registered from job.json) + the frame's offset.
  const ms = (JOB_START_MS[id] ?? 0) + (frame / fps) * 1000;
  const off = driftAt(ms);
  const bg = canvasFor(id).bg;
  if (nested) {
    return <AbsoluteFill style={{ fontFamily: FONT, color: C.ink, ...style }}>{children}</AbsoluteFill>;
  }
  const glow = tone === "dark" ? "radial-gradient(ellipse 90% 75% at 50% 40%, rgba(226,119,78,0.12) 0%, rgba(18,18,18,0) 60%)" : "none";
  if (transparent || !panel) {
    return (
      <AbsoluteFill style={{ background: transparent ? "transparent" : bg, fontFamily: FONT, color: C.ink, overflow: "hidden", ...style }}>
        {!transparent ? (
          <>
            <CutroomDots id={id} ms={ms} />
            <div style={{ position: "absolute", inset: 0, backgroundImage: glow }} />
          </>
        ) : null}
        {children}
      </AbsoluteFill>
    );
  }
  const innerH = 1080 * PANEL_SCALE;
  return (
    <AbsoluteFill style={{ background: bg, fontFamily: FONT, color: C.ink, overflow: "hidden" }}>
      <CutroomDots id={id} ms={ms} />
      <div
        style={{
          position: "absolute",
          left: PANEL.x,
          top: PANEL.y,
          width: PANEL.w,
          height: PANEL.h,
          borderRadius: PANEL.r,
          overflow: "hidden",
          background: "#181716",
          boxShadow: "0 20px 20px rgba(0,0,0,0.39), inset 0 0 0 1px rgba(255,255,255,0.07)",
          ...style,
        }}
      >
        <DotLayer off={off * 0.8} color="rgb(42,42,40)" pitch={32} />
        <div style={{ position: "absolute", inset: 0, backgroundImage: glow }} />
        <div
          style={{
            position: "absolute",
            left: 0,
            top: (PANEL.h - innerH) / 2,
            width: 1920,
            height: 1080,
            transform: `scale(${PANEL_SCALE})`,
            transformOrigin: "0 0",
          }}
        >
          <InPanel.Provider value={true}>{children}</InPanel.Provider>
        </div>
      </div>
    </AbsoluteFill>
  );
};

/* ----------------------------------------------------------------- motion */

/**
 * Entrance (and optional exit) wrapper: rise + de-blur + settle scale.
 * `out` is the frame the exit starts. Position it with `style`.
 */
export const Reveal: React.FC<{
  delay?: number;
  dur?: number;
  y?: number;
  x?: number;
  scale?: number;
  blur?: number;
  out?: number;
  outDur?: number;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}> = ({ delay = 0, dur = 18, y = 36, x = 0, scale = 0.97, blur = 10, out, outDur = 12, style, children }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, dur);
  const o = out != null ? 1 - prog(f, out, outDur, EASE_IN_OUT) : 1;
  return (
    <div
      style={{
        opacity: p * o,
        transform: `translate(${(1 - p) * x}px, ${(1 - p) * y - (1 - o) * 14}px) scale(${mix(p, scale, 1) * mix(o, 0.98, 1)})`,
        filter: blur ? `blur(${(1 - p) * blur + (1 - o) * 6}px)` : undefined,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

/** A pop with a little overshoot, for chips, stamps and icons. */
export const Pop: React.FC<{
  delay?: number;
  dur?: number;
  from?: number;
  rotate?: number;
  out?: number;
  outDur?: number;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}> = ({ delay = 0, dur = 14, from = 0.6, rotate = 0, out, outDur = 10, style, children }) => {
  const f = useCurrentFrame();
  const t = prog(f, delay, dur, (x) => x);
  // back-out curve: overshoots ~8% then settles
  const s = 1.70158 * 1.2;
  const back = t === 0 ? 0 : 1 + (s + 1) * Math.pow(t - 1, 3) + s * Math.pow(t - 1, 2);
  const o = out != null ? 1 - prog(f, out, outDur, EASE_IN_OUT) : 1;
  return (
    <div
      style={{
        opacity: Math.min(1, t * 2) * o,
        transform: `scale(${mix(back, from, 1) * mix(o, 0.9, 1)}) rotate(${(1 - t) * rotate}deg)`,
        ...style,
      }}
    >
      {children}
    </div>
  );
};

/** Exit-only fade for a whole group (use when a beat clears the stage). */
export const FadeOut: React.FC<{ at: number; dur?: number; style?: React.CSSProperties; children?: React.ReactNode }> = ({
  at,
  dur = 12,
  style,
  children,
}) => {
  const f = useCurrentFrame();
  const o = 1 - prog(f, at, dur, EASE_IN_OUT);
  return <div style={{ position: "absolute", inset: 0, opacity: o, filter: `blur(${(1 - o) * 6}px)`, ...style }}>{children}</div>;
};

/* ------------------------------------------------------------------- type */

const clean = (w: string) => w.toLowerCase().replace(/[^a-z0-9$%]/g, "");

/**
 * Big headline that builds word by word. "\n" breaks a line. Words listed in
 * `accent` (case and punctuation ignored) are painted in the accent colour.
 */
export const Headline: React.FC<{
  text: string;
  delay?: number;
  stagger?: number;
  size?: number;
  weight?: number;
  color?: string;
  accent?: string[];
  accentColor?: string;
  lineHeight?: number;
  letterSpacing?: string;
  align?: "left" | "center" | "right";
  maxWidth?: number;
  out?: number;
  style?: React.CSSProperties;
}> = ({
  text,
  delay = 0,
  stagger = 3,
  size = 96,
  weight = 800,
  color,
  accent = [],
  accentColor = C.accent,
  lineHeight = 1.04,
  letterSpacing = "-0.035em",
  align = "left",
  maxWidth,
  out,
  style,
}) => {
  const f = useCurrentFrame();
  const accents = new Set(accent.map(clean));
  const lines = text.split("\n");
  let k = 0;
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  return (
    <div
      style={{
        fontFamily: FONT,
        fontSize: size,
        fontWeight: weight,
        lineHeight,
        letterSpacing,
        textAlign: align,
        maxWidth,
        color,
        opacity: o,
        ...style,
      }}
    >
      {lines.map((line, li) => (
        <div key={li}>
          {line.split(" ").map((w, wi) => {
            const p = prog(f, delay + k++ * stagger, 16);
            return (
              <span
                key={wi}
                style={{
                  display: "inline-block",
                  whiteSpace: "pre",
                  opacity: p,
                  transform: `translateY(${(1 - p) * 0.45}em)`,
                  filter: `blur(${(1 - p) * 8}px)`,
                  color: accents.has(clean(w)) ? accentColor : undefined,
                }}
              >
                {w}
                {wi < line.split(" ").length - 1 ? " " : ""}
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
};

/** Small uppercase label with a leading rule: "STEP 3 OF 8". */
export const Kicker: React.FC<{
  text: string;
  delay?: number;
  color?: string;
  size?: number;
  rule?: boolean;
  style?: React.CSSProperties;
}> = ({ text, delay = 0, color = C.accent, size = 26, rule = true, style }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 18);
  return (
    <div style={{ display: "flex", alignItems: "center", gap: size * 0.6, fontFamily: FONT, ...style }}>
      {rule ? <div style={{ width: size * 1.6 * p, height: Math.max(3, size * 0.12), background: color, borderRadius: 4 }} /> : null}
      <div
        style={{
          fontSize: size,
          fontWeight: 750,
          letterSpacing: "0.16em",
          textTransform: "uppercase",
          color,
          opacity: p,
          transform: `translateX(${(1 - p) * -16}px)`,
        }}
      >
        {text}
      </div>
    </div>
  );
};

/** Rounded pill / chip. */
export const Pill: React.FC<{
  children?: React.ReactNode;
  tone?: "accent" | "dark" | "light" | "mint" | "outline" | "glass";
  size?: number;
  icon?: React.ReactNode;
  style?: React.CSSProperties;
}> = ({ children, tone = "dark", size = 30, icon, style }) => {
  const tones: Record<string, React.CSSProperties> = {
    accent: { background: C.accent, color: C.white },
    dark: { background: C.dark, color: C.onDark },
    light: { background: C.paper, color: C.ink, boxShadow: velour.shadow.soft },
    mint: { background: C.mintSoft, color: C.mintDeep },
    outline: { background: "transparent", color: C.ink, border: `3px solid ${C.ink}` },
    glass: { background: "rgba(20,19,17,0.62)", color: C.white, backdropFilter: "blur(8px)" },
  };
  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: size * 0.4,
        padding: `${size * 0.42}px ${size * 0.8}px`,
        borderRadius: 999,
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 700,
        letterSpacing: "-0.01em",
        whiteSpace: "nowrap",
        ...tones[tone],
        ...style,
      }}
    >
      {icon}
      {children}
    </div>
  );
};

/* ----------------------------------------------------------------- numbers */

/** Animated number. format: compact (4.76M), money ($2,000), int (1,234), plain. */
export const CountUp: React.FC<{
  to: number;
  from?: number;
  delay?: number;
  dur?: number;
  format?: "compact" | "money" | "int" | "plain";
  digits?: number;
  prefix?: string;
  suffix?: string;
  style?: React.CSSProperties;
}> = ({ to, from = 0, delay = 0, dur = 36, format = "compact", digits = 2, prefix = "", suffix = "", style }) => {
  const f = useCurrentFrame();
  const v = mix(prog(f, delay, dur, EASE_OUT), from, to);
  const txt =
    format === "compact"
      ? compact(v, digits)
      : format === "money"
        ? money(v)
        : format === "int"
          ? Math.round(v).toLocaleString("en-US")
          : v.toFixed(digits);
  return (
    <span style={{ fontVariantNumeric: "tabular-nums", ...style }}>
      {prefix}
      {txt}
      {suffix}
    </span>
  );
};

/* ------------------------------------------------------------------- media */

export type MediaSpec = {
  src: string;
  kind?: "image" | "video";
  /** video: first source frame to play (30 fps frames) */
  startFrom?: number;
  /** video: loop the clip every N frames (use for clips shorter than the hold) */
  loopFrames?: number;
  playbackRate?: number;
  objectPosition?: string;
  fit?: "cover" | "contain";
  filter?: string;
};

/** The media itself (image or muted video), filling its parent. */
export const MediaFill: React.FC<MediaSpec & { startAt?: number; zoom?: number; origin?: string }> = ({
  src,
  kind,
  startFrom = 0,
  loopFrames,
  playbackRate = 1,
  objectPosition = "50% 50%",
  fit = "cover",
  filter,
  startAt = 0,
  zoom = 1,
  origin = "50% 50%",
}) => {
  const isVideo = kind ? kind === "video" : /\.(mp4|mov|webm)$/i.test(src);
  const mediaStyle: React.CSSProperties = {
    width: "100%",
    height: "100%",
    objectFit: fit,
    objectPosition,
    transform: `scale(${zoom})`,
    transformOrigin: origin,
    filter,
  };
  if (!isVideo) return <Img src={resolveSrc(src)} style={mediaStyle} />;
  const video = <OffthreadVideo src={resolveSrc(src)} muted startFrom={startFrom} playbackRate={playbackRate} style={mediaStyle} />;
  return (
    <Sequence from={startAt} layout="none">
      {loopFrames ? (
        <Loop durationInFrames={loopFrames} layout="none">
          {video}
        </Loop>
      ) : (
        video
      )}
    </Sequence>
  );
};

/**
 * Rounded media card with a soft shadow, entrance, optional slow push-in and
 * an optional label chip. The push is geometric and eased (never stepped).
 */
export const MediaFrame: React.FC<
  MediaSpec & {
    x: number;
    y: number;
    w: number;
    h: number;
    radius?: number;
    delay?: number;
    dur?: number;
    push?: [number, number];
    pushDur?: number;
    origin?: string;
    rotate?: number;
    shadow?: string;
    border?: string;
    label?: string;
    labelTone?: "dark" | "light" | "accent" | "glass";
    out?: number;
    outDur?: number;
    bg?: string;
    children?: React.ReactNode;
    style?: React.CSSProperties;
  }
> = (props) => {
  const {
    x, y, w, h, radius = 28, delay = 0, dur = 20, push = [1.0, 1.06], pushDur, origin = "50% 45%",
    rotate = 0, shadow = velour.shadow.card, border, label, labelTone = "glass", out, outDur = 12, bg = C.dark,
    children, style, ...media
  } = props;
  const f = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const p = prog(f, delay, dur);
  const o = out != null ? 1 - prog(f, out, outDur, EASE_IN_OUT) : 1;
  const zt = prog(f, delay, pushDur ?? Math.max(30, durationInFrames - delay), EASE_IN_OUT);
  const zoom = geoZoom(zt, push[0], push[1]);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: h,
        borderRadius: radius,
        overflow: "hidden",
        background: bg,
        boxShadow: shadow,
        border,
        opacity: p * o,
        transform: `translateY(${(1 - p) * 50}px) scale(${mix(p, 0.94, 1) * mix(o, 0.96, 1)}) rotate(${rotate}deg)`,
        filter: `blur(${(1 - p) * 10 + (1 - o) * 8}px)`,
        ...style,
      }}
    >
      <MediaFill {...media} startAt={delay} zoom={zoom} origin={origin} />
      {children}
      {label ? (
        <div style={{ position: "absolute", left: 22, bottom: 22 }}>
          <Pill tone={labelTone} size={24}>
            {label}
          </Pill>
        </div>
      ) : null}
    </div>
  );
};

const EyeIcon: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
    <path d="M1.5 12S5.5 4.5 12 4.5 22.5 12 22.5 12 18.5 19.5 12 19.5 1.5 12 1.5 12Z" stroke={color} strokeWidth={2.2} />
    <circle cx={12} cy={12} r={3.4} fill={color} />
  </svg>
);

const HeartIcon: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24">
    <path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 2.9 4.5 6.7 4.5c2.2 0 3.6 1.2 4.3 2.4.7-1.2 2.1-2.4 4.3-2.4 3.8 0 5.8 3.9 4.3 7.3C19.5 16.4 12 21 12 21Z" fill={color} />
  </svg>
);

/**
 * A 9:16 phone playing a vertical clip, with an optional Instagram-like
 * handle row and a view counter that counts up. Width is derived from h.
 */
export const Phone: React.FC<
  MediaSpec & {
    x: number;
    y: number;
    h: number;
    delay?: number;
    dur?: number;
    rotate?: number;
    handle?: string;
    /** Draw Instagram's blue verified badge after the handle. Only for accounts that really are verified. */
    verified?: boolean;
    views?: { to: number; from?: number; delay?: number; dur?: number };
    likes?: string;
    out?: number;
    outDur?: number;
    push?: [number, number];
    glow?: string;
    children?: React.ReactNode;
    style?: React.CSSProperties;
  }
> = (props) => {
  const { x, y, h, delay = 0, dur = 20, rotate = 0, handle, verified = false, views, likes, out, outDur = 12, push = [1, 1.04], glow, children, style, ...media } = props;
  const f = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const bezel = h * 0.016;
  const w = (h - bezel * 2) * (9 / 16) + bezel * 2;
  const p = prog(f, delay, dur);
  const o = out != null ? 1 - prog(f, out, outDur, EASE_IN_OUT) : 1;
  const zoom = geoZoom(prog(f, delay, Math.max(30, durationInFrames - delay), EASE_IN_OUT), push[0], push[1]);
  const fs = h * 0.026;
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: h,
        borderRadius: h * 0.07,
        background: "#0B0B0B",
        border: "2px solid rgba(255,255,255,0.16)",
        padding: bezel,
        boxSizing: "border-box",
        boxShadow: `${velour.shadow.card}${glow ? `, 0 0 120px -10px ${glow}` : ""}`,
        opacity: p * o,
        transform: `translateY(${(1 - p) * 70}px) scale(${mix(p, 0.92, 1) * mix(o, 0.95, 1)}) rotate(${rotate}deg)`,
        filter: `blur(${(1 - p) * 10 + (1 - o) * 8}px)`,
        ...style,
      }}
    >
      <div style={{ position: "relative", width: "100%", height: "100%", borderRadius: h * 0.056, overflow: "hidden", background: "#000" }}>
        <MediaFill {...media} startAt={delay} zoom={zoom} />
        <div style={{ position: "absolute", top: h * 0.012, left: "50%", width: w * 0.3, height: h * 0.026, marginLeft: -w * 0.15, borderRadius: 999, background: "#000" }} />
        {handle ? (
          <div
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              top: 0,
              padding: `${h * 0.05}px ${h * 0.03}px ${h * 0.05}px`,
              background: "linear-gradient(180deg, rgba(0,0,0,0.55), rgba(0,0,0,0))",
              display: "flex",
              alignItems: "center",
              gap: fs * 0.6,
              color: "#fff",
              fontFamily: FONT,
              fontSize: fs,
              fontWeight: 700,
            }}
          >
            <div style={{ width: fs * 1.6, height: fs * 1.6, borderRadius: 999, background: `linear-gradient(135deg, ${C.gold}, ${C.accent})`, border: "2px solid #fff" }} />
            {handle}
            {verified ? <div style={{ width: fs * 0.8, height: fs * 0.8, borderRadius: 999, background: "#3897F0" }} /> : null}
          </div>
        ) : null}
        {views || likes ? (
          <div
            style={{
              position: "absolute",
              left: 0,
              right: 0,
              bottom: 0,
              padding: `${h * 0.08}px ${h * 0.03}px ${h * 0.035}px`,
              background: "linear-gradient(0deg, rgba(0,0,0,0.7), rgba(0,0,0,0))",
              display: "flex",
              alignItems: "center",
              gap: fs * 1.2,
              color: "#fff",
              fontFamily: FONT,
              fontSize: fs * 1.25,
              fontWeight: 800,
            }}
          >
            {views ? (
              <div style={{ display: "flex", alignItems: "center", gap: fs * 0.45 }}>
                <EyeIcon size={fs * 1.3} color="#fff" />
                <CountUp to={views.to} from={views.from ?? 0} delay={views.delay ?? delay + 8} dur={views.dur ?? 45} />
              </div>
            ) : null}
            {likes ? (
              <div style={{ display: "flex", alignItems: "center", gap: fs * 0.45 }}>
                <HeartIcon size={fs * 1.2} color="#FF3B5C" />
                {likes}
              </div>
            ) : null}
          </div>
        ) : null}
        {children}
      </div>
    </div>
  );
};

/* ------------------------------------------------------------------- cards */

/** Deck-style numbered card ("01 Paid per view"), optionally with media on top. */
export const NumberCard: React.FC<{
  n: string;
  title: string;
  body?: string;
  tone?: "accent" | "dark" | "paper";
  x: number;
  y: number;
  w: number;
  h: number;
  delay?: number;
  out?: number;
  media?: MediaSpec;
  mediaRatio?: number;
  dimmed?: boolean;
  dimAt?: number;
  titleSize?: number;
  style?: React.CSSProperties;
}> = ({ n, title, body, tone = "accent", x, y, w, h, delay = 0, out, media, mediaRatio = 0.56, dimmed = false, dimAt = 0, titleSize = 46, style }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, 20);
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  const dim = dimmed ? prog(f, dimAt, 14, EASE_IN_OUT) : 0;
  const bg = tone === "accent" ? C.accent : tone === "dark" ? C.dark : C.paper;
  const fg = tone === "paper" ? C.ink : C.white;
  const pad = Math.round(w * 0.075);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: w,
        height: h,
        borderRadius: 30,
        overflow: "hidden",
        background: bg,
        color: fg,
        fontFamily: FONT,
        boxShadow: velour.shadow.card,
        display: "flex",
        flexDirection: "column",
        opacity: p * o * mix(dim, 1, 0.38),
        transform: `translateY(${(1 - p) * 60}px) scale(${mix(p, 0.94, 1) * mix(dim, 1, 0.96)})`,
        filter: `blur(${(1 - p) * 10}px) saturate(${mix(dim, 1, 0.4)})`,
        ...style,
      }}
    >
      {media ? (
        <div style={{ position: "relative", height: h * mediaRatio, overflow: "hidden" }}>
          <MediaFill {...media} startAt={delay} zoom={geoZoom(prog(f, delay, 150, EASE_IN_OUT), 1, 1.06)} />
        </div>
      ) : null}
      <div style={{ padding: pad, display: "flex", flexDirection: "column", gap: 12, flex: 1, justifyContent: media ? "flex-start" : "flex-end" }}>
        <div style={{ fontSize: 24, fontWeight: 800, letterSpacing: "0.14em", opacity: 0.8 }}>{n}</div>
        <div style={{ fontSize: titleSize, fontWeight: 800, letterSpacing: "-0.03em", lineHeight: 1.02 }}>{title}</div>
        {body ? <div style={{ fontSize: 26, fontWeight: 500, lineHeight: 1.3, opacity: 0.86 }}>{body}</div> : null}
      </div>
    </div>
  );
};

/** Check-list rows that tick in one by one. */
export const Checklist: React.FC<{
  items: { text: string; at: number }[];
  x: number;
  y: number;
  size?: number;
  gap?: number;
  tone?: "cream" | "dark";
  out?: number;
}> = ({ items, x, y, size = 48, gap = 26, tone = "cream", out }) => {
  const f = useCurrentFrame();
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  return (
    <div style={{ position: "absolute", left: x, top: y, display: "flex", flexDirection: "column", gap, fontFamily: FONT, opacity: o }}>
      {items.map((it, i) => {
        const p = prog(f, it.at, 16);
        const tick = prog(f, it.at + 6, 12);
        return (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: size * 0.45, opacity: p, transform: `translateX(${(1 - p) * -30}px)` }}>
            <svg width={size} height={size} viewBox="0 0 48 48">
              <circle cx={24} cy={24} r={22} fill={C.accent} />
              <path d="M14 24.5 21 31.5 34.5 17" fill="none" stroke="#fff" strokeWidth={5} strokeLinecap="round" strokeLinejoin="round" strokeDasharray={32} strokeDashoffset={32 * (1 - tick)} />
            </svg>
            <div style={{ fontSize: size * 0.92, fontWeight: 750, letterSpacing: "-0.02em", color: tone === "dark" ? C.onDark : C.ink }}>{it.text}</div>
          </div>
        );
      })}
    </div>
  );
};

/** A rotated rubber-stamp label ("100% AI"). */
export const Stamp: React.FC<{ text: string; x: number; y: number; delay?: number; size?: number; color?: string; rotate?: number; out?: number }> = ({
  text,
  x,
  y,
  delay = 0,
  size = 120,
  color = C.accent,
  rotate = -8,
  out,
}) => {
  const f = useCurrentFrame();
  const t = prog(f, delay, 10, (v) => v);
  const o = out != null ? 1 - prog(f, out, 10, EASE_IN_OUT) : 1;
  const scale = mix(prog(f, delay, 10, EASE_OUT), 2.2, 1);
  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        transform: `translate(-50%, -50%) rotate(${rotate}deg) scale(${scale})`,
        opacity: Math.min(1, t * 3) * o,
        padding: `${size * 0.14}px ${size * 0.34}px`,
        border: `${Math.max(6, size * 0.07)}px solid ${color}`,
        borderRadius: size * 0.16,
        color,
        fontFamily: FONT,
        fontSize: size,
        fontWeight: 900,
        letterSpacing: "0.02em",
        lineHeight: 1,
        whiteSpace: "nowrap",
        background: "rgba(255,255,255,0.04)",
      }}
    >
      {text}
    </div>
  );
};

/** A clean arrow that draws on from (x1,y1) to (x2,y2), bowed by `bend`. */
export const ArrowTo: React.FC<{
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  bend?: number;
  delay?: number;
  dur?: number;
  color?: string;
  width?: number;
  out?: number;
}> = ({ x1, y1, x2, y2, bend = 0.25, delay = 0, dur = 16, color = C.accent, width = 6, out }) => {
  const f = useCurrentFrame();
  const p = prog(f, delay, dur, EASE_IN_OUT);
  const o = out != null ? 1 - prog(f, out, 10, EASE_IN_OUT) : 1;
  const mx = (x1 + x2) / 2 - (y2 - y1) * bend;
  const my = (y1 + y2) / 2 + (x2 - x1) * bend;
  const len = Math.hypot(x2 - x1, y2 - y1) * (1 + Math.abs(bend));
  const ang = Math.atan2(y2 - my, x2 - mx);
  const hs = width * 3.4;
  const head = `M ${x2 - hs * Math.cos(ang - 0.5)} ${y2 - hs * Math.sin(ang - 0.5)} L ${x2} ${y2} L ${x2 - hs * Math.cos(ang + 0.5)} ${y2 - hs * Math.sin(ang + 0.5)}`;
  return (
    <svg style={{ position: "absolute", left: 0, top: 0, width: 1920, height: 1080, overflow: "visible", opacity: o }} viewBox="0 0 1920 1080">
      <path d={`M ${x1} ${y1} Q ${mx} ${my} ${x2} ${y2}`} fill="none" stroke={color} strokeWidth={width} strokeLinecap="round" strokeDasharray={len} strokeDashoffset={len * (1 - p)} />
      <path d={head} fill="none" stroke={color} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" opacity={p > 0.92 ? 1 : 0} />
    </svg>
  );
};

/** A chat bubble that types its text, with a small Claude-style avatar. */
export const ChatBubble: React.FC<{
  text: string;
  x: number;
  y: number;
  w: number;
  delay?: number;
  cps?: number;
  size?: number;
  from?: "user" | "claude";
  out?: number;
}> = ({ text, x, y, w, delay = 0, cps = 38, size = 34, from = "user", out }) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const p = prog(f, delay, 14);
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  const chars = Math.max(0, Math.floor(((f - delay - 6) / fps) * cps));
  const shown = text.slice(0, chars);
  const caret = chars < text.length && Math.floor(f / 8) % 2 === 0;
  const isUser = from === "user";
  return (
    <div style={{ position: "absolute", left: x, top: y, width: w, display: "flex", gap: 18, alignItems: "flex-start", opacity: p * o, transform: `translateY(${(1 - p) * 30}px)`, fontFamily: FONT }}>
      <div style={{ width: size * 1.6, height: size * 1.6, flex: "none", borderRadius: 999, background: isUser ? C.ink : C.accent, display: "flex", alignItems: "center", justifyContent: "center", color: "#fff", fontWeight: 900, fontSize: size * 0.9 }}>
        {isUser ? "S" : "✳"}
      </div>
      <div style={{ background: isUser ? C.paper : C.darkSoft, border: isUser ? "none" : `2px solid ${C.accent}`, color: C.ink, borderRadius: 26, padding: `${size * 0.6}px ${size * 0.8}px`, fontSize: size, fontWeight: 550, lineHeight: 1.32, boxShadow: velour.shadow.soft, flex: 1 }}>
        {shown}
        <span style={{ opacity: caret ? 1 : 0, color: C.accent }}>|</span>
      </div>
    </div>
  );
};

/* -------------------------------------------------------------- chapters */

/** Short labels for the tracker, and the full chapter titles. Steps are 1-based. */
export const STEP_SHORT = ["Money", "Connect", "Design", "First video", "Go viral", "Product", "Get paid", "Autopilot"];
export const STEP_TITLES = [
  "How AI influencers make money",
  "Connect Claude to Higgsfield",
  "Design your character",
  "Make your first video",
  "Recreate viral videos",
  "Add product placements",
  "Get paid per view",
  "Put it on autopilot",
];

const CheckGlyph: React.FC<{ size: number; color: string }> = ({ size, color }) => (
  <svg width={size} height={size} viewBox="0 0 24 24">
    <path d="M5 12.5 10 17.5 19 7.5" fill="none" stroke={color} strokeWidth={3.2} strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/**
 * The linear progress tracker: one node per step on a line. Steps before
 * `current` are done (filled, checked), `current` is the terracotta node with a
 * pulse, later steps are hollow. The fill sweeps from step `from` to `current`
 * between `delay` and `delay + dur`. current = 0 shows the bare roadmap.
 */
export const StepTracker: React.FC<{
  current: number;
  from?: number;
  x: number;
  y: number;
  width: number;
  delay?: number;
  dur?: number;
  tone?: "cream" | "dark";
  labels?: boolean;
  steps?: string[];
  node?: number;
  revealStagger?: number;
  out?: number;
  /** opt-in (default true = unchanged): false draws a small dot instead of the step number in upcoming/current nodes */
  numbers?: boolean;
}> = ({ current, from, x, y, width, delay = 0, dur = 24, tone = "cream", labels = true, steps = STEP_SHORT, node = 54, revealStagger = 3, out, numbers = true }) => {
  const f = useCurrentFrame();
  const n = steps.length;
  const gap = width / (n - 1);
  const start = from ?? Math.max(0, current - 1);
  const fp = prog(f, delay + 10, dur, EASE_IN_OUT);
  const fillPos = mix(fp, Math.max(0, start - 1), Math.max(0, current - 1)); // in node units
  const track = C.line;
  const done = C.ink;
  const doneGlyph = C.bg;
  const hollow = C.paper;
  const o = out != null ? 1 - prog(f, out, 12, EASE_IN_OUT) : 1;
  const trackP = prog(f, delay, 20);
  return (
    <div style={{ position: "absolute", left: x, top: y, width, height: node, fontFamily: FONT, opacity: o }}>
      <div style={{ position: "absolute", left: 0, top: node / 2 - 4, height: 8, width: width * trackP, borderRadius: 8, background: track }} />
      {current > 0 ? (
        <div style={{ position: "absolute", left: 0, top: node / 2 - 4, height: 8, width: gap * fillPos * trackP, borderRadius: 8, background: C.accent }} />
      ) : null}
      {steps.map((label, i) => {
        const step = i + 1;
        const rp = prog(f, delay + i * revealStagger, 16);
        const reached = fillPos >= i - 0.02 && current > 0;
        const isCurrent = step === current && fp > 0.85;
        const isDone = step < current && reached;
        const pulse = isCurrent ? (f % 45) / 45 : 0;
        const bg = isCurrent ? C.accent : isDone ? done : hollow;
        const fg = isCurrent ? C.white : isDone ? doneGlyph : tone === "dark" ? C.muted : C.faint;
        return (
          <div key={i} style={{ position: "absolute", left: i * gap - node / 2, top: 0, width: node, height: node, opacity: rp, transform: `scale(${mix(rp, 0.5, 1) * (isCurrent ? 1.18 : 1)})` }}>
            {isCurrent ? (
              <div style={{ position: "absolute", inset: 0, borderRadius: 999, border: `4px solid ${C.accent}`, transform: `scale(${1 + pulse * 0.7})`, opacity: 0.55 * (1 - pulse) }} />
            ) : null}
            <div
              style={{
                position: "absolute",
                inset: 0,
                borderRadius: 999,
                background: bg,
                border: isCurrent || isDone ? "none" : `4px solid ${track}`,
                boxSizing: "border-box",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: fg,
                fontSize: node * 0.42,
                fontWeight: 800,
                boxShadow: isCurrent ? `0 10px 30px -8px ${C.accent}` : undefined,
              }}
            >
              {isDone ? <CheckGlyph size={node * 0.56} color={doneGlyph} /> : numbers ? step : <div style={{ width: node * 0.22, height: node * 0.22, borderRadius: 99, background: fg }} />}
            </div>
            {labels ? (
              <div
                style={{
                  position: "absolute",
                  top: node + 16,
                  left: node / 2 - gap * 0.48,
                  width: gap * 0.96,
                  textAlign: "center",
                  fontSize: node * 0.4,
                  lineHeight: 1.15,
                  fontWeight: isCurrent ? 800 : 600,
                  color: isCurrent ? (tone === "dark" ? C.onDark : C.ink) : isDone ? C.muted : tone === "dark" ? C.muted : C.faint,
                }}
              >
                {label}
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
};

/**
 * Full-frame chapter slate for behind-mode placement (the camera is pinned
 * bottom-right, so nothing essential sits at x >= 1488, y >= 772).
 */
export const ChapterSlate: React.FC<{
  step: number;
  title?: string;
  subtitle?: string;
  media?: MediaSpec;
  tone?: "cream" | "dark";
  total?: number;
}> = ({ step, title, subtitle, media, tone = "cream", total = 8 }) => {
  const f = useCurrentFrame();
  const t = title ?? STEP_TITLES[step - 1];
  const wipe = prog(f, 0, 16, EASE_IN_OUT);
  const ghost = prog(f, 4, 30);
  return (
    <VelourCanvas tone={tone}>
      <div
        style={{
          position: "absolute",
          left: 60,
          top: 120,
          fontSize: 560,
          fontWeight: 900,
          letterSpacing: "-0.06em",
          lineHeight: 1,
          color: C.accentSoft,
          opacity: ghost,
          transform: `translateX(${(1 - ghost) * -40}px)`,
        }}
      >
        {String(step).padStart(2, "0")}
      </div>
      <div style={{ position: "absolute", left: 0, top: 0, bottom: 0, width: 1920 * (1 - wipe), background: C.accent }} />
      <Kicker text={`Step ${step} of ${total}`} delay={8} size={30} style={{ position: "absolute", left: 132, top: 300 }} />
      <Headline text={t} delay={12} size={t.length > 26 ? 104 : 124} maxWidth={media ? 1000 : 1300} color={tone === "dark" ? C.onDark : C.ink} style={{ position: "absolute", left: 128, top: 356 }} />
      {subtitle ? (
        <Reveal delay={26} style={{ position: "absolute", left: 132, top: 640, fontSize: 38, fontWeight: 500, color: C.muted, maxWidth: 980 }}>
          {subtitle}
        </Reveal>
      ) : null}
      {media ? <MediaFrame {...media} x={1210} y={150} w={560} h={540} radius={34} delay={14} push={[1.02, 1.1]} /> : null}
      <StepTracker current={step} x={150} y={828} width={1180} delay={16} tone={tone} node={50} />
    </VelourCanvas>
  );
};

/**
 * Compact persistent chapter badge (rendered as a transparent still and placed
 * top-right above the camera, 354x150 at 1080p): "STEP 3 / 8", the title and
 * an eight-segment progress bar. Size it with the composition.
 */
export const MiniTracker: React.FC<{ step: number; title?: string; total?: number; kicker?: string }> = ({ step, title, total = 8, kicker }) => {
  const { width, height } = useVideoConfig();
  const s = height / 150; // authored at 354x150
  const t = title ?? STEP_TITLES[step - 1];
  return (
    <AbsoluteFill style={{ background: "transparent", fontFamily: FONT }}>
      <div
        style={{
          position: "absolute",
          inset: 6 * s,
          borderRadius: 22 * s,
          background: "rgba(38,36,33,0.94)",
          border: `${1.5 * s}px solid rgba(255,255,255,0.16)`,
          boxShadow: `0 ${10 * s}px ${26 * s}px -${10 * s}px rgba(0,0,0,0.6)`,
          padding: `${16 * s}px ${20 * s}px`,
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          boxSizing: "border-box",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 10 * s }}>
          <div style={{ width: 10 * s, height: 10 * s, borderRadius: 99, background: C.accent }} />
          <div style={{ fontSize: 17 * s, fontWeight: 800, letterSpacing: "0.16em", color: C.accent }}>
            {kicker ?? `STEP ${step} / ${total}`}
          </div>
        </div>
        <div style={{ fontSize: (t.length > 24 ? 23 : 26) * s, fontWeight: 800, letterSpacing: "-0.02em", lineHeight: 1.08, color: C.onDark }}>{t}</div>
        <div style={{ display: "flex", gap: 6 * s }}>
          {Array.from({ length: total }, (_, i) => (
            <div
              key={i}
              style={{
                flex: 1,
                height: 7 * s,
                borderRadius: 9,
                background: i + 1 < step ? C.onDark : i + 1 === step ? C.accent : "rgba(255,255,255,0.18)",
              }}
            />
          ))}
        </div>
      </div>
      <div style={{ width, height }} />
    </AbsoluteFill>
  );
};
