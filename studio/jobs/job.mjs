#!/usr/bin/env node
// editroom studio: animation jobs (section films and demo overlays) for cutroom to compose.
//
// A job is a folder anywhere, usually <work>/studio/jobs/<slot>/:
//   job.json      runner-owned record (see README: id, slot, mode, slot0, frames, renders…)
//   brief.md      the binding brief the designer works from
//   words.json    his words in this slot with output ms and slot frames (cutroom timing --json)
//   Scene.tsx     default-exported scene (+ any local .tsx components), velour style
//   render.json   {"version": N, "notes": "...", "title": "..."}: "ready to render v N"
//   assets/       files the scene uses: staticFile(`jobs/<id>/<file>`) (the scaffold's A())
//   out/          renders (<id>-vN.mp4 | .mov, <id>-vN-draft.mp4), logs, status, stills, sheets
//
// Every command prints JSON on stdout and progress on stderr. Renders detach by default (agent
// shells kill long commands): `render` returns at once, `wait` polls. Renders share a global
// queue (EDITROOM_RENDER_SLOTS, default 3); stills, sheets and typechecks never queue.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, copyFileSync, existsSync, mkdirSync, openSync, readdirSync, readFileSync, statSync, writeFileSync, cpSync } from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  PLUGIN_ROOT, TEMPLATE_DIR, assertKitReady, editroomHome, ensureKit, ffmpegBin, jobTsconfig, kitHome, mirrorJob, nodeBin, readJson, tscBin, writeJsonAtomic,
} from "./lib/kit.mjs";
import { acquireSlot, queueState } from "./lib/queue.mjs";
import { countFrames, renderJob } from "./lib/render.mjs";
import { pidAlive, runProcess, sleep } from "./lib/proc.mjs";
import { scaffoldScene } from "./lib/scaffold.mjs";

const THIS = fileURLToPath(import.meta.url);
const FPS = 30;

export function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--") && a.length > 2) {
      const k = a.slice(2);
      const nx = argv[i + 1];
      if (nx !== undefined && !(nx.startsWith("--") && nx.length > 2)) { out[k] = nx; i++; } else out[k] = true;
    } else out._.push(a);
  }
  return out;
}

const hash6 = (s) => createHash("sha1").update(String(s)).digest("hex").slice(0, 6);
const slug = (s) => String(s || "job").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 32) || "job";
/** Remotion composition ids: letters, digits and dashes. */
export const jobIdFor = (slot, dir) => `${slug(slot)}-${hash6(resolve(dir))}`;
/** Frame of an output time inside a slot (the same formula every brief and checker uses). */
export const frameAt = (ms, slot0) => Math.round(((ms - slot0) * FPS) / 1000);

/* ------------------------------------------------------------ job record */

function loadJob(dirArg) {
  if (!dirArg) throw new Error("Give the job folder (e.g. <work>/studio/jobs/T4).");
  const dir = resolve(String(dirArg));
  const job = readJson(join(dir, "job.json"), null);
  if (!job || !job.id) throw new Error(`No job at ${dir} (job.json missing). Create it with: job.mjs create --dir ${dir} ...`);
  job.dir = dir;
  return job;
}

function saveJob(job) {
  const { dir, ...rest } = job;
  writeJsonAtomic(join(dir, "job.json"), rest);
}

function logTo(job, entry) {
  try { appendFileSync(join(job.dir, "log.jsonl"), JSON.stringify({ ts: Date.now(), ...entry }) + "\n"); } catch { /* best effort */ }
}

const statusPath = (job, draft) => join(job.dir, "out", draft ? "render-status-draft.json" : "render-status.json");

function readSignal(job) {
  const s = readJson(join(job.dir, "render.json"), null);
  if (!s) return null;
  const v = parseInt(s.version, 10);
  return Number.isFinite(v) && v > 0 ? { version: v, notes: s.notes || "", title: s.title || null } : null;
}

/* ------------------------------------------------------------ create */

export function createJob(a, { log = () => {} } = {}) {
  if (!a.dir) throw new Error("--dir <job folder> is required.");
  const dir = resolve(String(a.dir));
  if (existsSync(join(dir, "job.json")) && !a.force) throw new Error(`${dir} already has a job.json (pass --force to recreate the record; Scene.tsx is never overwritten).`);
  const slot = String(a.slot || basename(dir));
  const slot0 = Number(a.slot0);
  if (!Number.isFinite(slot0) || slot0 < 0) throw new Error("--slot0 <ms> is required: the slot's start in OUTPUT time (cutroom slots).");
  let frames = a.frames !== undefined ? parseInt(a.frames, 10) : NaN;
  if (!Number.isFinite(frames) && a.end !== undefined) frames = frameAt(Number(a.end), slot0);
  if (!Number.isFinite(frames) || frames < 15) throw new Error("--frames <N> (or --end <output ms>) is required, at least 15 frames.");
  const mode = String(a.mode || "behind");
  if (!["behind", "front"].includes(mode)) throw new Error("--mode must be behind (a full-frame opaque section film) or front (a transparent demo overlay).");
  const width = parseInt(a.width || "1920", 10), height = parseInt(a.height || "1080", 10);
  const id = a.id ? String(a.id) : jobIdFor(slot, dir);
  if (!/^[a-zA-Z0-9-]+$/.test(id)) throw new Error(`Job id "${id}" may only hold letters, digits and dashes.`);
  const assetDirs = [];
  let proofAs = null;
  if (a["proof-dir"]) {
    const pd = resolve(String(a["proof-dir"]));
    proofAs = `proof/${hash6(pd)}`;
    assetDirs.push({ src: pd, as: proofAs });
  }
  const canvas = a.canvas ? JSON.parse(String(a.canvas)) : null;
  mkdirSync(join(dir, "assets"), { recursive: true });
  mkdirSync(join(dir, "out"), { recursive: true });
  const job = {
    id, slot, title: a.title ? String(a.title) : slot, mode,
    background: mode === "front" ? "transparent" : "solid",
    style: "velour", fps: FPS, width, height, frames, durationInFrames: frames,
    slot0, slotEnd: slot0 + (frames * 1000) / FPS,
    canvas, assetDirs, proofAs,
    work: a.work ? resolve(String(a.work)) : null,
    cutsHash: a["cuts-hash"] ? String(a["cuts-hash"]) : null,
    createdAt: Date.now(), currentVersion: 0, lastRenderedVersion: 0, renders: [], drafts: [],
    dir,
  };
  if (a.words) copyFileSync(resolve(String(a.words)), join(dir, "words.json"));
  if (a.brief) copyFileSync(resolve(String(a.brief)), join(dir, "brief.md"));
  const scenePath = join(dir, "Scene.tsx");
  if (!existsSync(scenePath)) writeFileSync(scenePath, scaffoldScene(job));
  saveJob(job);
  logTo(job, { kind: "created", text: `${mode === "front" ? "Overlay" : "Section film"} ${slot}: ${frames} frames from ${slot0} ms.` });
  log(`created ${id} in ${dir}`);
  return { ok: true, id, dir, frames, slot0, slotEnd: job.slotEnd, mode, scene: scenePath, words: existsSync(join(dir, "words.json")), brief: existsSync(join(dir, "brief.md")) };
}

/* ------------------------------------------------------------ render */

async function renderForeground(job, { draft = false, force = false, env = process.env, log = () => {} }) {
  const kit = kitHome(env);
  assertKitReady(kit);
  const sig = readSignal(job);
  let version;
  if (draft) version = sig ? sig.version : (job.lastRenderedVersion || 0) + 1;
  else {
    if (!sig) throw new Error(`No render.json in ${job.dir}: the design step writes {"version": N, "notes": "..."} when the scene is ready.`);
    if (sig.version <= (job.lastRenderedVersion || 0) && !force) throw new Error(`v${sig.version} of ${job.id} is already rendered. Bump render.json's version, or pass --force.`);
    version = sig.version;
  }
  const sp = statusPath(job, draft);
  const base = { state: "running", version, draft, progress: 0, pid: process.pid, startedAt: Date.now(), message: "starting" };
  writeJsonAtomic(sp, base);
  const logFile = join(job.dir, "out", `render-v${version}${draft ? "-draft" : ""}.log`);
  const mirror = mirrorJob(job, kit);
  log(`mirrored ${mirror.scene} source and ${mirror.assets} asset files`);
  const q = await acquireSlot({ job: job.id, dir: job.dir, version, draft }, {
    env,
    onWait: (st) => { log(`waiting for a render slot (${st.busy}/${st.slots} busy)`); writeJsonAtomic(sp, { ...base, state: "running", queued: true, message: `queued: ${st.busy} of ${st.slots} render slots busy` }); },
  });
  const queuedMs = q.waitedMs;
  try {
    const r = await renderJob({
      job, kit, version, draft, logFile, env,
      onProgress: (text, pct) => { log(text); writeJsonAtomic(sp, { ...base, queued: false, slot: q.slot, queuedMs, progress: pct || 0, message: text }); },
    });
    const elapsedMs = Date.now() - base.startedAt;
    const rec = { version, file: r.file, path: r.path, frames: r.frames, codec: r.codec, bytes: r.bytes, renderMs: r.renderMs, queuedMs, elapsedMs, ts: Date.now(), notes: sig ? sig.notes : "", title: sig ? sig.title : null };
    const fresh = loadJob(job.dir); // another command may have touched the record meanwhile
    if (draft) fresh.drafts = [...(fresh.drafts || []).filter((d) => d.version !== version), rec];
    else {
      fresh.renders = [...(fresh.renders || []).filter((d) => d.version !== version), rec];
      fresh.lastRenderedVersion = version;
      fresh.currentVersion = Math.max(fresh.currentVersion || 0, version);
      if (sig && sig.title) fresh.title = sig.title;
      // keep the scene that made this version
      mkdirSync(join(job.dir, "out", "history"), { recursive: true });
      copyFileSync(join(job.dir, "Scene.tsx"), join(job.dir, "out", "history", `v${version}-Scene.tsx`));
    }
    saveJob(fresh);
    logTo(job, { kind: draft ? "draft" : "rendered", text: `${draft ? "Draft" : "Rendered"} v${version}: ${r.file}, ${r.frames} frames, ${(r.renderMs / 1000).toFixed(1)} s render, ${(queuedMs / 1000).toFixed(1)} s queued.`, version });
    const done = { state: "done", ...rec, draft, slot: q.slot, progress: 100, pid: process.pid, startedAt: base.startedAt, finishedAt: Date.now(), logPath: logFile };
    writeJsonAtomic(sp, done);
    return done;
  } catch (e) {
    logTo(job, { kind: "error", text: `${draft ? "Draft" : "Render"} v${version} failed: ${e.message}`, version });
    writeJsonAtomic(sp, { state: "failed", version, draft, pid: process.pid, startedAt: base.startedAt, finishedAt: Date.now(), queuedMs, elapsedMs: Date.now() - base.startedAt, error: e.message, logPath: logFile });
    throw e;
  } finally {
    q.release();
  }
}

function renderDetached(job, { draft = false, force = false, env = process.env } = {}) {
  const sp = statusPath(job, draft);
  const running = readJson(sp, null);
  if (running && running.state === "running" && pidAlive(running.pid)) return { state: "running", alreadyRunning: true, statusPath: sp, version: running.version, pid: running.pid };
  mkdirSync(join(job.dir, "out"), { recursive: true });
  const fd = openSync(join(job.dir, "out", `worker${draft ? "-draft" : ""}.log`), "a");
  const args = [THIS, "render", job.dir, "--foreground"];
  if (draft) args.push("--draft");
  if (force) args.push("--force");
  writeJsonAtomic(sp, { state: "running", draft, progress: 0, pid: null, startedAt: Date.now(), message: "starting" });
  const child = spawn(nodeBin(), args, { detached: true, stdio: ["ignore", fd, fd], env });
  child.unref();
  const st = readJson(sp, {});
  if (st.state === "running" && !st.pid) writeJsonAtomic(sp, { ...st, pid: child.pid });
  return { state: "running", draft, statusPath: sp, pid: child.pid };
}

function readStatus(job, draft) {
  const sp = statusPath(job, draft);
  const st = readJson(sp, null);
  if (!st) return { state: "idle", draft, lastRenderedVersion: job.lastRenderedVersion || 0, statusPath: sp };
  if (st.state === "running" && st.pid && !pidAlive(st.pid) && Date.now() - (st.startedAt || 0) > 5000) {
    const dead = { ...st, state: "failed", error: "the render worker exited without reporting (see out/worker.log)", finishedAt: Date.now() };
    writeJsonAtomic(sp, dead);
    return { ...dead, statusPath: sp };
  }
  return { ...st, statusPath: sp };
}

async function waitFor(job, { draft = false, timeoutSec = 540 } = {}) {
  const end = Date.now() + timeoutSec * 1000;
  for (;;) {
    const st = readStatus(job, draft);
    if (st.state !== "running") return st;
    if (Date.now() >= end) return { ...st, timedOut: true };
    await sleep(1000);
  }
}

/* ------------------------------------------------------------ stills and sheets */

async function stills(job, frames, { outDir, scale = 1, prefix = "f", env = process.env } = {}) {
  const kit = kitHome(env);
  assertKitReady(kit);
  mirrorJob(job, kit);
  const out = outDir || join(job.dir, "out", "stills");
  const { stdout } = await runProcess(nodeBin(), [join(dirname(THIS), "lib", "stills.mjs"), kit, `src/entries/${job.id}.tsx`, job.id, out, "--frames", frames.join(","), "--scale", String(scale), "--prefix", prefix], { cwd: kit, env, label: "remotion stills", stallMs: 300000 });
  const line = stdout.trim().split("\n").filter((l) => l.startsWith("{")).pop();
  return JSON.parse(line);
}

/** Tile PNGs into one contact sheet (ffmpeg xstack, `cols` per row). */
async function tile(paths, outPath, { cols = 4, width = 480, env = process.env } = {}) {
  const rows = Math.ceil(paths.length / cols);
  const inputs = paths.flatMap((p) => ["-i", p]);
  const scaled = paths.map((_, i) => `[${i}:v]scale=${width}:-2,pad=iw+8:ih+8:4:4:color=#202020[s${i}]`).join(";");
  const blanks = [];
  const cells = paths.map((_, i) => `[s${i}]`);
  for (let i = paths.length; i < rows * cols; i++) { blanks.push(`color=c=#202020:s=${width + 8}x${Math.round(width * 9 / 16) + 8}:d=1[b${i}]`); cells.push(`[b${i}]`); }
  const layout = [];
  for (let i = 0; i < rows * cols; i++) {
    const c = i % cols, r = Math.floor(i / cols);
    layout.push(`${c ? Array.from({ length: c }, (_, k) => `w${k}`).join("+") : "0"}_${r ? Array.from({ length: r }, (_, k) => `h${k * cols}`).join("+") : "0"}`);
  }
  const fc = [scaled, ...blanks, `${cells.join("")}xstack=inputs=${rows * cols}:layout=${layout.join("|")}[o]`].filter(Boolean).join(";");
  mkdirSync(dirname(outPath), { recursive: true });
  await runProcess(ffmpegBin(env), ["-y", "-hide_banner", "-loglevel", "error", ...inputs, "-filter_complex", fc, "-map", "[o]", "-frames:v", "1", outPath], { env, label: "ffmpeg tile", timeoutMs: 300000 });
  return outPath;
}

/** Frames of a rendered file at given seconds (alpha composited over the canvas colour), tiled. */
async function sheetFromFile(file, { at = null, every = null, count = 12, out, env = process.env } = {}) {
  const info = await countFrames(file, { env });
  const { stdout } = await runProcess("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", file], { env, label: "ffprobe" });
  const dur = Number(stdout.trim()) || 0;
  let times = at ? String(at).split(",").map(Number).filter((n) => Number.isFinite(n)) : null;
  if (!times && every) { times = []; for (let t = 0; t < dur; t += Number(every)) times.push(t); }
  if (!times) times = Array.from({ length: count }, (_, i) => (dur * (i + 0.5)) / count);
  // land on frame boundaries (30 fps), so the grabbed frame is the one asked for
  times = times.map((t) => Math.round(t * 30) / 30);
  const tmp = join(dirname(out), `.sheet-${process.pid}`);
  mkdirSync(tmp, { recursive: true });
  const alpha = /a/.test(String(info.pixFmt || "")) && /yuva|rgba|argb|bgra|gbrap/.test(String(info.pixFmt || ""));
  const paths = [];
  for (const [i, t] of times.entries()) {
    const p = join(tmp, `t${String(i).padStart(3, "0")}.png`);
    // alpha renders: over a mid-grey checkerboard, so dark cards and light text both read
    const vf = alpha
      ? `[0:v]setpts=PTS-STARTPTS,format=rgba[fg];nullsrc=s=${info.width}x${info.height},format=gray,geq=lum='if(mod(floor(X/40)+floor(Y/40)\\,2)\\,96\\,72)'[bg];[bg][fg]overlay=shortest=1`
      : "null";
    const args = ["-y", "-hide_banner", "-loglevel", "error", "-ss", String(Math.max(0, t)), "-i", file, "-frames:v", "1"];
    if (alpha) args.push("-filter_complex", vf); else args.push("-vf", vf);
    args.push(p);
    await runProcess(ffmpegBin(env), args, { env, label: "ffmpeg frame", timeoutMs: 120000 });
    paths.push(p);
  }
  await tile(paths, out, { env });
  return { path: out, times, frames: info.frames, alpha, durationSec: dur };
}

function lastRender(job, { draft = false } = {}) {
  const list = draft ? job.drafts || [] : job.renders || [];
  return list.length ? list[list.length - 1] : null;
}

/* ------------------------------------------------------------ reference jobs */

function referenceJob(name, { env = process.env } = {}) {
  const src = join(TEMPLATE_DIR, "references", name);
  if (!existsSync(src)) throw new Error(`No reference "${name}". Available: ${readdirSync(join(TEMPLATE_DIR, "references")).filter((f) => !f.includes(".")).join(", ")}`);
  const meta = readJson(join(src, "source-job.json"), {});
  const dir = join(editroomHome(env), "reference-jobs", name);
  mkdirSync(dir, { recursive: true });
  cpSync(src, dir, { recursive: true, force: true });
  if (!existsSync(join(dir, "job.json"))) {
    createJob({ dir, slot: name, id: meta.id, slot0: String(meta.startMs || 0), frames: String(meta.durationInFrames), mode: meta.background === "transparent" ? "front" : "behind", title: meta.title || name, force: true });
  }
  return loadJob(dir);
}

/* ------------------------------------------------------------ main */

export async function main(argv = process.argv.slice(2)) {
  const a = parseArgs(argv);
  const cmd = a._[0];
  const quiet = !!a.quiet;
  const log = (m) => { if (!quiet) process.stderr.write(`[job] ${m}\n`); };
  const print = (o) => process.stdout.write(JSON.stringify(o, null, 2) + "\n");
  try {
    switch (cmd) {
      case "kit": {
        const sub = a._[1] || "status";
        if (sub === "ensure") { print(await ensureKit({ log })); return 0; }
        const kit = kitHome();
        print({ kit, template: TEMPLATE_DIR, plugin: PLUGIN_ROOT, installed: existsSync(join(kit, "node_modules", "remotion")), stamp: readJson(join(kit, ".editroom-kit.json"), null) });
        return 0;
      }
      case "create": print(createJob(a, { log })); return 0;
      case "render": {
        const job = loadJob(a._[1]);
        const draft = !!a.draft, force = !!a.force;
        if (a.foreground) { print(await renderForeground(job, { draft, force, log })); return 0; }
        print(renderDetached(job, { draft, force }));
        return 0;
      }
      case "wait": {
        const job = loadJob(a._[1]);
        const st = await waitFor(job, { draft: !!a.draft, timeoutSec: Number(a.timeout) > 0 ? Number(a.timeout) : 540 });
        print(st);
        return st.state === "failed" ? 1 : 0;
      }
      case "status": {
        const job = loadJob(a._[1]);
        print({ final: readStatus(job, false), draft: readStatus(job, true) });
        return 0;
      }
      case "typecheck": {
        const job = loadJob(a._[1]);
        const kit = kitHome();
        assertKitReady(kit);
        mirrorJob(job, kit);
        const cfg = jobTsconfig(job, kit);
        try {
          await runProcess(nodeBin(), [tscBin(kit), "-p", cfg, "--noEmit"], { cwd: kit, label: "tsc", timeoutMs: 600000 });
          print({ ok: true, id: job.id });
          return 0;
        } catch (e) {
          // tsc reports paths inside the workspace mirror: point them back at the job folder
          const msg = String(e.message).split(`src/jobs/${job.id}/`).join(`${job.dir}/`);
          print({ ok: false, id: job.id, errors: msg.slice(-6000) });
          return 1;
        }
      }
      case "still": {
        const job = loadJob(a._[1]);
        const frame = Math.max(0, parseInt(a.frame || "0", 10) || 0);
        const r = await stills(job, [frame], { outDir: a.out ? dirname(resolve(String(a.out))) : undefined, scale: Number(a.scale) || 1, prefix: a.out ? basename(String(a.out), extname(String(a.out))) + "-" : "f" });
        let path = r.stills[0].path;
        if (a.out) { copyFileSync(path, resolve(String(a.out))); path = resolve(String(a.out)); }
        print({ ok: true, frame, path, bundleMs: r.bundleMs, renderMs: r.renderMs });
        return 0;
      }
      case "stills": {
        const job = loadJob(a._[1]);
        let frames = a.frames ? String(a.frames).split(",").map((s) => parseInt(s, 10)).filter((n) => Number.isFinite(n)) : null;
        if (!frames) {
          const every = parseInt(a.every || "0", 10) || Math.max(1, Math.floor(job.frames / 12));
          frames = [];
          for (let f = Math.floor(every / 2); f < job.frames; f += every) frames.push(f);
        }
        const outDir = a.out ? resolve(String(a.out)) : join(job.dir, "out", "stills", `s${Date.now()}`);
        const r = await stills(job, frames, { outDir, scale: Number(a.scale) || 0.5 });
        let sheet = null;
        if (a.sheet) sheet = await tile(r.stills.map((s) => s.path), join(outDir, "sheet.png"), { cols: Number(a.cols) || 4 });
        print({ ok: true, ...r, sheet });
        return 0;
      }
      case "sheet": {
        const target = a._[1];
        let file, out;
        if (target && existsSync(target) && statSync(target).isFile()) {
          file = resolve(target);
          out = a.out ? resolve(String(a.out)) : join(dirname(file), `${basename(file, extname(file))}-sheet.png`);
        } else {
          const job = loadJob(target);
          const last = lastRender(job, { draft: !!a.draft }) || lastRender(job, { draft: !a.draft });
          if (!last) throw new Error(`No render of ${job.id} yet.`);
          file = last.path;
          out = a.out ? resolve(String(a.out)) : join(job.dir, "out", "sheets", `${basename(file, extname(file))}-sheet.png`);
        }
        print(await sheetFromFile(file, { at: a.at || null, every: a.every || null, count: Number(a.count) || 12, out }));
        return 0;
      }
      case "probe": {
        const f = a._[1];
        if (!f || !existsSync(f)) throw new Error(`probe: no such file ${f || ""}`);
        print({ path: resolve(f), ...(await countFrames(resolve(f))) });
        return 0;
      }
      case "show": print(loadJob(a._[1])); return 0;
      case "signal": {
        const job = loadJob(a._[1]);
        const s = readSignal(job);
        print({ ...(s || { version: null }), pending: !!s && s.version > (job.lastRenderedVersion || 0), lastRenderedVersion: job.lastRenderedVersion || 0 });
        return 0;
      }
      case "list": {
        const root = a.work ? join(resolve(String(a.work)), "studio", "jobs") : resolve(String(a._[1] || "."));
        const jobs = [];
        for (const e of existsSync(root) ? readdirSync(root, { withFileTypes: true }) : []) {
          if (!e.isDirectory()) continue;
          const j = readJson(join(root, e.name, "job.json"), null);
          if (!j) continue;
          const st = readJson(join(root, e.name, "out", "render-status.json"), null);
          jobs.push({ slot: j.slot, id: j.id, mode: j.mode, frames: j.frames, slot0: j.slot0, lastRenderedVersion: j.lastRenderedVersion || 0, render: st ? { state: st.state, version: st.version, progress: st.progress } : null, dir: join(root, e.name) });
        }
        print(jobs.sort((x, y) => x.slot0 - y.slot0));
        return 0;
      }
      case "log": {
        const job = loadJob(a._[1]);
        logTo(job, { kind: String(a.kind || "note"), text: String(a.text || "") });
        print({ ok: true });
        return 0;
      }
      case "queue": print(queueState()); return 0;
      case "reference": {
        const job = referenceJob(String(a._[1] || ""));
        if (a.render === undefined && a.draft === undefined) { print({ ok: true, dir: job.dir, id: job.id, frames: job.frames }); return 0; }
        const draft = !!a.draft;
        if (a.foreground) { print(await renderForeground(job, { draft, force: true, log })); return 0; }
        print({ dir: job.dir, ...renderDetached(job, { draft, force: true }) });
        return 0;
      }
      case undefined:
      case "help":
        process.stderr.write("job.mjs kit [ensure|status] | create --dir D --slot S --slot0 MS --frames N [--mode behind|front] [--words F] [--brief F] [--proof-dir D] | render D [--draft] [--force] [--foreground] | wait D [--draft] | status D | typecheck D | still D --frame N | stills D [--frames a,b|--every N] [--sheet] | sheet D|FILE [--at s,s|--every s] | probe FILE | show D | signal D | list --work W | log D --text T | queue | reference NAME [--draft|--render]\n");
        return cmd ? 0 : 2;
      default:
        throw new Error(`Unknown command "${cmd}".`);
    }
  } catch (e) {
    process.stderr.write(`[job] ${e.message}\n`);
    print({ ok: false, error: e.message });
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().then((c) => process.exit(c), (e) => { process.stdout.write(JSON.stringify({ ok: false, error: String(e && e.message || e) }) + "\n"); process.exit(1); });
}
