# Reference jobs

Six finished velour scenes from Samin's two best edits. Designers read these before writing a
scene: they show the level of craft the edit is judged against (real proof, one highlight per
spoken target, beats every 2.5 to 6 s, nothing static, everything keyed to his words).

| Folder | Source | What to learn from it |
|---|---|---|
| `honcho-hook` | Honcho video, hook (anim-mvagv60a) | A cold open built from real proof crops: a strip of tools, a warning card, one sentence highlighted as he says it. How a hook moves fast without being busy. |
| `honcho-proof-cards` | Honcho video (anim-mvagv66m) | Proof cards (GitHub header, one quoted line) plus a local `HubDiagram.tsx`: a hub-and-spoke diagram that builds on his words. How to mix a diagram with the proof it is about. |
| `tips-t1-ask-for-html` | Tips video, TIP 1 (anim-mvas6emb) | The shared `TipOpener`, section kickers ("WHY IT WORKS", "THE TIP"), tweet cards with a highlighted phrase, a research page (OpenStax) with the exact sentence marked, and a real b-roll clip (`geo-clickthrough.mp4`) in a browser card. The standard tip section. |
| `tips-t4-agent-handoffs` | Tips video, TIP 4 (anim-mvas6eqq) | Local components (`Handoff.tsx`, `LocalCards.tsx`): an X article and two posts with boxed phrases, then an animated handoff diagram (nodes, packets travelling along links) that follows the narration, and a clean exit so the next section opens on an empty panel. |
| `tips-t6-agent-os` | Tips video, TIP 6 (anim-mvas6evb) | The checklist with inline SVG icons and check marks ("For this step": Routines, Skills, Connectors, Context), docs crops in dark mode, a UI mockup of an "operating system" window. How to build a layered concept step by step. |
| `tips-t7b-t3code-overlay` | Tips video, TIP 7 demo (anim-mvas6eyk) | A TRANSPARENT overlay (`background: transparent`, ProRes 4444) over his screen recording: right-click callouts ("RIGHT-CLICK · Hand off to another model"), repo and README crops sliding in beside the demo, an account rail. How to annotate a screen demo without covering what he is doing. |

Each folder has the job's own files (`Scene.tsx` plus local components, `brief.md` = the brief the
designer worked from, `words.json` = his words with timings, `render.json`), `source-job.json`
(size, frames, the project start it was made for), `public/` (the proof crops and clips it uses,
at the same paths the scene asks `staticFile()` for) and `stills/sheet.png` (a contact sheet of
the final render).

Render one to see it move:

```bash
node studio/jobs/job.mjs reference tips-t4-agent-handoffs --draft   # creates a job from it and renders 960x540
```

These scenes were made for Borumi's timeline (project time, panel mode). In studio mode the same
components draw cutroom's canvas instead; the scenes need no changes.
