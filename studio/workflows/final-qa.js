export const meta = {
  name: 'editroom-final-qa',
  description: 'Studio final QA of the rendered video: three independent read-only reviewers in parallel (cards and joins, narrative flow and audio, a full watch-through against the proof pack and privacy rules) on top of the ffmpeg QA; flags only',
  whenToUse: 'editroom studio mode, after `cutroom render` produced the final video and qa.py ran: before importing into Borumi and reporting.',
  phases: [{ title: 'Review', detail: 'three independent reviewers in parallel (Sonnet)' }],
}

/*
Studio port of the proof-first skill's final-qa.js ("master" mode), which the good tips edit never got to run.
Args from `python3 scripts/studio.py args final-qa <W> [--video <mp4>]`:
  er, work, title, creator, studio_py, cutroom, proof_dir, proof_index, video, chapters_file, captions,
  cards [{n, title, card_s: [start, end]}], slots [{key, mode, clock, slot0_s, end_s}], scratch, qa_cmd, agents_dir
Each reviewer writes <W>/studio/receipts/final-qa-<name>.json. Nothing is modified: fixes go back through the
stage that owns them (design --only <KEY>, a cut, a blur).
*/

const A = args || {}
if (!A.video || !A.work) throw new Error('final-qa: use the args from `studio.py args final-qa <W>`')
const M = Object.assign({ qa: 'sonnet' }, A.models || {})
const R = (name) => `${A.work}/studio/receipts/final-qa-${name}.json`

const COMMON = `You are an INDEPENDENT final reviewer of ${A.creator || 'the creator'}'s finished video "${A.title}". READ-ONLY: never modify, move or
re-render anything; scratch files only under "${A.scratch}-<your name>/".
Video: "${A.video}" (1920x1080, 30 fps). YouTube chapters: "${A.chapters_file}". Captions: "${A.captions}".
Chapter cards (seconds): ${JSON.stringify(A.cards || [])}. Section films/overlays (seconds): ${JSON.stringify(A.slots || [])}.
The automated QA (ffmpeg detectors, loudness) is in "${A.work}/qa.md" (run \`${A.qa_cmd}\` if it is missing); don't repeat it, build on it.
Layout every non-card frame must show: the panel on the left (a section film, or his screen recording with an overlay), his camera tile
on the right with the step badge above it, the dark dot-grid canvas around them. Frames: \`ffmpeg -ss <s> -i "${A.video}" -frames:v 1 <png>\`
(look at each with Read) or \`python3 "${A.studio_py}" sheet-video "${A.video}" --at t1,t2,.. -o <png>\`.
Return {ok, summary, issues: [{t_s, severity: "blocker"|"major"|"minor", problem, fix}]}, and write the same JSON to your receipt file.
A fix names the owner: "design <KEY>" (a film), "cut" (cuts.yaml), "blur" (privacy), "chapters", or "audio".`

const OUT = {
  type: 'object',
  properties: {
    ok: { type: 'boolean' }, summary: { type: 'string' },
    issues: { type: 'array', items: { type: 'object', properties: { t_s: { type: 'number' }, severity: { type: 'string', enum: ['blocker', 'major', 'minor'] }, problem: { type: 'string' }, fix: { type: 'string' } }, required: ['severity', 'problem'] } },
  },
  required: ['ok', 'summary', 'issues'],
}

phase('Review')
const [joins, flow, watch] = await parallel([
  () => agent(`${COMMON}
YOUR NAME: joins. Receipt: "${R('joins')}".
Check every chapter card and every slot boundary: frames at -0.2 s, -1 frame, +1 frame and +0.2 s around each. The card reads right (title,
kicker number), nothing pops or jumps (the dot grid continues), no frame of the wrong film, no flash of bare canvas or black, the badge text
matches the chapter. Also the very first and last 2 seconds.`, { label: 'qa:joins', phase: 'Review', schema: OUT, model: M.qa }),
  () => agent(`${COMMON}
YOUR NAME: flow. Receipt: "${R('flow')}".
Read the captions file as the transcript (or run whisper-cli on the extracted audio). Check the narrative: no orphaned sentence starters at cuts,
no repeated sentence (a retake left in), no promise the cut no longer delivers, chapters in a sensible order and the chapters file starting at
0:00 with every chapter >= 10 s. Listen-check by looking at the loudness timeline in qa.md around every flagged cut. Report the exact second.`,
  { label: 'qa:flow', phase: 'Review', schema: OUT, model: M.qa }),
  () => agent(`${COMMON}
YOUR NAME: watch. Receipt: "${R('watch')}".
Watch-through: a frame every 3 seconds across the whole video (contact sheets of ~20 frames), zooming into any frame with text. Every number,
name, handle, date and quote on screen must match the proof pack ("${A.proof_index}": status, honest_line, do_not_show) or his words at that
moment; flag overstatements and misspelled names. Privacy: no third-party names, chats, calendars, emails, member lists or account ids, and
never the price of his paid community. Legibility: nothing under ~22 px; nothing covering what he is pointing at in his screen recording.`,
  { label: 'qa:watch', phase: 'Review', schema: OUT, model: M.qa }),
])
const reviews = { joins, flow, watch }
const all = Object.entries(reviews).flatMap(([name, r]) => (r && r.issues ? r.issues.map(i => ({ ...i, reviewer: name })) : []))
const missing = Object.entries(reviews).filter(([, r]) => !r).map(([n]) => n)
if (missing.length) log(`reviewer(s) returned nothing: ${missing.join(', ')}`)
return {
  ok: !all.some(i => i.severity !== 'minor') && !missing.length,
  blockers: all.filter(i => i.severity === 'blocker'), majors: all.filter(i => i.severity === 'major'),
  minors: all.filter(i => i.severity === 'minor').length, missing_reviewers: missing,
  summaries: Object.fromEntries(Object.entries(reviews).map(([n, r]) => [n, r ? r.summary : null])),
}
