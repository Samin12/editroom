export const meta = {
  name: 'editroom-continuity',
  description: 'Studio continuity: one reviewer over every render in timeline order, every join and every chapter-card boundary; slots with blocking issues go back through the design loop with numbered fixes',
  whenToUse: 'editroom studio mode, after design.js passed every slot (or as many as it could): before wiring the films into the edit.',
  phases: [
    { title: 'Continuity', detail: 'one reviewer (Opus) across all renders, joins and cards' },
    { title: 'Fix', detail: 'design.js revision loop for the slots with blocking issues (nested workflow)' },
  ],
}

/*
Studio port of the continuity pass of the good tips edit ("tips-t7b-check-and-continuity"): it caught a misplaced
TIP 5 / TIP 6 opener there. Args from `python3 scripts/studio.py args continuity <W>`:
  er, work, title, creator, studio_py, cutroom, kit, job_cmd, agents_dir, designer_brief, proof_dir, references,
  renders [{key, title, mode, frames, slot0, end, clock, timing, brief, job_dir, depends_on, render, version, pass}],
  cards [{n, title, card: [start_ms, end_ms]}], out (receipt path), design_script, fix_minor? (false), max_rounds? (2)
*/

const A = args || {}
if (!A.work || !Array.isArray(A.renders)) throw new Error('continuity: use the args from `studio.py args continuity <W>`')
const M = Object.assign({ continuity: 'opus' }, A.models || {})
const ROLE = (r) => `${A.agents_dir}/${r}.md`
if (A.renders.length < 2) return { issues: [], fixed: [], note: 'fewer than two renders: nothing to compare' }

const CONT = {
  type: 'object',
  properties: {
    issues: { type: 'array', items: { type: 'object', properties: { key: { type: 'string' }, severity: { type: 'string', enum: ['blocking', 'minor'] }, frames: { type: 'string' }, problem: { type: 'string' }, fix: { type: 'string' } }, required: ['key', 'severity', 'problem', 'fix'] } },
    summary: { type: 'string' },
  },
  required: ['issues', 'summary'],
}

phase('Continuity')
const order = [...A.renders].sort((a, b) => a.slot0 - b.slot0)
const cont = await agent(`Read your role instructions first: "${ROLE('continuity')}". Receipt to write: "${A.out}".
VIDEO: ${A.creator || 'the creator'}'s YouTube video "${A.title}". Work dir W = "${A.work}". Designer brief (shared conventions):
"${A.designer_brief}". studio.py = "${A.studio_py}", cutroom = "${A.cutroom}".
Renders in timeline order (key | mode | output clock | slot0 ms | frames | render | checker pass):
${order.map(r => `${r.key} | ${r.mode} | ${r.clock} | ${r.slot0} | ${r.frames} | ${r.render} | ${r.pass ? 'pass' : 'NOT PASSING'}`).join('\n')}
BEHIND renders are full opaque frames with their own canvas and panel (camera tile and badge area left clear for cutroom);
FRONT renders are transparent overlays over his screen recording (judge them composited: \`cutroom still\` at the same time).
Chapter cards (cutroom draws them; check the frames just before and after each): ${JSON.stringify(A.cards || [])}`,
{ label: 'continuity', phase: 'Continuity', schema: CONT, model: M.continuity })

const issues = (cont && cont.issues) || []
const want = issues.filter(i => i.severity === 'blocking' || A.fix_minor)
const byKey = {}
for (const i of want) (byKey[i.key] = byKey[i.key] || []).push(i)
const tasks = order.filter(r => byKey[r.key]).map(r => ({
  key: r.key, title: r.title, mode: r.mode, frames: r.frames, slot0: r.slot0, end: r.end, clock: r.clock, timing: r.timing,
  brief: r.brief, job_dir: r.job_dir, depends_on: r.depends_on,
  resume: { version: r.version, render_path: r.render }, fixes: byKey[r.key],
}))
const unknown = Object.keys(byKey).filter(k => !order.find(r => r.key === k))
if (unknown.length) log(`continuity named unknown slot keys (ignored): ${unknown.join(', ')}`)

let fixed = null
if (tasks.length) {
  phase('Fix')
  log(`sending ${tasks.length} slot(s) back through design with continuity fixes: ${tasks.map(t => t.key).join(', ')}`)
  fixed = await workflow({ scriptPath: A.design_script }, {
    er: A.er, work: A.work, title: A.title, creator: A.creator, studio_py: A.studio_py, cutroom: A.cutroom, kit: A.kit,
    job_cmd: A.job_cmd, agents_dir: A.agents_dir, designer_brief: A.designer_brief, proof_dir: A.proof_dir,
    references: A.references, max_rounds: Number.isFinite(A.max_rounds) ? A.max_rounds : 2, tasks, models: A.models,
  })
}
return { summary: cont ? cont.summary : 'continuity reviewer returned nothing', issues, fixed }
