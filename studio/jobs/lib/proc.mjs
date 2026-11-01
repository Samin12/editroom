// Child processes with a stall watchdog (silence for `stallMs` = wedged) and an optional log file.
import { spawn } from "node:child_process";
import { openSync, writeSync, closeSync } from "node:fs";

export function runProcess(bin, args, { cwd, env = process.env, onStdout, onStderr, timeoutMs = 0, stallMs = 0, label = bin, logFile = null } = {}) {
  return new Promise((resolveP, reject) => {
    let child;
    try {
      child = spawn(bin, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    } catch (e) {
      reject(new Error(`Could not run ${label}: ${e.message}`));
      return;
    }
    let fd = null;
    if (logFile) { try { fd = openSync(logFile, "a"); } catch { fd = null; } }
    const logOut = (s) => { if (fd != null) { try { writeSync(fd, s); } catch { /* ignore */ } } };
    let timedOut = false, stalled = false;
    const kill = () => { try { child.kill("SIGKILL"); } catch { /* gone */ } };
    const timer = timeoutMs ? setTimeout(() => { timedOut = true; kill(); }, timeoutMs) : null;
    let stallTimer = stallMs ? setTimeout(() => { stalled = true; kill(); }, stallMs) : null;
    const bump = () => {
      if (!stallMs) return;
      clearTimeout(stallTimer);
      stallTimer = setTimeout(() => { stalled = true; kill(); }, stallMs);
    };
    const done = () => { if (timer) clearTimeout(timer); if (stallTimer) clearTimeout(stallTimer); if (fd != null) { try { closeSync(fd); } catch { /* ignore */ } fd = null; } };
    let out = "", err = "";
    child.stdout.on("data", (d) => { bump(); const s = d.toString(); logOut(s); out += s; if (out.length > 200000) out = out.slice(-200000); if (onStdout) onStdout(s); });
    child.stderr.on("data", (d) => { bump(); const s = d.toString(); logOut(s); err += s; if (err.length > 20000) err = err.slice(-20000); if (onStderr) onStderr(s); });
    child.on("error", (e) => { done(); reject(new Error(`Could not run ${label}: ${e.message}`)); });
    child.on("close", (code) => {
      done();
      if (stalled) return reject(new Error(`${label} stopped reporting progress for ${Math.round(stallMs / 60000)} min and looked stuck, so it was stopped.`));
      if (timedOut) return reject(new Error(`${label} timed out after ${Math.round(timeoutMs / 60000)} min.`));
      if (code === 0) return resolveP({ stdout: out, stderr: err });
      reject(new Error(`${label} failed (exit ${code}). ${(err || out).slice(-800)}`));
    });
  });
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export function pidAlive(pid) {
  if (!pid) return false;
  try { process.kill(pid, 0); return true; } catch (e) { return e && e.code === "EPERM"; }
}
