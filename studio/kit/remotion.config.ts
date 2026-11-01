import { Config } from "@remotion/cli/config";

// Applies to Remotion Studio + the `remotion` CLI only (NOT the @remotion/renderer Node API).
// PNG (lossless) intermediate frames keep fine hand-drawn text crisp through
// camera motion (JPEG intermediates mangle thin strokes on the zoom).
// NO Config.setCrf here: a global CRF makes every ProRes render throw
// ("prores" does not support --crf), and front-mode (transparent) jobs render ProRes 4444.
// The plugin's job.mjs render passes --crf=14 explicitly on its h264 renders instead.
Config.setVideoImageFormat("png");
Config.setOverwriteOutput(true);
Config.setStillImageFormat("png");

// NO GLOBAL MUTE. Every shipped style renders silent video (the narration lives
// on the Borumi timeline) and job.mjs render passes --muted explicitly for
// those, which is the real backstop. It cannot live here because a CLI flag
// cannot UN-mute a config that mutes: a style that declares "audio": true in
// its style.json (one that makes its own sound) would be silent forever.

// Keyframe every 30 frames (1s). x264's default GOP is huge, and desktop
// editors fail to seek deep into a long GOP ("Error retrieving frame N ... substituting"
// about 30s in, seen upstream). Dense keyframes make renders editor-safe; the
// slight size cost is fine for a deliverable, and Borumi re-encodes on export anyway.
Config.overrideFfmpegCommand(({ type, args }) => {
  if (type !== "stitcher") return args;
  return [...args.slice(0, -1), "-g", "30", args[args.length - 1]];
});

// Job compositions are created at the Borumi canvas's exact pixel size, so
// no scaling is needed (job.mjs render passes explicit flags on final renders anyway).
Config.setScale(1);
