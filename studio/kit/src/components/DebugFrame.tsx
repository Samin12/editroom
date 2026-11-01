import React from "react";
import { AbsoluteFill, Img, getInputProps, staticFile } from "remotion";

/**
 * A real footage frame rendered UNDER a frame-aware overlay so stills show the
 * annotations composited over what is actually on screen. This is the
 * verification layer for frame-aware (--frames) jobs: point `src` at a frame
 * that frames.mjs wrote from Borumi's inspect of the composite
 * (e.g. "frames/<jobId>/full/t0012.50.png"), render a still at the matching
 * frame number with `job.mjs still`, and check the drawing lands on its target.
 *
 * It renders NOTHING in the final render (`job.mjs render` passes the
 * { final: true } input prop), so it can safely stay in the scene: the
 * delivered clip keeps full transparency where the footage shows through.
 */
export const DebugFrame: React.FC<{ src: string; opacity?: number }> = ({ src, opacity = 1 }) => {
  const { final } = getInputProps() as { final?: boolean };
  if (final) return null;
  return (
    <AbsoluteFill style={{ opacity }}>
      <Img src={staticFile(src)} style={{ width: "100%", height: "100%" }} />
    </AbsoluteFill>
  );
};
