---
description: Edit a raw recording into a finished video (Borumi project name or camera/screen files) with the editroom pipeline.
argument-hint: <Borumi project name | .bmprojbundle path | camera/screen files> [music file] [notes]
---

# /editroom:edit

Get this edited: **$ARGUMENTS**

Load the `edit-video` skill from this plugin and follow it end to end: identify the project, run
the pipeline, handle each gate (proofs in parallel, check, contact-sheet review), render, QA, and
report back with the video path, the chapters block and any flags. Do everything without stopping
unless the project can't be identified. Never upload or publish.
