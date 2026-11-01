// The runtime animation workspace: a copy of the plugin's studio/kit template at ~/.editroom/kit
// (EDITROOM_KIT_HOME), where node_modules live and where each job's sources are mirrored before
// a typecheck, still or render. Job folders themselves live anywhere (usually
// <work>/studio/jobs/<slot>/); the workspace only ever holds copies.
import { createHash } from "node:crypto";
import {
  existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync, statSync, copyFileSync, rmSync, renameSync, utimesSync,
} from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
import { runProcess } from "./proc.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
/** The plugin root (…/editroom), two levels above studio/jobs/lib. */
export const PLUGIN_ROOT = resolve(HERE, "..", "..", "..");
export const TEMPLATE_DIR = join(PLUGIN_ROOT, "studio", "kit");

export const editroomHome = (env = process.env) => resolve(env.EDITROOM_HOME || join(homedir(), ".editroom"));
export const kitHome = (env = process.env) => resolve(env.EDITROOM_KIT_HOME || join(editroomHome(env), "kit"));
export const nodeBin = () => process.execPath;
export const ffmpegBin = (env = process.env) => env.FFMPEG || "ffmpeg";
export const ffprobeBin = (env = process.env) => env.FFPROBE || "ffprobe";

/** Write JSON atomically (temp file + rename), so readers never see half a file. */
export function writeJsonAtomic(p, obj) {
  mkdirSync(dirname(p), { recursive: true });
  const tmp = `${p}.${process.pid}.tmp`;
  writeFileSync(tmp, JSON.stringify(obj, null, 2) + "\n");
  renameSync(tmp, p);
}

export function readJson(p, fallback = null) {
  try { return JSON.parse(readFileSync(p, "utf8")); } catch { return fallback; }
}

function binOf(dir, pkg, name) {
  const pkgPath = join(dir, "node_modules", ...pkg.split("/"), "package.json");
  try {
    const j = JSON.parse(readFileSync(pkgPath, "utf8"));
    const bin = typeof j.bin === "string" ? j.bin : (j.bin && j.bin[name]) || null;
    return bin ? join(dirname(pkgPath), bin) : null;
  } catch { return null; }
}
export const remotionCli = (kit) => binOf(kit, "@remotion/cli", "remotion");
export const tscBin = (kit) => binOf(kit, "typescript", "tsc");

const SKIP_TOP = new Set(["node_modules", "references", "out", ".git"]);
/** Template paths the sync never touches in the workspace (job mirrors, generated entries). */
const isRuntimePath = (rel) => /^src\/(jobs\/(?!types\.ts$|manifest\.ts$)|entries\/)/.test(rel) || /^public\/(jobs|refs|proof)\//.test(rel);

function walk(dir, base = dir, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    const rel = relative(base, p);
    if (dir === base && SKIP_TOP.has(e.name)) continue;
    if (e.name === ".DS_Store") continue;
    if (e.isDirectory()) walk(p, base, out);
    else if (e.isFile()) out.push(rel);
  }
  return out;
}

/** Copy `src` to `dst` when the size or mtime differ. Returns true when it copied. */
export function copyIfChanged(src, dst) {
  const s = statSync(src);
  if (existsSync(dst)) {
    const d = statSync(dst);
    if (d.size === s.size && Math.abs(d.mtimeMs - s.mtimeMs) < 1) return false;
  }
  mkdirSync(dirname(dst), { recursive: true });
  copyFileSync(src, dst);
  // keep the source mtime so the next sync can skip the file
  try { utimesSync(dst, s.atime, s.mtime); } catch { /* best effort */ }
  return true;
}

/** Mirror a whole folder (files only, changed ones), optionally deleting files that vanished. */
export function mirrorDir(src, dst, { prune = false, filter = () => true } = {}) {
  let copied = 0;
  if (!existsSync(src)) return 0;
  const files = walk(src).filter(filter);
  for (const rel of files) if (copyIfChanged(join(src, rel), join(dst, rel))) copied++;
  if (prune && existsSync(dst)) {
    const keep = new Set(files);
    for (const rel of walk(dst)) if (!keep.has(rel) && filter(rel)) rmSync(join(dst, rel), { force: true });
  }
  return copied;
}

const sha1 = (s) => createHash("sha1").update(s).digest("hex");

/**
 * Make the runtime workspace match the template (changed files only; job mirrors and generated
 * entries are left alone), then `npm ci` when node_modules is missing or package-lock changed.
 */
export async function ensureKit({ env = process.env, log = () => {}, install = true } = {}) {
  const kit = kitHome(env);
  if (!existsSync(TEMPLATE_DIR)) throw new Error(`The kit template is missing: ${TEMPLATE_DIR}`);
  mkdirSync(kit, { recursive: true });
  let synced = 0;
  for (const rel of walk(TEMPLATE_DIR)) {
    if (isRuntimePath(rel)) continue;
    if (copyIfChanged(join(TEMPLATE_DIR, rel), join(kit, rel))) synced++;
  }
  mkdirSync(join(kit, "src", "entries"), { recursive: true });
  const lock = readFileSync(join(TEMPLATE_DIR, "package-lock.json"), "utf8");
  const want = sha1(lock);
  const stampPath = join(kit, ".editroom-kit.json");
  const stamp = readJson(stampPath, {}) || {};
  const haveDeps = existsSync(join(kit, "node_modules", "remotion")) && !!remotionCli(kit);
  let installed = false;
  if (install && (!haveDeps || stamp.lockHash !== want)) {
    log(`installing the animation kit's npm packages in ${kit} (one-time, about a minute)`);
    await runProcess("npm", ["ci", "--no-audit", "--no-fund", "--loglevel=error"], { cwd: kit, env, timeoutMs: 20 * 60000, label: "npm ci" });
    installed = true;
  }
  // Remotion's headless browser (a one-time download per workspace)
  const browserStamp = join(kit, ".editroom-browser");
  if (install && remotionCli(kit) && (installed || !existsSync(browserStamp))) {
    log("making sure Remotion's headless browser is present");
    await runProcess(process.execPath, [remotionCli(kit), "browser", "ensure"], { cwd: kit, env, timeoutMs: 30 * 60000, label: "remotion browser ensure" });
    writeFileSync(browserStamp, String(Date.now()));
  }
  writeJsonAtomic(stampPath, { lockHash: want, template: TEMPLATE_DIR, syncedAt: Date.now() });
  return { kit, template: TEMPLATE_DIR, synced, installed, ready: existsSync(join(kit, "node_modules", "remotion")) && !!remotionCli(kit) };
}

export function assertKitReady(kit) {
  if (!remotionCli(kit)) throw new Error(`The animation kit is not installed at ${kit}. Run: node studio/jobs/job.mjs kit ensure`);
}

/** Source files of a job folder that make up its scene (copied into the workspace). */
const SCENE_FILE = /\.(tsx?|json|md)$/;
const NOT_SCENE = new Set(["render-status.json", "log.jsonl"]);
const isSceneFile = (rel) => !rel.includes("/") ? (SCENE_FILE.test(rel) && !NOT_SCENE.has(rel)) : /^(lib|parts|components)\/.*\.tsx?$/.test(rel);

/**
 * Copy a job's sources into the workspace (src/jobs/<id>/), its assets into public/, and write
 * the job's own entry file (src/entries/<id>.tsx): it registers the job's slot start for the
 * canvas phase, then this one composition. Renders and stills bundle that entry only.
 */
export function mirrorJob(job, kit) {
  const target = join(kit, "src", "jobs", job.id);
  mkdirSync(target, { recursive: true });
  // scene sources (prune the mirror so a renamed component never lingers)
  const n = mirrorDir(job.dir, target, { prune: true, filter: (rel) => isSceneFile(rel) && !rel.startsWith("out/") });
  // assets: <job>/assets -> public/jobs/<id>/, <job>/public -> public/ (as-is, for references),
  // and every extra asset dir in job.json (`assetDirs: [{src, as}]`, e.g. the edit's proof folder)
  let a = mirrorDir(join(job.dir, "assets"), join(kit, "public", "jobs", job.id));
  a += mirrorDir(join(job.dir, "public"), join(kit, "public"));
  for (const d of job.assetDirs || []) if (d && d.src && d.as) a += mirrorDir(resolve(d.src), join(kit, "public", d.as));
  writeFileSync(join(kit, "src", "entries", `${job.id}.tsx`), entrySource(job));
  return { target, scene: n, assets: a };
}

export function entrySource(job) {
  const canvas = job.canvas && Object.keys(job.canvas).length ? JSON.stringify(job.canvas) : "null";
  return `// GENERATED by studio/jobs/job.mjs for ${job.id} (${job.slot || job.title || ""}). Do not edit.
import "../theme/fonts";
import React from "react";
import { Composition, registerRoot } from "remotion";
import { registerJob } from "../../styles/velour/src/phase";
import Scene from "../jobs/${job.id}/Scene";

registerJob(${JSON.stringify(job.id)}, ${Number(job.slot0) || 0}, ${canvas});

const Root: React.FC = () => (
  <Composition
    id=${JSON.stringify(job.id)}
    component={Scene}
    fps={${job.fps}}
    width={${job.width}}
    height={${job.height}}
    durationInFrames={${job.frames}}
  />
);

registerRoot(Root);
`;
}

/** A tsconfig that typechecks one job (plus the kit), so other jobs' errors never block it. */
export function jobTsconfig(job, kit) {
  const p = join(kit, `tsconfig.job-${job.id}.json`);
  writeFileSync(p, JSON.stringify({
    extends: "./tsconfig.json",
    include: ["src/theme", "src/animation", "src/components", "src/sketch", "src/jobs/types.ts", "styles", `src/jobs/${job.id}`, `src/entries/${job.id}.tsx`],
  }, null, 2));
  return p;
}
