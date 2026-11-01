export const meta = {
  name: 'editroom-design',
  description: 'Studio design loop, pipelined per slot: a designer builds the Remotion section film (or demo overlay) from its brief, renders through the global queue, an independent checker judges it after the mechanical checks, numbered fixes go back to a reviser (max 3 rounds)',
  whenToUse: 'editroom studio mode, after the section briefs are final. Also called by continuity.js with numbered fixes for specific slots.',
  phases: [
    { title: 'Design', detail: 'one designer per slot (Opus): Scene.tsx, stills at every beat, render via the queue' },
    { title: 'Check', detail: 'one independent checker per render (Sonnet, medium): mechanical checks first, then the 12 points' },
    { title: 'Revise', detail: 'numbered fixes -> re-render -> re-check, at most max_rounds' },
  ],
}

/*
Studio port of the proof-first skill's design-visuals.js and the "design-visuals-finish" / "minor-fixes" workflows
of the good tips edit. Differences: job folders live in the work dir (<W>/studio/jobs/<KEY>), the kit and job runner
are the plugin's (studio/kit, studio/jobs/job.mjs with a global render queue), cutroom composes the films afterwards
(so nothing is placed anywhere), and checkers start from studio.py check-render's measurements.
Args from `python3 scripts/studio.py args design <W> [--only K1,K2] [--max-rounds 3]`:
  er, work, title, today, creator, studio_py, cutroom, kit, job_cmd, agents_dir, designer_brief, proof_dir,
  references {readme, jobs[]}, max_rounds, tasks [{key, title, mode, frames, slot0, end, clock, timing, brief, job_dir,
  depends_on, note, resume?: {version, render_path, check}, fixes?: [{severity, frames, problem, fix}]}],
  models? {design, check}, check_effort?
Every agent writes its result to disk (design-report.json, checks/check-v<N>.json), so `studio.py status` and the next
`studio.py args design` know what is done.
*/

const A = args || {}
if (!A.work || !Array.isArray(A.tasks)) throw new Error('design: use the args from `studio.py args design <W>`')
const M = Object.assign({ design: 'opus', check: 'sonnet' }, A.models || {})
const MAX_ROUNDS = Number.isFinite(A.max_rounds) ? A.max_rounds : 3
const CHECK_EFFORT = A.check_effort || 'medium'
const JOB = A.job_cmd
const ROLE = (r) => `${A.agents_dir}/${r}.md`
if ((A.skipped || []).length) log(`skipped (already passing, or no final brief): ${A.skipped.join(', ')}`)
if (!A.tasks.length) return { results: [], films: [], needs_attention: [], skipped: A.skipped || [] }

const keys = new Set(A.tasks.map(t => t.key))
const refs = A.references && (A.references.jobs || []).length
  ? `Reference jobs (pick the two that fit, per "${A.references.readme}"):\n${A.references.jobs.map(j => `- ${j}`).join('\n')}`
  : 'Reference jobs: none installed (studio/kit/references missing).'

const CONTEXT = `
VIDEO: ${A.creator || 'the creator'}'s YouTube video "${A.title}". Work dir W = "${A.work}".
Designer brief (the law for look, grammar, honesty, privacy): "${A.designer_brief}". Proof pack: "${A.proof_dir}".
Kit: "${A.kit}" (README.md, GUIDE.md, styles/velour/). Job runner: \`${JOB}\` (run \`${JOB} --help\` once for the exact syntax;
renders queue globally, a few at a time: never start two renders of one job).
studio.py = "${A.studio_py}" (\`check-render "${A.work}" <KEY> --render <render> --json\`, \`sheet-video <render> --every 2 -o <png>\`,
\`boxes <png> --phrase "<words>" --json\`). cutroom = "${A.cutroom}" (\`still "${A.work}" --at o:<seconds> -o <png>\`: the composed
frame with his footage, for FRONT slots).
${refs}`

const slot = (t) => `SLOT ${t.key} (${t.mode.toUpperCase()}): "${t.title}", ${t.clock}. Job folder: "${t.job_dir}" (job.json: slot0 ${t.slot0}
output ms, frames ${t.frames} AUTHORITATIVE). Section brief (binding): "${t.brief}". Timing file: "${t.timing}".${t.depends_on ? `
Shares elements with slot ${t.depends_on}: import/copy its component from that job's Scene.tsx (same folder layout) so they look IDENTICAL.` : ''}`

const FRONT = `FRONT SLOT: a TRANSPARENT ProRes 4444 overlay over his real screen recording. Final-frame pixels, inside the panel
(x 19-1493, y 19-1061), fully transparent elsewhere and nothing at x >= 1500. Before placing a card, look at his footage at that
output time (\`cutroom still\`) and put it in an EMPTY area, timed to when that area is free; never over the UI he is talking about
or a blurred region. Small cards, short holds (~3-4 s), rings/arrows on the real UI at the word.`

const RESULT = {
  type: 'object',
  properties: {
    key: { type: 'string' }, version: { type: 'number' }, render_path: { type: 'string' }, frames: { type: 'number' }, sheet: { type: 'string' },
    beats: { type: 'array', items: { type: 'object', properties: { frame: { type: 'number' }, word_id: { type: 'string' }, word: { type: 'string' }, shows: { type: 'string' }, proof_file: { type: 'string' } }, required: ['frame', 'shows'] } },
    numbers_on_screen: { type: 'array', items: { type: 'string' } },
    mismatches: { type: 'array', items: { type: 'string' } },
    problems: { type: 'string' },
  },
  required: ['key', 'version', 'render_path', 'frames', 'beats'],
}
const CHECK = {
  type: 'object',
  properties: {
    key: { type: 'string' }, version: { type: 'number' }, render: { type: 'string' }, pass: { type: 'boolean' },
    issues: { type: 'array', items: { type: 'object', properties: { severity: { type: 'string', enum: ['blocking', 'minor'] }, frames: { type: 'string' }, problem: { type: 'string' }, fix: { type: 'string' } }, required: ['severity', 'problem'] } },
    evidence: { type: 'array', items: { type: 'string' } },
  },
  required: ['pass', 'issues'],
}
const passed = (c) => !!c && (c.pass === true || !(c.issues || []).some(i => i.severity === 'blocking'))
const numbered = (issues) => issues.map((i, n) => `${n + 1}. [${i.severity}] ${i.frames ? `${i.frames}: ` : ''}${i.problem}${i.fix ? ` FIX: ${i.fix}` : ''}`).join('\n')

const designPrompt = (t, dep) => `${CONTEXT}

Read your role instructions first: "${ROLE('designer')}".
${slot(t)}
${t.mode === 'front' ? FRONT : ''}${dep && dep.result ? `\nThe ${t.depends_on} design is done: ${dep.result.render_path} (job "${dep.task.job_dir}").` : ''}
Build the film, render it, run the mechanical checks, write design-report.json and return it.`

const checkPrompt = (t, r, round) => `${CONTEXT}

Read your role instructions first: "${ROLE('checker')}". Your verdict file: "${t.job_dir}/checks/check-v${r.version}.json".
WRITE the verdict file (the same JSON you return, plus "key" and "version") BEFORE you return, then confirm it with
\`python3 -c "import json;json.load(open('<file>'))"\`. A verdict that is only returned is lost on resume.
${slot(t)}
${t.mode === 'front' ? FRONT : ''}
Render to check: "${r.render_path}" (version ${r.version}). Designer report:
${JSON.stringify(r, null, 1).slice(0, 12000)}${round > 0 ? `\nThis is revision round ${round}: also check that every listed fix is done and nothing else changed.` : ''}`

const revisePrompt = (t, r, issues, round, source) => `${CONTEXT}

Read your role instructions first: "${ROLE('designer')}" (Revisions section).
${slot(t)}
${t.mode === 'front' ? FRONT : ''}
REVISION ROUND ${round} (fixes from ${source}): the current render is v${r.version} ("${r.render_path}"). Fix every BLOCKING issue and the
minor ones that are cheap and safe; keep every other beat, its timing and the frame count (${t.frames}). Render v${Number(r.version) + 1},
confirm each fix with stills at the listed frames, compare 12-frame sheets of v${r.version} and the new render, rewrite design-report.json.
ISSUES:
${numbered(issues)}`

const gate = {}
const release = {}
for (const t of A.tasks) gate[t.key] = new Promise(resolve => { release[t.key] = resolve })

phase('Design')
const out = await pipeline(
  A.tasks,
  async (t) => {
    const dep = t.depends_on && keys.has(t.depends_on) ? await gate[t.depends_on] : null
    let r = null
    try {
      if (t.resume && t.resume.render_path) {
        r = { key: t.key, version: t.resume.version, render_path: t.resume.render_path, frames: t.frames, beats: [], resumed: true }
        log(`${t.key}: resuming from v${t.resume.version}`)
      } else {
        r = await agent(designPrompt(t, dep), { label: `design:${t.key}`, phase: 'Design', schema: RESULT, model: M.design })
      }
    } finally {
      release[t.key]({ task: t, result: r })
    }
    return r
  },
  async (r, t) => {
    const rec = { key: t.key, task: t, final: r, rounds: 0, history: [], check: null }
    if (!r) {
      rec.check = { pass: false, issues: [{ severity: 'blocking', problem: 'designer returned no result' }] }
      return rec
    }
    let chk = null
    if (Array.isArray(t.fixes) && t.fixes.length) {
      // continuity (or a human) sent numbered fixes: revise first, then check
      rec.rounds += 1
      const next = await agent(revisePrompt(t, r, t.fixes, rec.rounds, 'the continuity review'), { label: `revise:${t.key}#c`, phase: 'Revise', schema: RESULT, model: M.design })
      if (next) rec.final = next
      chk = await agent(checkPrompt(t, rec.final, rec.rounds), { label: `check:${t.key}#c`, phase: 'Revise', schema: CHECK, model: M.check, effort: CHECK_EFFORT })
    } else if (t.resume && t.resume.check && Number(t.resume.check.version) === Number(r.version)) {
      chk = t.resume.check
    } else {
      chk = await agent(checkPrompt(t, r, 0), { label: `check:${t.key}`, phase: 'Check', schema: CHECK, model: M.check, effort: CHECK_EFFORT })
    }
    while (chk && !passed(chk) && rec.rounds < MAX_ROUNDS) {
      rec.rounds += 1
      rec.history.push({ version: rec.final.version, issues: chk.issues })
      const next = await agent(revisePrompt(t, rec.final, chk.issues, rec.rounds, 'the independent checker'), { label: `revise:${t.key}#${rec.rounds}`, phase: 'Revise', schema: RESULT, model: M.design })
      if (!next) break
      rec.final = next
      chk = await agent(checkPrompt(t, next, rec.rounds), { label: `check:${t.key}#${rec.rounds}`, phase: 'Revise', schema: CHECK, model: M.check, effort: CHECK_EFFORT })
    }
    rec.check = chk || { pass: false, issues: [{ severity: 'blocking', problem: 'checker returned nothing: rerun the design workflow (it resumes at the check)' }] }
    return rec
  },
)

const recs = out.map((rec, i) => rec || { key: A.tasks[i].key, task: A.tasks[i], final: null, rounds: 0, history: [], check: { pass: false, issues: [{ severity: 'blocking', problem: 'pipeline stage failed' }] } })
const results = recs.map(rec => ({
  key: rec.key, mode: rec.task.mode, clock: rec.task.clock, frames: rec.task.frames,
  render_path: rec.final ? rec.final.render_path : null, version: rec.final ? rec.final.version : null,
  pass: passed(rec.check), rounds: rec.rounds,
  open_issues: (rec.check.issues || []).filter(i => i.severity === 'blocking'),
  minor_issues: (rec.check.issues || []).filter(i => i.severity === 'minor').length,
  mismatches: rec.final ? rec.final.mismatches || [] : [],
}))
const needs = results.filter(r => !(r.pass && r.render_path)).map(r => r.key)
if (needs.length) log(`not passing after ${MAX_ROUNDS} rounds (see open_issues; rerun with --only to try again): ${needs.join(', ')}`)
return { results, films: results.filter(r => r.pass && r.render_path).map(r => ({ key: r.key, mode: r.mode, render: r.render_path, version: r.version })), needs_attention: needs, skipped: A.skipped || [] }
