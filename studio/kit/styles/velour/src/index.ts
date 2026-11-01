/**
 * Velour style package: public API.
 * From a job scene, import via: `import { ... } from "../../../styles/velour/src";`
 */
export { velour, C, EASE_OUT, EASE_IN_OUT, EASE_IN, prog, mix, geoZoom, compact, money, type VelourTheme } from "./theme";
export {
  VelourCanvas,
  PANEL,
  PANEL_MODE,
  PANEL_SCALE,
  Reveal,
  Pop,
  FadeOut,
  Headline,
  Kicker,
  Pill,
  CountUp,
  MediaFill,
  MediaFrame,
  Phone,
  NumberCard,
  Checklist,
  Stamp,
  ArrowTo,
  ChatBubble,
  StepTracker,
  ChapterSlate,
  MiniTracker,
  STEP_SHORT,
  STEP_TITLES,
  type MediaSpec,
} from "./components";

// Proof components (BrowserCard, SourcePill, Highlight, CodeChip): additive, 2026-10-16.
export { BrowserCard, SourcePill, Highlight, CodeChip, TweetCard, TipOpener } from "./proof";

// Course roadmap for the YouTube course video (M-ROADMAP): additive, 2026-10-28.
export { CourseRoadmap, COURSE_STAGES } from "./course";

// editroom studio mode: canvas phase, cutroom's canvas and layout (what a film must line up with).
export { CUTROOM_CANVAS, LAYOUT, JOB_START_MS, registerJob, canvasFor, driftAt, type CanvasSpec } from "./phase";
