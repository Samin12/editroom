// The global render queue: N slots shared by every render on this Mac (all edits, all agents),
// so a design workflow with a dozen sections never starts a dozen Chrome farms at once (the
// Oct 29 meltdown: 12 renders fighting over 36 cores, each 7x slower). Each slot is a directory
// lock with the holder's pid; a dead holder's slot is reclaimed.
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { editroomHome } from "./kit.mjs";
import { pidAlive, sleep } from "./proc.mjs";

export const queueDir = (env = process.env) => join(editroomHome(env), "render-queue");

export function slotCount(env = process.env) {
  const n = parseInt(env.EDITROOM_RENDER_SLOTS || "", 10);
  return Number.isFinite(n) && n > 0 ? Math.min(n, 16) : 3;
}

export function renderConcurrency(env = process.env) {
  const n = parseInt(env.EDITROOM_RENDER_CONCURRENCY || "", 10);
  return Number.isFinite(n) && n > 0 ? n : 8;
}

function holder(dir) {
  try { return JSON.parse(readFileSync(join(dir, "holder.json"), "utf8")); } catch { return null; }
}

function tryTake(dir, info) {
  try {
    mkdirSync(dir);
  } catch (e) {
    if (!e || e.code !== "EEXIST") throw e;
    const h = holder(dir);
    let age = 0;
    try { age = Date.now() - statSync(dir).mtimeMs; } catch { /* raced */ }
    // a dead holder, or a lock that never got its holder file (a crash between mkdir and write)
    if ((h && !pidAlive(h.pid)) || (!h && age > 10000)) {
      try { rmSync(dir, { recursive: true, force: true }); } catch { /* raced */ }
      return tryTake(dir, info);
    }
    return false;
  }
  writeFileSync(join(dir, "holder.json"), JSON.stringify({ ...info, pid: process.pid, since: Date.now() }));
  return true;
}

/** Who holds which slot right now (for `job.mjs queue`). */
export function queueState(env = process.env) {
  const dir = queueDir(env);
  const n = slotCount(env);
  const slots = [];
  for (let k = 0; k < n; k++) {
    const d = join(dir, `slot-${k}`);
    const h = existsSync(d) ? holder(d) : null;
    slots.push({ slot: k, busy: !!(h && pidAlive(h.pid)), holder: h });
  }
  let stray = [];
  try { stray = readdirSync(dir).filter((f) => /^slot-\d+$/.test(f) && Number(f.slice(5)) >= n); } catch { /* none */ }
  return { slots: n, dir, busy: slots.filter((s) => s.busy).length, entries: slots, beyondLimit: stray };
}

/**
 * Wait for a free slot (polling once a second, up to `waitMs`), then return a release function.
 * `onWait(state)` is called once when the caller has to queue.
 */
export async function acquireSlot(info, { env = process.env, waitMs = 3 * 3600000, onWait = () => {} } = {}) {
  const dir = queueDir(env);
  mkdirSync(dir, { recursive: true });
  const n = slotCount(env);
  const started = Date.now();
  let told = false;
  for (;;) {
    for (let k = 0; k < n; k++) {
      const d = join(dir, `slot-${k}`);
      if (tryTake(d, info)) {
        return { slot: k, waitedMs: Date.now() - started, release: () => { try { rmSync(d, { recursive: true, force: true }); } catch { /* gone */ } } };
      }
    }
    if (Date.now() - started > waitMs) throw new Error(`No render slot came free in ${Math.round(waitMs / 60000)} min (${n} slots, all busy).`);
    if (!told) { told = true; onWait(queueState(env)); }
    await sleep(1000);
  }
}
