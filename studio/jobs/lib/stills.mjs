#!/usr/bin/env node
// Many stills of one job from ONE bundle and ONE browser (the Remotion CLI's `still` bundles per
// call, ~15 s each). Run with the workspace as cwd:
//   node stills.mjs <kit> <entry> <compositionId> <outDir> --frames 0,30,90 [--scale 0.5] [--prefix f]
// Prints {ok, stills:[{frame, path}], bundleMs, renderMs}. Never takes a render-queue slot.
import { createRequire } from "node:module";
import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

const [kit, entry, id, outDir, ...rest] = process.argv.slice(2);
const opt = {};
for (let i = 0; i < rest.length; i += 2) opt[rest[i].replace(/^--/, "")] = rest[i + 1];
const frames = String(opt.frames || "0").split(",").map((s) => parseInt(s, 10)).filter((n) => Number.isFinite(n) && n >= 0);
const scale = Number(opt.scale) > 0 ? Number(opt.scale) : 1;
const prefix = opt.prefix || "f";

const req = createRequire(join(resolve(kit), "package.json"));
const { bundle } = req("@remotion/bundler");
const { selectComposition, renderStill, openBrowser } = req("@remotion/renderer");

const t0 = Date.now();
const serveUrl = await bundle({ entryPoint: resolve(kit, entry), onProgress: () => {} });
const bundleMs = Date.now() - t0;
const browser = await openBrowser("chrome");
try {
  const inputProps = { final: true };
  const composition = await selectComposition({ serveUrl, id, inputProps, puppeteerInstance: browser });
  mkdirSync(outDir, { recursive: true });
  const stills = [];
  const t1 = Date.now();
  for (const f of frames) {
    const frame = Math.min(f, composition.durationInFrames - 1);
    const path = join(outDir, `${prefix}${String(frame).padStart(5, "0")}.png`);
    await renderStill({ composition, serveUrl, output: path, frame, imageFormat: "png", scale, inputProps, puppeteerInstance: browser, overwrite: true });
    stills.push({ frame, path });
  }
  process.stdout.write(JSON.stringify({ ok: true, stills, bundleMs, renderMs: Date.now() - t1, durationInFrames: composition.durationInFrames }) + "\n");
} finally {
  await browser.close({ silent: true }).catch(() => {});
}
