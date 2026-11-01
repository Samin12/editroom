#!/usr/bin/env node
// tweetshot.mjs - a real, logged-out screenshot of one X post (x.com status page), cropped to the post, hi-res.
//
// usage: node tweetshot.mjs <x.com status url> <out.png> [--theme dark|light] [--scale 2.5] [--pad 20]
//                           [--highlight "phrase"]... [--max-height 1300] [--no-verify]
//
// Writes:
//   <out.png>           the post as X renders it (avatar, name, badge, text, media, time, views, action bar)
//   <out>.json          { url, id, handle, captured_utc, scale, size, boxes: {post, text?, media?, time_views?,
//                         highlights: [{phrase, box}]}, page: {...text the page shows...}, fx: {...api.fxtwitter.com...} }
//                       every box is [x0, y0, x1, y1] in <out.png> pixels, so a designer can draw an exact highlight.
//   <out>-page.png      the whole first screen (context; never the best visual)
//   <out>-full.png      only when the post is taller than --max-height css px (long posts, X Articles): the whole
//                       post, for a slow scroll; <out.png> is then its top --max-height px (+ boxes inside it)
//
// Logged out, read-only: never logs in, never clicks anything except "Show more" inside the post.
import { createRequire } from 'module'
import { execFileSync } from 'child_process'
import fs from 'fs'
const require = createRequire(import.meta.url)
const puppeteer = require('puppeteer')

const argv = process.argv.slice(2)
if (argv.length < 2 || argv.includes('--help')) {
  console.log(fs.readFileSync(new URL(import.meta.url)).toString().split('\n').slice(1, 14).map(l => l.replace(/^\/\/ ?/, '')).join('\n'))
  process.exit(argv.includes('--help') ? 0 : 2)
}
const url = argv[0].replace('twitter.com', 'x.com').split('?')[0]
const out = argv[1]
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d }
const theme = opt('--theme', 'dark')
const scale = parseFloat(opt('--scale', '2.5'))
const pad = parseInt(opt('--pad', '20'), 10)
const maxH = parseInt(opt('--max-height', '1300'), 10)
const highlights = argv.flatMap((a, i) => a === '--highlight' ? [argv[i + 1]] : [])
const m = url.match(/x\.com\/([^/]+)\/status\/(\d+)/)
if (!m) { console.error('not an x.com status url: ' + url); process.exit(2) }
const [, handle, id] = m

let fx = null
if (!argv.includes('--no-verify')) {
  try { fx = JSON.parse(execFileSync('curl', ['-s', '--max-time', '20', `https://api.fxtwitter.com/${handle}/status/${id}`]).toString()) } catch (e) { fx = { error: String(e) } }
}

const browser = await puppeteer.launch({ headless: 'new', args: ['--no-sandbox', '--lang=en-US'] })
try {
  const page = await browser.newPage()
  await page.setUserAgent('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36')
  await page.setExtraHTTPHeaders({ 'Accept-Language': 'en-US,en;q=0.9' })
  await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: theme }])
  await page.setViewport({ width: 1280, height: 2200, deviceScaleFactor: scale })
  await page.goto(url, { waitUntil: 'networkidle2', timeout: 60000 })
  await page.waitForSelector('article', { timeout: 30000 })
  await new Promise(r => setTimeout(r, 2000))

  // the post itself: the article that links to this status id with a full "h:mm AM · Mon d, yyyy" timestamp
  const pick = `(() => {
    const arts = [...document.querySelectorAll('article')]
    const full = /\\d{1,2}:\\d{2}\\s?(AM|PM)\\s?·/
    let a = arts.find(x => [...x.querySelectorAll('a[href*="/status/${id}"]')].some(l => full.test(l.textContent)))
    if (!a) a = arts.find(x => x.querySelector('a[href*="/status/${id}"]')) || arts[0]
    document.querySelectorAll('[data-tweetshot]').forEach(e => e.removeAttribute('data-tweetshot'))
    a.setAttribute('data-tweetshot', '1')
    return arts.indexOf(a)
  })()`
  await page.evaluate(pick)
  // expand a truncated post (only inside the post)
  const more = await page.$$('[data-tweetshot] button, [data-tweetshot] [role=button], [data-tweetshot] span')
  for (const b of more) {
    const t = (await b.evaluate(e => e.textContent || '')).trim()
    if (t === 'Show more') { await b.click().catch(() => {}); await new Promise(r => setTimeout(r, 1200)); break }
  }
  await page.evaluate(pick)
  // hide everything fixed/sticky that is not the post or its ancestors (login bars, app banners, headers)
  await page.evaluate(() => {
    const a = document.querySelector('[data-tweetshot]')
    for (const e of document.querySelectorAll('body *')) {
      if (e.contains(a) || a.contains(e)) continue
      const p = getComputedStyle(e).position
      if (p === 'fixed' || p === 'sticky') e.style.setProperty('visibility', 'hidden', 'important')
    }
  })
  await page.evaluate(() => document.querySelectorAll('video').forEach(v => { try { v.pause(); v.currentTime = 0 } catch (e) {} }))
  // Never scroll: size the viewport so the whole post sits inside it at scroll 0. Then viewport
  // and document coordinates are the same, and the boxes measured below land exactly on the
  // pixels page.screenshot() crops (puppeteer clips in document coordinates; the old scroll left
  // every box a few css px, 9-17 output px, above the real glyphs).
  await page.evaluate(() => window.scrollTo(0, 0))
  const postBottom = await page.evaluate(() => { const r = document.querySelector('[data-tweetshot]').getBoundingClientRect(); return r.bottom + window.scrollY })
  await page.setViewport({ width: 1280, height: Math.min(30000, Math.ceil(postBottom + 400)), deviceScaleFactor: scale })
  await new Promise(r => setTimeout(r, 1200))
  await page.evaluate(() => window.scrollTo(0, 0))
  await page.evaluate(() => document.querySelectorAll('video').forEach(v => { try { v.pause(); v.currentTime = 0 } catch (e) {} }))
  // wait until the layout is stable (late images shift the post)
  let last = ''
  for (let i = 0; i < 10; i++) {
    await new Promise(r => setTimeout(r, 600))
    const now = await page.evaluate(() => JSON.stringify(document.querySelector('[data-tweetshot]').getBoundingClientRect()))
    if (now === last) break
    last = now
  }

  const info = await page.evaluate((phrases) => {
    const a = document.querySelector('[data-tweetshot]')
    const sx = window.scrollX, sy = window.scrollY
    const R = e => { const r = e.getBoundingClientRect(); return [r.left + sx, r.top + sy, r.right + sx, r.bottom + sy] }
    const union = rs => rs.length ? [Math.min(...rs.map(r => r[0])), Math.min(...rs.map(r => r[1])), Math.max(...rs.map(r => r[2])), Math.max(...rs.map(r => r[3]))] : null
    // text nodes of the post, for phrase search across nodes
    const nodes = []; const w = document.createTreeWalker(a, NodeFilter.SHOW_TEXT); let n
    while ((n = w.nextNode())) if (n.textContent) nodes.push(n)
    let all = ''; const starts = []
    for (const t of nodes) { starts.push(all.length); all += t.textContent }
    const at = k => { let i = starts.length - 1; while (i > 0 && starts[i] > k) i--; return [nodes[i], k - starts[i]] }
    const hl = phrases.map(ph => {
      const k = all.toLowerCase().indexOf(ph.toLowerCase())
      if (k < 0) return { phrase: ph, box: null }
      const [sn, so] = at(k), [en, eo] = at(k + ph.length - 1)
      const rg = document.createRange(); rg.setStart(sn, so); rg.setEnd(en, eo + 1)
      return { phrase: ph, box: union([...rg.getClientRects()].map(r => [r.left + sx, r.top + sy, r.right + sx, r.bottom + sy])) }
    })
    const imgs = [...a.querySelectorAll('img, video')].filter(e => e.getBoundingClientRect().width > 150)
    const timeA = [...a.querySelectorAll('a')].find(l => /\d{1,2}:\d{2}\s?(AM|PM)\s?·/.test(l.textContent))
    let tv = null
    if (timeA) { const row = timeA.parentElement; tv = R(row) }
    const tt = a.querySelector('[data-testid="tweetText"]')
    return { post: R(a), media: union(imgs.map(R)), time_views: tv, text: tt ? R(tt) : null, highlights: hl,
             page_text: a.innerText.slice(0, 4000), bg: getComputedStyle(document.body).backgroundColor }
  }, highlights)

  const [x0, y0, x1, y1] = info.post
  const full = { x: Math.max(0, x0 - pad), y: Math.max(0, y0 - pad), width: (x1 - x0) + 2 * pad, height: (y1 - y0) + 2 * pad }
  const clip = { ...full, height: Math.min(full.height, maxH) }
  const truncated = full.height > clip.height
  await page.screenshot({ path: out, clip, captureBeyondViewport: false })
  // Chrome caps one capture at 16384 device px: past that it repeats the top. Cap the -full capture below that.
  const maxFullCss = Math.floor(16000 / scale)
  const fullClip = { ...full, height: Math.min(full.height, maxFullCss) }
  if (truncated) await page.screenshot({ path: out.replace(/\.png$/, '-full.png'), clip: fullClip, captureBeyondViewport: false })
  await page.screenshot({ path: out.replace(/\.png$/, '-page.png'), clip: { x: 0, y: Math.max(0, full.y - 60), width: 1280, height: Math.min(1100, Math.max(200, full.height + 120)) } })
  const toPx = b => {
    if (!b) return null
    const r = b.map((v, i) => Math.round((v - (i % 2 ? clip.y : clip.x)) * scale))
    return r[1] >= Math.round(clip.height * scale) ? null : [r[0], r[1], r[2], Math.min(r[3], Math.round(clip.height * scale))]
  }
  const meta = {
    url, id, handle, theme, captured_utc: new Date().toISOString(), scale, pad_css_px: pad,
    size: [Math.round(clip.width * scale), Math.round(clip.height * scale)], truncated,
    full_size: truncated ? [Math.round(fullClip.width * scale), Math.round(fullClip.height * scale)] : null,
    full_capped: truncated && fullClip.height < full.height,
    boxes: { post: toPx(info.post), text: toPx(info.text), media: toPx(info.media), time_views: toPx(info.time_views),
             highlights: info.highlights.map(h => ({ phrase: h.phrase, box: toPx(h.box) })) },
    page: { text: info.page_text, background: info.bg },
    fx: fx && fx.tweet ? { text: fx.tweet.text, views: fx.tweet.views, likes: fx.tweet.likes, retweets: fx.tweet.retweets,
             replies: fx.tweet.replies, created_at: fx.tweet.created_at, author: fx.tweet.author && { name: fx.tweet.author.name,
             screen_name: fx.tweet.author.screen_name, description: fx.tweet.author.description } } : fx,
  }
  fs.writeFileSync(out.replace(/\.png$/, '.json'), JSON.stringify(meta, null, 1))
  const missing = meta.boxes.highlights.filter(h => !h.box).map(h => h.phrase)
  console.log(JSON.stringify({ ok: true, out, size: meta.size, truncated, views: meta.fx && meta.fx.views, missing_highlights: missing }))
} catch (e) {
  console.log(JSON.stringify({ ok: false, url, error: String(e && e.message || e) }))
  process.exitCode = 1
} finally {
  await browser.close()
}
