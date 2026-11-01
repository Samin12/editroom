/**
 * Velour style theme: the engine tokens re-skinned with the AI Influencer
 * deck palette (warm cream paper, ink, terracotta accent, charcoal cards,
 * Gaston's mint). Import `velour` INSTEAD of the engine `tokens` in velour
 * scenes so every colour lands on-brand.
 */
import { Easing, interpolate } from "remotion";
import { tokens } from "../../../src/theme/tokens";

export const velour = {
  ...tokens,
  color: {
    ...tokens.color,
    // DARK MODE (Samin, 2026-10-09): the whole video sits on the Borumi dark dot-grid canvas.
    // Names stay semantic: ink = primary text (light), paper = raised card surface (dark),
    // dark = a second, slightly lifted card tone. Use them by role, never by literal shade.
    bg: "#121212", // canvas (matches the Borumi background image)
    paper: "#1E1D1B", // raised card / chip surface
    paperEdge: "#0C0C0B",
    line: "#34322E", // hairlines, empty track
    ink: "#F4F1EC", // primary text
    inkSoft: "#D9D4CC",
    muted: "#A39C91", // secondary text
    faint: "#6E685F", // tertiary text, upcoming steps
    accent: "#E2774E", // terracotta, lifted for dark backgrounds
    accentDeep: "#C9643F",
    accentSoft: "#3A241A", // soft accent tint (ghost numbers, fills)
    dark: "#2A2825", // second card tone, lighter than the canvas
    darkSoft: "#33302B",
    darkLine: "#45413A",
    onDark: "#F6F1E9", // text on dark cards
    mint: "#7FC9AE", // Gaston's suit
    mintDeep: "#9BDCC3",
    mintSoft: "#1F3A31",
    gold: "#E0B04F",
    green: "#46B482",
    red: "#E5564A",
    white: "#FFFFFF", // text on the accent
  },
  font: {
    ...tokens.font,
    display: "'Inter', system-ui, -apple-system, sans-serif",
  },
  shadow: {
    ...tokens.shadow,
    card: "0 40px 80px -30px rgba(0,0,0,0.85), 0 12px 28px -12px rgba(0,0,0,0.6), 0 0 0 1px rgba(255,255,255,0.08)",
    soft: "0 18px 40px -18px rgba(0,0,0,0.7), 0 0 0 1px rgba(255,255,255,0.07)",
    onDark: "0 40px 90px -30px rgba(0,0,0,0.8), 0 0 0 1px rgba(255,255,255,0.06)",
  },
  /** The pinned camera rectangle in behind mode (canvas ratios). Keep content out of it. */
  cameraKeepOut: { x: 0.775, y: 0.715 },
} as const;

export type VelourTheme = typeof velour;
export const C = velour.color;

/** House easing: a confident ease-out (fast start, long settle). */
export const EASE_OUT = Easing.bezier(0.16, 1, 0.3, 1);
export const EASE_IN_OUT = Easing.bezier(0.65, 0, 0.35, 1);
export const EASE_IN = Easing.bezier(0.7, 0, 0.84, 0);

/** Clamped 0..1 progress of `frame` over [start, start+dur] with an easing. */
export const prog = (frame: number, start: number, dur: number, easing: (t: number) => number = EASE_OUT): number =>
  interpolate(frame, [start, start + Math.max(1, dur)], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing,
  });

/** Linear map of a 0..1 progress onto [a, b]. */
export const mix = (t: number, a: number, b: number): number => a + (b - a) * t;

/** Geometric zoom: z0 * (z1/z0)^t, the only zoom that reads as constant speed. */
export const geoZoom = (t: number, z0: number, z1: number): number => z0 * Math.pow(z1 / z0, t);

/** Compact number formatting: 4760000 -> "4.76M", 23000 -> "23K". */
export const compact = (n: number, digits = 2): string => {
  const a = Math.abs(n);
  if (a >= 1e9) return `${trim((n / 1e9).toFixed(digits))}B`;
  if (a >= 1e6) return `${trim((n / 1e6).toFixed(digits))}M`;
  if (a >= 1e4) return `${trim((n / 1e3).toFixed(0))}K`;
  if (a >= 1e3) return `${trim((n / 1e3).toFixed(1))}K`;
  return `${Math.round(n)}`;
};
const trim = (s: string) => s.replace(/\.0+$/, "").replace(/(\.\d*?)0+$/, "$1");

/** "$2,000" style money formatting. */
export const money = (n: number): string => `$${Math.round(n).toLocaleString("en-US")}`;
