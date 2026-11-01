import "./theme/fonts"; // side-effect: load the inlined Excalifont + Inter
import React from "react";
import { Composition } from "remotion";
import { jobs } from "./jobs/manifest";

/**
 * Every registered composition is an animation job scaffolded by the borumi
 * plugin's job.mjs (see src/jobs/manifest.ts). Width and height come from the
 * Borumi project canvas, fps is always 30 (the kit's SEC() assumes it and
 * Borumi conforms the imported clip), and durationInFrames comes from the
 * selected range. They are fixed per job and must not be changed by hand.
 */
export const RemotionRoot: React.FC = () => (
  <>
    {jobs.map((j) => (
      <Composition
        key={j.id}
        id={j.id}
        component={j.component}
        fps={j.fps}
        width={j.width}
        height={j.height}
        durationInFrames={j.durationInFrames}
      />
    ))}
  </>
);
