// One render of a job: Remotion CLI on the job's own entry (bundles only that scene), inside a
// global queue slot, with a stall watchdog and one retry on transient renderer failures; then a
// stream-copy remux (no audio track) and a frame-exact check against the job's frame count.
import { existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { ffmpegBin, ffprobeBin, nodeBin, remotionCli } from "./kit.mjs";
import { runProcess } from "./proc.mjs";
import { renderConcurrency } from "./queue.mjs";

/** Silence this long means the renderer is wedged (Remotion prints a line per frame). */
export const stallMs = (env = process.env) => {
  const v = parseInt(env.EDITROOM_RENDER_STALL_MS || "", 10);
  return Number.isFinite(v) && v > 0 ? v : 600000;
};

export function renderFileName(job, version, { draft = false } = {}) {
  if (draft) return `${job.id}-v${version}-draft.mp4`;
  return `${job.id}-v${version}${job.background === "transparent" ? ".mov" : ".mp4"}`;
}

/** Remotion CLI arguments (after the CLI path). Pure. */
export function renderArgs(job, tmp, { draft = false, concurrency = 8 } = {}) {
  const args = [
    "render", `src/entries/${job.id}.tsx`, job.id, tmp,
    "--timeout=120000", "--image-format=png", "--overwrite", '--props={"final":true}', "--muted",
    `--concurrency=${concurrency}`,
  ];
  if (draft) args.push("--scale=0.5", "--codec=h264", "--crf=23", "--pixel-format=yuv420p");
  else if (job.background === "transparent") args.push("--codec=prores", "--prores-profile=4444", "--pixel-format=yuva444p10le");
  else args.push("--codec=h264", "--crf=14", "--pixel-format=yuv420p");
  return args;
}

/** "Rendered 123/456" style progress out of the CLI output. Pure. */
export function parseProgress(chunk) {
  const all = String(chunk || "").match(/(\d+)\s*\/\s*(\d+)/g);
  if (!all) return null;
  const m = /(\d+)\s*\/\s*(\d+)/.exec(all[all.length - 1]);
  const done = Number(m[1]), total = Number(m[2]);
  if (!total || done > total) return null;
  return Math.min(99, Math.round((done / total) * 100));
}

export function isTransient(message) {
  const m = String(message || "");
  if (/Cancelled|timed out after \d+ min/i.test(m)) return false;
  if (/stopped reporting progress/i.test(m)) return true;
  return /delayRender|renderer crashed|Target closed|Session closed|Protocol error|browser has disconnected|out of memory|ENOMEM|ECONNRESET|socket hang up/i.test(m);
}

/** Exact frame count of a video file (packets in the first video stream). */
export async function countFrames(path, { env = process.env } = {}) {
  const { stdout } = await runProcess(ffprobeBin(env), ["-v", "error", "-select_streams", "v:0", "-count_packets", "-show_entries", "stream=nb_read_packets,width,height,pix_fmt,codec_name", "-of", "json", path], { env, label: "ffprobe", timeoutMs: 120000 });
  const s = (JSON.parse(stdout).streams || [])[0] || {};
  return { frames: Number(s.nb_read_packets) || null, width: s.width || null, height: s.height || null, pixFmt: s.pix_fmt || null, codec: s.codec_name || null };
}

export async function renderJob({ job, kit, version, draft = false, logFile = null, env = process.env, onProgress = () => {} }) {
  const cli = remotionCli(kit);
  if (!cli) throw new Error(`Remotion is not installed in ${kit}. Run: node studio/jobs/job.mjs kit ensure`);
  const outDir = join(job.dir, "out");
  mkdirSync(outDir, { recursive: true });
  const file = renderFileName(job, version, { draft });
  const outPath = join(outDir, file);
  const tmp = join(outDir, `.render-tmp-${version}${draft ? "-draft.mp4" : job.background === "transparent" ? ".mov" : ".mp4"}`);
  rmSync(tmp, { force: true });
  const args = [cli, ...renderArgs(job, tmp, { draft, concurrency: renderConcurrency(env) })];
  const attempts = 2;
  const t0 = Date.now();
  for (let attempt = 1; ; attempt++) {
    onProgress(attempt === 1 ? `rendering v${version}${draft ? " (draft)" : ""}` : `rendering v${version} again (attempt ${attempt})`, 0);
    let last = -1;
    const onChunk = (s) => { const p = parseProgress(s); if (p != null && p !== last) { last = p; onProgress(`rendering v${version}${draft ? " (draft)" : ""}: ${p}%`, p); } };
    try {
      await runProcess(nodeBin(), args, { cwd: kit, env, stallMs: stallMs(env), label: "remotion render", onStdout: onChunk, onStderr: onChunk, logFile });
      break;
    } catch (e) {
      if (attempt >= attempts || !isTransient(e.message)) throw e;
      rmSync(tmp, { force: true });
    }
  }
  if (!existsSync(tmp)) throw new Error("Remotion reported success but wrote no file.");
  const renderMs = Date.now() - t0;
  onProgress("remuxing", 99);
  rmSync(outPath, { force: true });
  const mp4 = outPath.endsWith(".mp4");
  await runProcess(ffmpegBin(env), ["-y", "-hide_banner", "-loglevel", "error", "-i", tmp, "-map", "0:v:0", "-c:v", "copy", "-an", ...(mp4 ? ["-movflags", "+faststart"] : []), outPath], { env, label: "ffmpeg remux", timeoutMs: 600000, logFile });
  rmSync(tmp, { force: true });
  const probe = await countFrames(outPath, { env });
  if (probe.frames !== job.frames) {
    throw new Error(`The render has ${probe.frames} frames but the slot needs exactly ${job.frames}. (Scene compositions take their length from job.json; nothing in Scene.tsx may change it.)`);
  }
  return {
    path: outPath, file, draft, version, codec: draft ? "h264-draft" : job.background === "transparent" ? "prores4444-alpha" : "h264",
    frames: probe.frames, width: probe.width, height: probe.height, pixFmt: probe.pixFmt, bytes: statSync(outPath).size, renderMs,
  };
}
