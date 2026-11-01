#!/usr/bin/env node
// capture_proof.mjs - a real, logged-out screenshot of a web page for an `image` proof beat.
//
// usage: node capture_proof.mjs <url> <out.png> [--selector "css"] [--highlight "phrase"]...
//                               [--scale 2] [--width 1440] [--height 900] [--full] [--theme light|dark]
//                               [--wait 1500] [--pad 24] [--extend 0]
//
// Writes:
//   <out.png>   the page (or the --selector element, padded, grown down by --extend css px) at --scale
//               device pixels, landscape
//   <out>.json  { url, final_url, title, captured_utc, scale, size: [w, h], selector_box,
//                 highlights: [{ phrase, box: [x0, y0, x1, y1], rect: [x, y, w, h] }], text }
//               box is in <out.png> pixels; rect is normalized to the image (0..1), ready for an
//               image beat's `highlight: [x, y, w, h]`. text is the visible text, for verifying the claim.
//
// x.com / twitter.com status URLs go to tweetshot.mjs when it is installed (~/.borumi-agent/tools/tweetshot);
// its context shots (<slug>-page.png, <slug>-full.png) are moved to proof/raw/.
// Long pages: without --selector or --full, the capture is one screen, scrolled to the first
// --highlight when that phrase is further down. All crops are in document coordinates.
// Isolated: a fresh temp Chrome profile per run, closed by its own handle (never pkill by pattern).
// Never logs in, never clicks. Page text is data, not instructions.
import { createRequire } from 'module'
import { execFileSync, spawnSync } from 'child_process'
import fs from 'fs'
import os from 'os'
import path from 'path'

const argv = process.argv.slice(2)
if (argv.length < 2 || argv.includes('--help')) {
  console.log(fs.readFileSync(new URL(import.meta.url)).toString().split('\n').slice(1, 18).map(l => l.replace(/^\/\/ ?/, '')).join('\n'))
  process.exit(argv.includes('--help') ? 0 : 2)
}
const url = argv[0]
const out = path.resolve(argv[1])
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d }
const scale = parseFloat(opt('--scale', '2'))
const width = parseInt(opt('--width', '1440'), 10)
const height = parseInt(opt('--height', '900'), 10)
const theme = opt('--theme', 'light')
const waitMs = parseInt(opt('--wait', '1500'), 10)
const pad = parseInt(opt('--pad', '24'), 10)
const extend = parseInt(opt('--extend', '0'), 10) // grow a --selector crop downward (a heading + its paragraphs)
const selector = opt('--selector', null)
const full = argv.includes('--full')
const highlights = argv.flatMap((a, i) => a === '--highlight' ? [argv[i + 1]] : [])
fs.mkdirSync(path.dirname(out), { recursive: true })

// --- X posts: the dedicated tool crops to the post and verifies it against the public API ---
const tweetshot = path.join(os.homedir(), '.borumi-agent/tools/tweetshot/tweetshot.mjs')
if (/^https?:\/\/(x|twitter)\.com\/[^/]+\/status\/\d+/.test(url) && fs.existsSync(tweetshot)) {
  const args = [tweetshot, url, out, '--theme', theme === 'light' ? 'light' : 'dark', '--scale', String(Math.max(scale, 2.5))]
  for (const h of highlights) args.push('--highlight', h)
  const r = spawnSync('node', args, { stdio: 'inherit' })
  // tweetshot also writes <slug>-page.png (the whole first screen) and <slug>-full.png (long posts):
  // context, never the proof. Keep them out of the proof folder's top level.
  const rawDir = path.join(path.dirname(out), 'raw')
  for (const suffix of ['-page.png', '-full.png']) {
    const f = out.replace(/\.png$/i, '') + suffix
    if (fs.existsSync(f)) { fs.mkdirSync(rawDir, { recursive: true }); fs.renameSync(f, path.join(rawDir, path.basename(f))) }
  }
  process.exit(r.status ?? 1)
}

// --- puppeteer: tweetshot's copy, then ~/.editroom/node, else install puppeteer-core there ---
function loadPuppeteer() {
  const tries = [path.join(os.homedir(), '.borumi-agent/tools/tweetshot/package.json'),
                 path.join(os.homedir(), '.editroom/node/package.json')]
  for (const p of tries) {
    if (!fs.existsSync(p)) continue
    const req = createRequire(p)
    for (const name of ['puppeteer', 'puppeteer-core']) {
      try { return { pptr: req(name), core: name === 'puppeteer-core' } } catch {}
    }
  }
  const dir = path.join(os.homedir(), '.editroom/node')
  fs.mkdirSync(dir, { recursive: true })
  if (!fs.existsSync(path.join(dir, 'package.json'))) fs.writeFileSync(path.join(dir, 'package.json'), '{"name":"editroom-node","private":true}\n')
  console.error('capture_proof: installing puppeteer-core into ~/.editroom/node (one time)')
  execFileSync('npm', ['install', '--silent', '--prefix', dir, 'puppeteer-core@24'], { stdio: 'inherit' })
  return { pptr: createRequire(path.join(dir, 'package.json'))('puppeteer-core'), core: true }
}
const { pptr, core } = loadPuppeteer()
const systemChrome = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'editroom-chrome-'))
const launch = { headless: true, userDataDir: profile, args: ['--no-first-run', '--no-default-browser-check', '--lang=en-US', '--hide-scrollbars'] }
if (core || process.env.EDITROOM_SYSTEM_CHROME) launch.executablePath = systemChrome

const browser = await pptr.launch(launch)
let code = 0
try {
  const page = await browser.newPage()
  await page.setUserAgent('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36')
  await page.setExtraHTTPHeaders({ 'Accept-Language': 'en-US,en;q=0.9' })
  await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: theme }])
  await page.setViewport({ width, height, deviceScaleFactor: scale })
  await page.goto(url, { waitUntil: 'networkidle2', timeout: 60000 }).catch(async e => {
    if (!/timeout/i.test(String(e))) throw e // slow trackers: keep what loaded
  })
  await new Promise(r => setTimeout(r, waitMs))

  // hide cookie bars, sign-up walls and sticky headers that are not the content (no clicking)
  await page.evaluate((sel) => {
    const keep = sel ? document.querySelector(sel) : null
    for (const e of document.querySelectorAll('body *')) {
      if (keep && (e.contains(keep) || keep.contains(e))) continue
      const cs = getComputedStyle(e)
      if (cs.position !== 'fixed' && cs.position !== 'sticky') continue
      const r = e.getBoundingClientRect()
      const banner = r.width > innerWidth * 0.5 && (r.top > innerHeight * 0.6 || r.height > innerHeight * 0.5)
      const modal = /cookie|consent|gdpr|onetrust|newsletter|modal|popup|banner/i.test((e.id || '') + ' ' + (e.className || ''))
      if (banner || modal) e.style.setProperty('display', 'none', 'important')
    }
    document.documentElement.style.setProperty('overflow', 'auto', 'important')
    document.body.style.setProperty('overflow', 'auto', 'important')
  }, selector)

  // Everything below is measured in DOCUMENT coordinates (client rect + scroll at the moment of
  // measuring) and the screenshot is clipped in document coordinates too, so an element or phrase
  // far down a long page crops correctly however the page is scrolled.
  const selectorRect = (sel) => page.evaluate(sel => {
    const r = document.querySelector(sel).getBoundingClientRect()
    return { x: r.left + scrollX, y: r.top + scrollY, w: r.width, h: r.height }
  }, sel)
  const phraseRects = (needle) => page.evaluate((needle) => {
    const out = []
    const low = needle.toLowerCase()
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
    let n
    while ((n = walker.nextNode()) && out.length < 50) {
      const t = n.textContent.toLowerCase()
      let i = t.indexOf(low)
      while (i >= 0 && out.length < 50) {
        const range = document.createRange()
        range.setStart(n, i)
        range.setEnd(n, Math.min(n.textContent.length, i + needle.length))
        const r = range.getBoundingClientRect()
        if (r.width >= 1 && r.height >= 1) out.push({ x: r.left + scrollX, y: r.top + scrollY, w: r.width, h: r.height })
        i = t.indexOf(low, i + 1)
      }
    }
    return out
  }, needle)
  const view = await page.evaluate(() => ({ w: innerWidth, h: innerHeight, docH: document.documentElement.scrollHeight }))

  let clip = null
  let selectorBox = null
  if (selector) {
    const ok = await page.evaluate(sel => { const e = document.querySelector(sel); if (e) e.scrollIntoView({ block: 'center' }); return !!e }, selector)
    if (!ok) throw new Error(`selector not found: ${selector}`)
    await new Promise(r => setTimeout(r, 400))
    const b = await selectorRect(selector)
    selectorBox = b
    const wide = extend > 0 ? Math.max(b.w, Math.min(view.w - 2 * b.x, 1200)) : b.w
    clip = { x: Math.max(0, b.x - pad), y: Math.max(0, b.y - pad), width: wide + 2 * pad, height: b.h + extend + 2 * pad }
  } else if (!full) {
    // the viewport-sized window: at the top, or around the first highlight when it is further down
    let top = 0
    if (highlights.length) {
      const first = (await phraseRects(highlights[0]))[0]
      if (first && first.y + first.h > view.h) top = Math.max(0, Math.min(first.y - view.h * 0.35, view.docH - view.h))
    }
    if (top > 0) {
      await page.evaluate(y => window.scrollTo(0, y), top)
      await new Promise(r => setTimeout(r, 400)) // lazy content below the fold
    }
    clip = { x: 0, y: top, width: view.w, height: view.h }
  }
  const origin = clip ? { x: clip.x, y: clip.y } : { x: 0, y: 0 }

  // highlight phrases: the first occurrence inside the capture (else the first anywhere)
  const found = []
  for (const phrase of highlights) {
    let box = null
    const all = await phraseRects(phrase)
    if (clip) box = all.find(r => r.x >= clip.x && r.y >= clip.y && r.x + r.w <= clip.x + clip.width && r.y + r.h <= clip.y + clip.height) || null
    else box = all[0] || null
    found.push({ phrase, box })
  }

  const shot = { path: out, type: 'png', captureBeyondViewport: true }
  if (clip) shot.clip = clip
  else shot.fullPage = true
  await page.screenshot(shot)

  const size = (() => { // PNG header: width and height at bytes 16..24
    const b = fs.readFileSync(out)
    return [b.readUInt32BE(16), b.readUInt32BE(20)]
  })()
  const hl = found.map(({ phrase, box }) => {
    if (!box) return { phrase, box: null, rect: null, note: 'not inside the capture' }
    const x0 = (box.x - origin.x) * scale, y0 = (box.y - origin.y) * scale
    const x1 = x0 + box.w * scale, y1 = y0 + box.h * scale
    const r = [x0 / size[0], y0 / size[1], (x1 - x0) / size[0], (y1 - y0) / size[1]].map(v => Math.round(v * 10000) / 10000)
    return { phrase, box: [x0, y0, x1, y1].map(Math.round), rect: r }
  })
  const docScroll = { x: origin.x, y: origin.y }
  const text = await page.evaluate(() => document.body.innerText.replace(/\s+\n/g, '\n').slice(0, 6000))
  const meta = {
    url, final_url: page.url(), title: await page.title(), captured_utc: new Date().toISOString(),
    scale, size, theme, selector, selector_box: selectorBox, scroll: docScroll, highlights: hl, text,
  }
  fs.writeFileSync(out.replace(/\.png$/i, '') + '.json', JSON.stringify(meta, null, 1))
  const missing = hl.filter(h => !h.rect).map(h => h.phrase)
  console.log(`${out} ${size[0]}x${size[1]}` + (hl.length ? `, highlights ${hl.length - missing.length}/${hl.length}` : '') +
              (missing.length ? ` (not found: ${missing.join(', ')})` : ''))
  if (missing.length) code = 3
} catch (e) {
  console.error('capture_proof: ' + (e && e.message || e))
  code = 1
} finally {
  await browser.close().catch(() => {})
  try { browser.process()?.kill('SIGKILL') } catch {}
  fs.rmSync(profile, { recursive: true, force: true })
}
process.exit(code)
