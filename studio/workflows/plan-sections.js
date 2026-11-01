export const meta = {
  name: 'editroom-plan-sections',
  description: 'Studio structure: chapters, kicker, badge groups and slots (one section film per talking stretch, a transparent overlay per live screen demo), resolved to output frames; an independent critic checks the plan',
  whenToUse: 'editroom studio mode, after the cuts are final (and ideally after the proof pack): before section briefs.',
  phases: [
    { title: 'Plan', detail: 'the section planner writes sections-plan.json and resolves it (studio.py resolve)' },
    { title: 'Critique', detail: 'an independent critic checks coverage, modes against stills, titles and groups' },
    { title: 'Revise', detail: 'the planner applies the fixes and resolves again (only if needed)' },
  ],
}

/*
Args from `python3 scripts/studio.py args plan <W>`:
  er, work, title, today, creator, studio_py, cutroom, agents_dir, plan_path, proof_index (or null), kept_cmd,
  review_dir, resolve_cmd, designer_brief, models? {plan, critic}
Writes <W>/studio/sections-plan.json, beats/00-chapters.yaml, studio/sections.json, timing/<KEY>.json, jobs/<KEY>/job.json
(all via `studio.py resolve`), and the "This video" block of the designer brief.
*/

const A = args || {}
if (!A.work || !A.plan_path || !A.resolve_cmd) throw new Error('plan-sections: use the args from `studio.py args plan <W>`')
const M = Object.assign({ plan: 'opus', critic: 'opus' }, A.models || {})
const ROLE = (r) => `${A.agents_dir}/${r}.md`

const CONTEXT = `
VIDEO: ${A.creator || 'the creator'}'s YouTube video "${A.title}". Today is ${A.today}. Work dir W = "${A.work}".
cutroom = "${A.cutroom}" (still/sheet: \`"${A.cutroom}" still "${A.work}" --at o:<s>|w<id> -o <png>\`, \`"${A.cutroom}" sheet "${A.work}" --every 8 -o <png>\`).
Kept transcript: \`${A.kept_cmd}\`. Proof index: ${A.proof_index ? `"${A.proof_index}"` : 'not assembled yet (write the "This video" block from style.md and the transcript; leave proof-specific notes out)'}.
Plan file: "${A.plan_path}". Resolve: \`${A.resolve_cmd}\`. Scratch for sheets and stills: "${A.review_dir}".
Designer brief (the "This video" block is filled from your plan): "${A.designer_brief}".`

const SLOT = {
  type: 'object',
  properties: { key: { type: 'string' }, mode: { type: 'string' }, clock: { type: 'string' }, frames: { type: 'number' }, note: { type: 'string' } },
  required: ['key', 'mode'],
}
const PLAN = {
  type: 'object',
  properties: {
    resolved: { type: 'boolean' }, summary: { type: 'string', description: 'the resolve summary JSON as text' },
    slots: { type: 'array', items: SLOT }, chapters: { type: 'array', items: { type: 'string' } },
    judgment_calls: { type: 'array', items: { type: 'string' } },
  },
  required: ['resolved', 'slots', 'judgment_calls'],
}
const CRIT = {
  type: 'object',
  properties: { pass: { type: 'boolean' }, fixes: { type: 'array', items: { type: 'string' } }, evidence: { type: 'array', items: { type: 'string' } } },
  required: ['pass', 'fixes'],
}

phase('Plan')
let plan = await agent(`${CONTEXT}

Read your role instructions first: "${ROLE('section-planner')}". Plan the sections, write the plan file, run the resolve
command until it succeeds, and return the result.`, { label: 'plan', phase: 'Plan', schema: PLAN, model: M.plan })
if (!plan || !plan.resolved) throw new Error(`plan-sections: the planner did not resolve the plan: ${plan ? plan.summary : 'no result'}`)

phase('Critique')
const crit = await agent(`${CONTEXT}

You are an INDEPENDENT CRITIC of the section plan (don't edit anything). Read "${A.plan_path}", "${A.work}/studio/sections.json"
and the kept transcript. Check:
1. Coverage: slots cover the whole timeline in order (intro, every chapter), no gaps or overlaps; the first chapter starts
   after the hook (no chapter card on the hook); every section he announces is a chapter.
2. Modes: for every FRONT slot, look at 3 stills across it (\`cutroom still ... --at o:<s>\`): his screen must be LIVE there
   (changing with his words). For BEHIND slots that cover his screen recording, look at 2 stills: if the screen is a live
   demo he narrates step by step, it should be FRONT. Slots over ~110 s should be split at a sentence break.
3. Titles 2 to 5 words, Title Case, his words, correct spellings (style.md "Names"); kicker matches how he numbers
   sections; groups only if he groups them, named in Title Case.
4. The "This video" block in the designer brief lists the sections and locked wording correctly.
Planner's judgment calls: ${JSON.stringify(plan.judgment_calls || [])}
pass = true only if nothing would make a section film wrong. Each fix: concrete (which slot/chapter, what to change).`,
{ label: 'critic', phase: 'Critique', schema: CRIT, model: M.critic })

let revised = null
if (crit && !crit.pass && (crit.fixes || []).length) {
  phase('Revise')
  revised = await agent(`${CONTEXT}

Read your role instructions first: "${ROLE('section-planner')}". REVISE the plan in "${A.plan_path}". An independent critic's
fixes (verify each against the transcript and stills; if one is wrong, keep your plan and say why):
${crit.fixes.map((f, i) => `${i + 1}. ${f}`).join('\n')}
Rewrite the plan file, run the resolve command until it succeeds, and return the result.`, { label: 'revise', phase: 'Revise', schema: PLAN, model: M.plan })
  if (revised && revised.resolved) plan = revised
}

return { plan, critic: crit, revised: !!(revised && revised.resolved) }
