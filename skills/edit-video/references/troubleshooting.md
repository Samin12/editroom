# Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `no cutroom engine` | not installed | `bash "$ER/scripts/install.sh"` |
| `MISSING tesseract` / `ffmpeg` / `claude` | dependency | install it (the message says how), rerun install.sh |
| engine build failed | Rust too old, or a broken engine change | `rustup update`; read `~/.editroom/build.log` |
| `no Borumi project matches` | name differs | `python3 -I "$ER/scripts/find_project.py"` lists them all; pass the `.bmprojbundle` path |
| `is ambiguous` | two projects with close names | pass the exact bundle path |
| ingest: `source is invalid` | a take without media, or media moved | open the project in Borumi once (it relinks), or `--scenes` to skip the bad scene |
| plain files: no transcript | whisper-cli missing | `brew install whisper-cpp`, or pass `--transcript words.json` |
| analyze: agent failed | `claude` not logged in, rate limit, network | `claude -p "hi"` to test; rerun `--from-stage analyze` |
| analyze: privacy failed | tesseract missing or a screen clip unreadable | `logs/privacy.log`; never skip privacy |
| `is an old cutroom build` | `~/.editroom/bin/cutroom` predates the merged engine | `bash "$ER/scripts/install.sh" --force` |
| gate 11 a third time | the same beat keeps failing | remove every beat the errors name (move to `W/agent/removed-by-check.yaml`), note them in the report |
| a cut removed something he meant to keep | the editorial pass misjudged it | delete its `by: agent` line in `cuts.yaml` (`review.md` lists them all), `--from-stage check` |
| music is too loud / pumps | bed level or duck | `gain_db: -22`, `duck_db: 12-15`, `release_ms: 600` in `output.music` |
| render refuses | check has errors or a visible privacy finding | fix them; never `--force` |
| render slow / Mac pinned | several renders or Remotion jobs at once | one render at a time; the engine already uses every core |
| QA: camera frozen | a missing camera clip in the source | check the take in Borumi; re-ingest |
| video looks different from Borumi | cutroom renders its own composition (camera + screen + graphics) | Borumi effects, overlays and cursor styling are not copied; set `style:`, add `--music`, use `scene` beats |
| a 2x playback in the source | cutroom plays source clips at 1x | fine for screen playback he recorded; a sped-up Borumi segment plays at normal speed |

## Where things are

- `W/editroom-state.json`: what is done, the current gate. `W/editroom-log.json`: timings.
- `W/logs/<stage>.log`: the exact command, its time and output.
- `W/agent/`: every prompt and answer the editorial agent sent (`usage.json` has cost and tokens).
- `W/beats/*.yaml`: all visuals; `W/cuts.yaml`: cuts; `W/edit.yaml`: settings and hand edits.

## Starting over

`--restart` forgets progress but keeps files; to rebuild from the raw project, use a fresh `--work`
folder (never delete the old one without asking Samin).
