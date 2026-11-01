# editroom for any coding agent

To edit a video ("get this edited", "edit my Borumi project", "cut and visualize"), follow
`skills/edit-video/SKILL.md` in this repo. `ER` in that file is this repo's root.

If your harness has no subagents, run the whole pipeline yourself:

```bash
bash scripts/install.sh                                   # once
bash scripts/pipeline.sh "<Borumi project or bundle path>" --auto-proof --no-review
```

`--auto-proof` captures proof headless (`claude -p`, 6 at a time); `--no-review` skips the
contact-sheet gate. Still handle gate 11 (check errors) by fixing `W/beats/*.yaml` and rerunning
with `--from-stage check`. Never upload or publish; never render with `--force`.
