# tools/jev: decision models for the studio pipeline

> **The rule (Samin, Nov 1): never compromise how the edits look and feel.** We optimize speed and
> cost; quality stays at the level of the Oct 16 tips edit. So these helpers only pre-screen, rank and
> suggest. They never remove a check: the Sonnet checker still does its full pass, the verifiers still
> verify, the Opus editorial pass still cuts. A model replaces a judgment only if it matches or beats the
> current judge on labelled data with recall at least as high, and then only as a reported proposal, never
> as a default. Unsure answers (null, a probability in 0.4–0.6, an error) always go to the agent; nothing
> is ever auto-passed.

Two providers behind one client (`decide.py`):

| | TypeSafe **Jev** (`jev-1.14.0`) | Levanto **Sage** |
|---|---|---|
| endpoint | `POST https://api.typesafe.ai/v1/systemone` | `POST https://sage.levanto.ai/decide`, `/decide/batch` |
| kinds | `noul` (yes/no probability), `choice`, `score`, `box` | yesno, choice, scale, sort, tags |
| images | `assets` (base64 PNG/JPEG, 280–1568 px a side), header `TypeSafe-Danger-Zone-Images: true`; many images per request | one image per content (data URI), beta |
| request | one `state` + many named questions (image billed once) | per question (image billed per question) |
| price | $0.042 / M input tokens, output free | $0.05 / M input (Developer), output $10 / M (reasoning only) |
| speed here | ~110–200 ms | ~0.3 s alone, 1–3 s under parallel load |
| key | `TYPESAFE_API_KEY` in `~/.editroom/.env` (mode 600) | `SAGE_API_KEY` in `~/.sagecut/.env` (mode 600) |

Keys are read from the environment or those files and never printed. Answers are cached under
`~/.editroom/cache/decide/` by a hash of provider, model, question and content.

```bash
python3 tools/jev/decide.py --selftest
python3 tools/jev/decide.py ask --image frame.png --yesno private:"Is an email address readable?" --provider both
python3 tools/jev/decide.py ask --image proof.png --box hl:"the words 'vibe coding'"          # Jev only
python3 tools/jev/decide.py run jobs.jsonl -o answers.jsonl --provider jev --jobs 16
```

## In the studio pipeline (all optional; `{"skipped": ...}` without a key)

| command | where | what it does |
|---|---|---|
| `studio.py jev-privacy <W> [--video render.mp4]` | after the first render, before final QA | 1 fps sweep (15 min: ~50 s, ~$0.16): candidate stretches with boxes, a contact sheet `studio/receipts/privacy-sweep/candidates.jpg`, suggested blur beats in `studio/receipts/privacy-candidates.json`. A reviewer confirms each by eye and copies the real ones into `beats/review-privacy.yaml`. Adds to the OCR regex; replaces nothing. |
| `studio.py jev-frames <W> <KEY>` | first step of each checker round | frames of a BEHIND film flagged as clipped / overlapping / empty / too small (each question routed to the more precise model), unsure frames, and the least polished frames -> `checks/jev-frames-v<N>.json`. The checker looks there FIRST, then still does its full 13-point pass. |
| `studio.py polish <video>` | continuity and final QA | polish score per sampled frame; the weakest frames first. A ranking, never a gate. |
| `studio.py rank-crops --claim "..." a.png b.png` | brief writer, choosing among captures | a starting order (agreed with the good edit's choice 71% vs 34% chance); the writer still picks. |

Benchmarks, tables and what each model is (and isn't) good for: [BENCHMARKS.md](BENCHMARKS.md).
