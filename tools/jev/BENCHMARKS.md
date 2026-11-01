# Decision models in the studio pipeline: benchmarks (Nov 1, 2026)

Jev = TypeSafe `jev-1.14.0`; Sage = Levanto `levanto-sage-2`. All on real data from Samin's edits, labels
from the hand edit, the verified proof pack and the reviewers of the tips raw draft. Latencies are medians
with 12–16 requests in flight from this Mac. Costs are input tokens only: Jev $0.042/M, Sage $0.05/M
(Developer plan rate; answers are free on both). Jev answers every question about one image from ONE
request (the image is billed once); Sage bills the image once per question.

Scripts and raw answers: `/private/tmp/editroom-jev/` (`scripts/`, `*-jev.jsonl`, `*-sage.jsonl`).

## 1. Privacy on screen frames (tips raw draft, 15:43, 1 fps, 935 labelled frames)

Positives = frames where a private detail is readable (5 emails the OCR regex found, plus 3 it missed:
a Heptabase card URL with workspace/card ids, a truncated personal email in a hover card, a "Share Mercury
bank" thread title readable only during a zoom). Labels were corrected by eye at window edges.

| | recall | precision | unsure | latency | cost per 1k frames | 15 min at 1 fps |
|---|---:|---:|---:|---:|---:|---:|
| Jev (yes/no + box, one request) | **0.99** (109/110) | 0.81 | 9 | 168 ms | ~$0.17 | 33 s API, 51 s with frame extraction, ~$0.16 |
| Sage (yes/no) | 0.95 (105/110) | **0.98** | 2 | 1,357 ms | ~$0.07 | 97 s, ~$0.07 |
| cutroom OCR regex (today) | 5 of 8 items | 1.0 | | | | 12 s |

- Jev caught all 3 items the regex missed (the zoomed bank line at frame level); Sage missed the bank line.
- Jev's box landed on the Heptabase URL and both emails; one box per question, so a second item in the same
  frame (the hover-card email next to the URL) is not boxed, and the bank-line box was off (conf 0.11).
- Jev's false positives: his own public GitHub repo URL and his own SamRabbit chat list (worth a glance).

## 2. Render QA on frames (65 clean frames from the good tips films + 65 planted defects)

| defect | Jev recall / precision | Sage recall / precision | note |
|---|---|---|---|
| text/card cut off at an edge | 1.00 / 0.59 | 1.00 / 0.45 | |
| overlap / ghost | 1.00 / 0.48 | 1.00 / **0.76** | |
| empty panel | 1.00 / 1.00 | 1.00 / 1.00 | cutroom `check-render` also catches it |
| content under the camera tile | 1.00 / 0.87 | 1.00 / 0.93 | `check-render` catches it exactly |
| text too small | 0.77 / 0.50 | 1.00 / 0.25 | Sage flags most body text |

Frame level: Jev flagged 64/65 defect frames and 9/65 clean ones; Sage 65/65 and 27/65. Latency 168 ms
(Jev) vs 2.9 s (Sage); ~810 Jev tokens per question with 5 questions per image. On a real good film
(T6, 92 frames) the routed pre-screen lists 2 confident flags + 10 unsure frames in 14 s, and an earlier
run's "clipped" flag found a real flaw that shipped in the good edit (a docs card cutting words mid-word
at its right edge, 44–46 s).

## 3. Proof support (80 captures from the verified tips proof pack: 60 supported, 20 partly/wrong)

| | catches not-supported (recall) | precision | yes/no accuracy | 3-way choice accuracy | latency |
|---|---:|---:|---:|---:|---:|
| Jev | 0.95 | 0.29 | 0.41 | 0.62 | 200 ms |
| Sage | 0.40 | 0.73 | 0.81 | 0.69 | 2.6 s |

Neither can stand in for the verifier: Jev flags nearly everything; Sage would wave through 12 of 20
weaker claims.

## 4. Box vs exact glyph boxes (25 phrases in proof PNGs, truth = `cutroom boxes`)

Jev box: median IoU 0.29, IoU >= 0.5 on 2/25, 212 ms. Good for "roughly here", not for highlight rects:
`cutroom boxes` (tesseract) stays.

## 5. Retake judgment (46 detector/editorial candidates with hand-edit labels, Youtube course)

| | accuracy | precision | recall | unsure | latency |
|---|---:|---:|---:|---:|---:|
| Opus editorial agent (today) | 0.78 | 0.81 | **0.95** | 0 | (one call per chunk) |
| Sage | **0.82** | **0.88** | 0.91 | 7 | 295 ms |
| Sage, unsure -> Opus | 0.78 | 0.83 | 0.92 | 0 | |
| Jev | 0.75 | 0.82 | 0.90 | 10 | 107 ms |

Recall below Opus's, on a small set; and the Opus pass also FINDS the spans, which a decision model can't.
Not a replacement (proposal only: Sage as a second opinion on the detector's unsure list).

## 6. Creative

- **Polish score** (one score question per frame): separates 50 frames of Samin's good tips edit from 50 of
  the rejected draft at **AUC 0.85 (Jev)**, 0.79 (Sage). A ranking signal ("look at the weakest frames
  first"), never a gate. ~4k Jev tokens per frame.
- **Proof crop choice** (78 claims with 2–4 captures; truth = the crop the good edit used): chance 0.34,
  Jev 0.71 (all crops in one request, 289 ms) / 0.72 (per-crop score), Sage 0.59 (per-crop score). A
  starting order for the brief writer.
