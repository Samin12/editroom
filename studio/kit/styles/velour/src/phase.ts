/**
 * Timing and canvas phase for velour scenes.
 *
 * editroom studio mode: cutroom composes the final video. A section film is a full-frame opaque
 * render that carries its own dot-grid canvas, so that canvas must line up with cutroom's canvas
 * frame for frame: same colours, pitch, dot size and drift, with the drift's phase origin at
 * OUTPUT time 0. Every job registers its slot start in output ms (`slot0`, from job.json) before
 * the composition mounts (the job runner writes that call into the job's entry file), and the
 * canvas reads it through `useVideoConfig().id`.
 *
 * The numbers below are cutroom's defaults (engine/crates/gfx/src/lib.rs Theme + Gfx::canvas):
 * #121212 canvas, #2c2c2c dots of radius 1.6 px (1 px anti-aliased edge) on a 40 px grid, dots at
 * multiples of the pitch from the frame origin, drifting down-right at 2.5 px/s. A job.json can
 * carry `canvas` overrides when the edit's `style:` changes them; the runner registers those too.
 */

export type CanvasSpec = {
  /** canvas fill */
  bg: string;
  /** dot colour */
  dot: string;
  /** grid pitch, px at 1080p */
  pitch: number;
  /** dot radius, px at 1080p (the edge fades over the next 1 px) */
  radius: number;
  /** diagonal drift, px per second, down-right */
  driftPxPerSec: number;
};

/** cutroom's canvas, the single source of truth for every velour film. */
export const CUTROOM_CANVAS: CanvasSpec = { bg: "#121212", dot: "#2c2c2c", pitch: 40, radius: 1.6, driftPxPerSec: 2.5 };

/**
 * cutroom's default layout at 1920x1080 (engine/crates/core/src/spec.rs d_panel / d_camera and the
 * badge rule in render/src/frame.rs). Films draw the panel themselves; the camera tile and the
 * step badge are drawn by cutroom ON TOP of the film, so nothing important may sit in `keepClear`.
 */
export const LAYOUT = {
  panel: { x: 19.2, y: 19.2, w: 1473.75, h: 1041.6 },
  camera: { x: 1529.5, y: 229.6, w: 353.9, h: 620.7 },
  badge: { x: 1529.5, y: 70, w: 353.9, h: 138 },
  /** everything right of the panel: canvas only (dots), never content */
  keepClear: { x: 1500, y: 0, w: 420, h: 1080 },
} as const;

/**
 * Slot start (output ms) per composition id. The job runner registers the job being rendered; the
 * entries below keep the reference jobs (studio/kit/references) on the phase they were made in.
 */
export const JOB_START_MS: Record<string, number> = {
  "anim-mvagv60a": 0, // Claude + Honcho: an AI second brain that evolves with you
  "anim-mvagv66m": 321702, // Claude + Honcho: an AI second brain that evolves with you
  "anim-mvas6emb": 37121, // 10 Claude Opus 5.5 Tips (Easy to Advanced)
  "anim-mvas6eqq": 248520, // 10 Claude Opus 5.5 Tips (Easy to Advanced)
  "anim-mvas6evb": 406211, // 10 Claude Opus 5.5 Tips (Easy to Advanced)
  "anim-mvas6eyk": 527011, // 10 Claude Opus 5.5 Tips (Easy to Advanced)
};

const JOB_CANVAS: Record<string, Partial<CanvasSpec>> = {};

/** Called by the job's generated entry file before the composition registers. */
export const registerJob = (id: string, slot0Ms: number, canvas?: Partial<CanvasSpec> | null): void => {
  JOB_START_MS[id] = slot0Ms;
  if (canvas) JOB_CANVAS[id] = canvas;
};

/** The canvas a composition draws: cutroom's, with that job's overrides. */
export const canvasFor = (id: string): CanvasSpec => ({ ...CUTROOM_CANVAS, ...(JOB_CANVAS[id] || {}) });

/** Kept for old scenes: the default pitch and drift. */
export const DRIFT_PX_PER_SEC = CUTROOM_CANVAS.driftPxPerSec;
export const DOT_PITCH = CUTROOM_CANVAS.pitch;

/** Drift offset (px, 0..pitch) at OUTPUT time `ms`: dots sit at k*pitch + offset on both axes. */
export const driftAt = (ms: number, spec: CanvasSpec = CUTROOM_CANVAS): number => {
  const off = (ms / 1000) * spec.driftPxPerSec;
  return ((off % spec.pitch) + spec.pitch) % spec.pitch;
};
