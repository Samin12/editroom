export const meta = {
  name: 'editroom-section-briefs',
  description: 'Studio section briefs: one beat-by-beat brief per slot (all slots at once) from exact word timings, the proof pack and measured glyph boxes; an independent critic per brief; a revision where the critic found problems',
  whenToUse: 'editroom studio mode, after plan-sections resolved the slots and the proof pack is assembled: before design.',
  phases: [
    { title: 'Write', detail: 'one brief writer per slot (Opus)' },
    { title: 'Critique', detail: 'one independent critic per brief (Opus)' },
    { title: 'Revise', detail: 'apply the numbered fixes (only where the critic did not pass the brief)' },
    { title: 'Design', detail: 'optional (args.design): each final brief goes straight into its own design loop (nested design.js)' },
  ],
}

/*
Studio port of the "tips-section-briefs" workflow of the good tips edit (write -> critic -> revise), with the slow part
removed: writers get exact word frames (timing files) and glyph boxes (studio.py boxes) instead of measuring by hand.
Args from `python3 scripts/studio.py args briefs <W>`:
  er, work, title, today, creator, studio_py, cutroom, agents_dir, designer_brief, proof_dir, template, kit,
  references {readme, jobs[]}, tasks [{key, title, mode, chapter, group, from, to, slot0, end, frames, clock, timing,
  brief, job_dir, depends_on, note, neighbours[]}], skipped_final[], models? {write, critic},
  design? (from `studio.py args briefs <W> --chain-design`: the design.js args minus tasks)
Resumable: slots whose brief is final (critic passed or revised) are not in `tasks`.
With args.design, each slot continues into design.js the moment its brief is final, so the slowest brief no longer
holds back every design (one nested design run per slot; depends_on is not enforced across those runs, so a slot that
depends on another waits for this workflow's end: run `studio.py args design` afterwards for those).
*/

const A = args || {}
if (!A.work || !Array.isArray(A.tasks)) throw new Error('section-briefs: use the args from `studio.py args briefs <W>`')
const M = Object.assign({ write: 'opus', critic: 'opus' }, A.models || {})
const ROLE = (r) => `${A.agents_dir}/${r}.md`
if ((A.skipped_final || []).length) log(`already final (skipped): ${A.skipped_final.join(', ')}`)
if (!A.tasks.length) return { briefs: [], skipped_final: A.skipped_final || [] }

const refs = A.references && (A.references.jobs || []).length
  ? `Reference jobs (read "${A.references.readme}" to pick the two that fit this slot; learn structure, not content):\n${A.references.jobs.map(j => `- ${j}`).join('\n')}`
  : 'Reference jobs: none installed yet (studio/kit/references missing); rely on the designer brief and the template.'

const CONTEXT = `
VIDEO: ${A.creator || 'the creator'}'s YouTube video "${A.title}". Today is ${A.today}. Work dir W = "${A.work}".
Designer brief (read it fully first; never contradict it): "${A.designer_brief}".
Section brief template (follow its structure): "${A.template}".
Proof pack: "${A.proof_dir}" (README.md "Fix before publishing" table, proof-index.json). Pack times are pre-cut: match by WORDS.
Tools: studio.py = "${A.studio_py}" (\`boxes <png> --phrase "<words>" --json\`, \`timing "${A.work}" --slot <KEY>\`);
cutroom = "${A.cutroom}" (\`still "${A.work}" --at o:<seconds> -o <png>\` shows the composed frame with his footage).
Kit (component names and props): "${A.kit}/README.md".
${refs}`

const task = (t) => `SLOT ${t.key} (${t.mode.toUpperCase()}): "${t.title}", ${t.clock}, slot0 ${t.slot0} output ms, ${t.frames} frames.
Timing file (the ONLY source of times and frames): "${t.timing}". Write the brief to: "${t.brief}" (assets to "${t.brief.replace(/\.md$/, '')}-assets/").
Job folder (job.json): "${t.job_dir}". Planner's note: ${t.note || '-'}.${t.depends_on ? `\nShares elements with the earlier slot ${t.depends_on}: read its brief (same folder) and match it.` : ''}
Neighbour slots (read their briefs if they exist, for the shared look and the hand-off at the joins): ${(t.neighbours || []).join(', ') || '-'}.`

const BRIEF = {
  type: 'object',
  properties: {
    key: { type: 'string' }, file: { type: 'string' }, beats: { type: 'number' },
    proof_files: { type: 'array', items: { type: 'string' } },
    honest_notes: { type: 'array', items: { type: 'string' } }, open_issues: { type: 'array', items: { type: 'string' } },
  },
  required: ['key', 'file', 'proof_files'],
}
const CRIT = {
  type: 'object',
  properties: { key: { type: 'string' }, pass: { type: 'boolean' }, fixes: { type: 'array', items: { type: 'string' } } },
  required: ['key', 'pass', 'fixes'],
}

const res = await pipeline(
  A.tasks,
  (t) => agent(`${CONTEXT}

Read your role instructions first: "${ROLE('brief-writer')}".
${task(t)}`, { label: `write:${t.key}`, phase: 'Write', schema: BRIEF, model: M.write }),
  (b, t) => !b ? null : agent(`${CONTEXT}

Read your role instructions first: "${ROLE('brief-critic')}". Your verdict file: "${t.brief.replace(/\.md$/, '.critic.json')}".
${task(t)}
The writer's report: ${JSON.stringify(b).slice(0, 4000)}`, { label: `critic:${t.key}`, phase: 'Critique', schema: CRIT, model: M.critic })
    .then(c => ({ b, c })),
  (bc, t) => {
    if (!bc) return null
    const { b, c } = bc
    if (!c || (c.pass && !(c.fixes || []).length)) return { ...b, critic: c, revised: false }
    return agent(`${CONTEXT}

Read your role instructions first: "${ROLE('brief-writer')}". REVISE the brief "${t.brief}".
${task(t)}
The independent critic's fixes (verify each against the timing file, the proof files and the tools; if a fix is wrong,
keep the brief and say why in OPEN ISSUES):
${(c.fixes || []).map((f, i) => `${i + 1}. ${f}`).join('\n')}
Rewrite the file, then write "${t.brief.replace(/\.md$/, '.revised.json')}" ({"key", "applied": [...], "rejected": [...]})
and return the JSON.`, { label: `revise:${t.key}`, phase: 'Revise', schema: BRIEF, model: M.write })
      .then(r => ({ ...(r || b), critic: c, revised: !!r }))
  },
  (r, t) => {
    // chained design: this slot's brief is final, design it now (other slots may still be writing)
    if (!r || !A.design || !A.design.design_script || t.depends_on) return r
    return workflow({ scriptPath: A.design.design_script }, { ...A.design, tasks: [{ ...t, neighbours: undefined }] })
      .then(d => ({ ...r, design: d }))
      .catch(e => ({ ...r, design: { error: String(e && e.message || e) } }))
  },
)

const held = A.design ? A.tasks.filter(t => t.depends_on).map(t => t.key) : []
if (held.length) log(`not chained (they depend on another slot's design; run design afterwards): ${held.join(', ')}`)

const out = res.map((r, i) => r ? {
  key: r.key || A.tasks[i].key, file: r.file, beats: r.beats, proof_files: (r.proof_files || []).length,
  open_issues: r.open_issues || [], critic_pass: !!(r.critic && r.critic.pass), critic_fixes: r.critic ? (r.critic.fixes || []).length : null, revised: r.revised,
} : { key: A.tasks[i].key, failed: true })
const failed = out.filter(o => o.failed).map(o => o.key)
if (failed.length) log(`no brief for: ${failed.join(', ')} (rerun the workflow: final briefs are skipped)`)
const designs = res.filter(r => r && r.design).map(r => r.design)
const designed = A.design ? {
  films: designs.flatMap(d => d.films || []),
  needs_attention: designs.flatMap(d => d.needs_attention || []),
  errors: designs.filter(d => d.error).map(d => d.error),
  not_chained: held,
} : null
return { briefs: out, failed, skipped_final: A.skipped_final || [], designed }
