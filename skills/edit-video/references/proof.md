# Proof: what counts, how to capture it, how to stay honest

## The flow

1. `cutroom agent` writes image beats with a `proof` request where Samin makes a checkable claim
   (`proof: {claim, find}`; `find` is optional):
   ```yaml
   - {at: w1340, kind: image, src: proof/launchpoint-rates.png,
      proof: {claim: "LaunchPoint charges $10,075 per video", find: "launchpoint.com pricing page"}}
   ```
   `check` warns "proof pending" for each, and `render` refuses while any is pending.
2. The pipeline's `proofs` stage writes `cutroom proofs W` to `W/proofs-pending.json`, one object
   per request: `file` (its beats file), `index` (its position in that file's `beats`), `at`,
   `out` (time in the edited video), `claim`, `find`, `src` (where the PNG must go, relative to `W`)
   and `line` (`talk` = he is on camera, `screen` = his screen recording shows there).
3. Gate 10: one `editroom:proof-capturer` per request, all in one message, in parallel. Each writes
   the PNG at `src` and `W/proof/<slug>.source.json`, and deletes the PNG when the source does not
   support the claim. At most 2 capture rounds (round 2 only for transient failures: a timeout,
   a blocked page worth a second route).
4. `drop` stage: `apply_proofs.py` writes each capture's `highlight` and `source` pill onto its
   beat, overwriting the agent's guesses ("x.com" becomes "x.com/lingxi · captured Oct 31, 2026"),
   plus the corrected `caption` when the capturer fixed a misspelled name. It handles the agent's
   JSON-line beats as well as YAML flow and block beats. Then `cutroom proofs W --drop-missing` replaces every request still without a PNG with its
   caption (else the claim) as a `title` (≤ 7 words, on camera), a `quote` (8-25 words, on camera)
   or a `callout` (≤ 5 words, over the screen), or removes it when none fits. The report lists every
   capture and every flag.

You can add a proof beat by hand anywhere: write the beat with `src` + `proof`, then run the capturer.

## What counts as proof

- The primary source, live, logged out: the product's pricing page, the repo, the post, the official
  report, the docs. Articles about it only when there is no primary source.
- His own screen recording: when the claim is visible in his footage, a `ring`/`highlight`/`zoom`
  on the screen beats a screenshot of anything.
- A real post (X, LinkedIn, Reddit, YouTube) as the platform renders it, with author and date.
- Exact figures from APIs: `gh api repos/<owner>/<repo> --jq .stargazers_count`.
- **Never:** generated images or video, a mock-up of a UI, a retouched number, a page you had to log
  in to see (he can supply his own logged-in screenshots).

## Capture recipes

```bash
CP="node $ER/scripts/capture_proof.mjs"
$CP "https://example.com/pricing" "$W/proof/example-pricing.png" --highlight "$10,075" --selector ".pricing-table"
$CP "https://x.com/user/status/123" "$W/proof/user-post.png" --highlight "10x faster"      # X post cropper
$CP "https://github.com/owner/repo" "$W/proof/repo.png" --highlight "Star" --scale 3
$CP "https://code.claude.com/docs/en/memory" "$W/proof/memory.png" --selector "#auto-memory" --extend 260 --highlight "accumulate knowledge"
$CP "https://docs.example.com/limits" "$W/proof/limits.png" --highlight "200,000 tokens"   # one screen, scrolled to the phrase
```

- Output: `<png>` at 2x (or `--scale`), landscape, and `<png minus .png>.json` with the page title,
  final URL, capture time, visible `text` (verify the claim against it) and `highlights[].rect`
  (normalized `[x, y, w, h]` in the image: put it straight into the beat's `highlight`).
- `--selector` crops to one element (padded by `--pad 24`); `--extend <css px>` grows that crop
  downward, so a heading id plus `--extend 200-400` takes the heading and its paragraphs. Use it so
  text reads at panel size: a 1920 px page captured whole is unreadable once scaled into the panel.
- Without `--selector` the capture is one screen (1440x900 css at 2x). When the first `--highlight`
  is further down a long page, that screen is scrolled to it (the phrase sits about a third down).
  `--full` takes the whole page. Crops are measured in document coordinates, so anything far down a
  long docs page crops correctly.
- X posts also leave context shots (`<slug>-page.png`, `<slug>-full.png`) in `W/proof/raw/`: they
  are for checking, never the proof.
- Cookie bars and sign-up walls are hidden (never clicked). Each run uses its own temp Chrome
  profile and closes its own browser; never `pkill` Chrome by pattern (it kills other captures).
- A site blocks headless Chrome: try its official PDF or a public embed page; else `not_found`.
- Pages follow the system theme; `--theme dark` matches the dark canvas when the site supports it.

## Verdicts and the trap in every claim

| Verdict | Meaning | Beat |
|---|---|---|
| `supported` | the source says exactly that | shown, highlighted |
| `rounded` | he rounded it ($23.77B vs "$23 billion") | shown with the exact figure highlighted |
| `partly` | a weaker or different version is true | shown with the TRUE line highlighted; flag |
| `unsupported` / `not_found` | not true, or nothing found | PNG deleted → text beat; flag |

Look for the trap: launch vs regular price; free vs free trial vs refund; per month vs per year;
estimate vs actual; an old version of a page; "open source" vs source-available; "made by X" vs
"built on X"; clean-room rebuild vs reverse engineering.

Real REA examples: "you can resell it" when only the tool is MIT licensed (never print "resell");
"download the free version" when there is no free trial (a 14-day refund); "worth like $15" that is a
$39 product at launch price (show both).

## The record

`W/proof/<slug>.source.json`:

```json
{"src": "proof/launchpoint-rates.png", "claim": "LaunchPoint charges $10,075 per video",
 "url": "https://launchpoint.com/pricing", "captured": "2026-10-31", "verdict": "supported",
 "shows": "Starter: $10,075 per video", "highlight": [0.41, 0.52, 0.18, 0.05],
 "source_pill": "launchpoint.com · captured Oct 31, 2026", "caption": null, "flag": null, "do_not_show": []}
```

`caption` is optional: the corrected on-screen caption when the transcript misspelled a name or term
(on the first real run: "Boris Cherney" → Cherny, "Grockbot" → Grok Bot, "Ling Ji Li" → Lingxi Li).
The report lists every correction.

One file per capture (parallel capturers never share a file). The source pill format is
`<domain> · captured <Mon D, YYYY>`.
