// The Scene.tsx a new job starts from: velour, cutroom's canvas, the slot's clock and asset helpers.
export function scaffoldScene(job) {
  const front = job.mode === "front";
  const proof = job.proofAs ? `const P = (file: string) => staticFile(\`${job.proofAs}/\${file}\`);` : "// (no --proof-dir given: put proof crops in assets/ and use A())";
  return `import React from "react";
import { staticFile, useCurrentFrame } from "remotion";
import {
  C,
  EASE_IN,
  EASE_IN_OUT,
  EASE_OUT,
  LAYOUT,
  Reveal,
  VelourCanvas,
  mix,
  prog,
} from "../../../styles/velour/src";

// ${job.id}: ${job.title} (${front ? "DEMO OVERLAY: transparent, over his screen recording" : "SECTION FILM: full frame, opaque, cutroom canvas + panel"}).
// Slot: output ${Math.round(job.slot0)} to ${Math.round(job.slotEnd)} ms, ${job.frames} frames (f0 to f${job.frames - 1}) at 30 fps.
// Frame of an output time: at(ms) = round((ms - SLOT0) * 30 / 1000). words.json has every word's frame already.
// cutroom draws his camera tile and the step badge ON TOP of this render: keep everything inside
// the panel (x < ${Math.round(1500)}), never in LAYOUT.keepClear.
export const SLOT0 = ${job.slot0};
export const FRAMES = ${job.frames};
export const at = (ms: number) => Math.round(((ms - SLOT0) * 30) / 1000);

/** Files in this job's assets/ folder. */
const A = (file: string) => staticFile(\`jobs/${job.id}/\${file}\`);
/** Proof captures from the edit's proof folder. */
${proof}

const Scene: React.FC = () => {
  const frame = useCurrentFrame();
  return (
    <VelourCanvas tone="dark"${front ? " transparent" : ""}>
      <Reveal delay={0} style={{ position: "absolute", left: 120, top: 120, fontSize: 64, fontWeight: 800, color: C.onDark }}>
        ${JSON.stringify(job.title)}
      </Reveal>
    </VelourCanvas>
  );
};

export default Scene;
`;
}
