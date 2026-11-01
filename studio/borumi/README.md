# studio/borumi: hand the finished video to Borumi

editroom edits and renders the video itself (cutroom). Borumi only receives the result, in its own
project, so Samin can open it, tweak it and export from Borumi like any other project. The source
recording project is never written.

## Commands

```bash
# the newest out/<name> - editroom vN.mp4 into the Borumi project "<name> - editroom"
python3 -I studio/borumi/import.py "~/Borumi Projects/tips raw editroom"

python3 -I studio/borumi/import.py "<work>" --video final.mp4 --name "tips raw - editroom"
python3 -I studio/borumi/import.py "<work>" --films            # also add the section films as Media
python3 -I studio/borumi/import.py "<work>" --export-check     # export 10 s through Borumi, compare a frame
python3 -I studio/borumi/import.py "<work>" --focus            # show the project in Borumi afterwards
python3 -I studio/borumi/import.py "<work>" --dry-run          # say what would happen, write nothing
python3 -I studio/borumi/import.py "<work>" --force            # re-import even an unchanged video

python3 -I studio/borumi/bmcp.py list_recent_projects '{}'    # any MCP tool, for debugging
```

Borumi must be running (Settings > AI > Enable MCP). Reports go to `<work>/studio/borumi-import.json`.

## What gets created

- Borumi project **`<name> - editroom`** at `~/Borumi Projects/<name> - editroom.bmprojbundle`,
  made with the MCP's `create_project` (1920x1080 landscape canvas; set to the video's size if it differs).
- One scene, **`editroom`**, holding the finished video as one take: `screen` video (face tracking off,
  cut transition) + `system_sound` audio (voice enhancement off, so the mastered mix is untouched),
  under a pinned `fullscreen` layout (cover). No camera layer, so no camera tile.
- With `--films`: each section film (`<work>/studio/jobs/*/render-status.json` with `state: done`,
  not drafts; or `<work>/studio/films/*`) imported as project Media, not on the timeline. Borumi
  keeps them in the bundle (`medias/`) for later edits; the `editroom` scene still plays only the
  finished video, so exporting the project exports exactly the edit.

## How an import runs

One Borumi transaction, checked before commit (any mismatch aborts it and nothing changes):

1. `get_guides` + the guides for project edits, scripting, editing, importing, canvas, take/layout add,
   video/audio update (Borumi gates tools per connection).
2. `list_recent_projects` / `list_open_projects` to find the project by name and path (project ids
   are 4-hex aliases that only exist on the current connection: never store or reuse them), then
   `create_project` or `open_project`.
3. `begin_project_edit` -> `get_project_overview` (canvas, scenes) -> `create_scenes` the first time,
   or `delete_segments` the previous clip's content segments (ripple; Borumi drops the layout that
   pointed at it; naming the layout in the same call makes the delete fail) on re-runs.
4. `import_media` (local files complete synchronously; `get_media_import_status` otherwise), then a
   check that Borumi read the same duration and both tracks.
5. `add_segments` take at the scene start -> `update_segments` (video: `face_tracking_mode: disabled`,
   `transition: cut`; audio: `voice_enhancement_filter: disabled`) -> `add_segments` fullscreen layout.
6. Verify the staged timeline (one video and one audio segment spanning the clip, layout present)
   and a rendered frame from `inspect_timeline` against the mp4 (PSNR >= 30 dB, best of +-2 frames:
   the preview is a JPEG on Borumi's own 30 fps clock).
7. `commit_project_edit`, then read the committed project back (scene length = video length).
8. `--export-check`: `export_video` of 10 s (custom 1920x1080/30), poll `get_export_status`, compare a
   frame. Borumi resamples to its own clock and a range export can start 1-2 frames early (10 s
   exports as 10.07 s), so the check takes the best match within +-2 frames.

Re-runs are idempotent: the registry keeps a fingerprint (size + hash of the first and last 4 MB)
of the last imported video and of each film, so an unchanged video or film is skipped. A new render
replaces the clip in the same scene.

## Safety rules

- Every write goes through one gate: the project's name must end with `" - editroom"` **and** the
  project must be in `~/.editroom/borumi-projects.json`, the list of projects this tool created.
  A same-named project editroom did not create is refused (pass another `--name`).
- Source projects are only ever read. No `trim_timeline`, `delete_scenes`, `delete_segments` or
  canvas change is ever sent to a project outside the registry; inside an editroom project only the
  `editroom` scene's own clip is replaced.
- Borumi's MCP cannot delete projects or Media, and this tool never touches files. Replaced videos
  stay in the bundle's `medias/` (each re-import of a changed 15-min render adds about 0.8 GB); remove
  old ones, or the whole project, from Borumi by hand when you want the space back.

## Tested (Oct 31, 2026, Borumi 0.32.1)

`tips raw - editroom v2.mp4` (15:43.6) into the test project
`~/Borumi Projects/tips raw test - editroom.bmprojbundle`: 8.3 s including a 10 s export check;
staged frame 49.6 dB, export 34.8 dB (one frame of clock offset). Re-run: skipped in 0.1 s.
Replace (a 3 s clip, then v2 again): old clip and layout removed, one new clip + one layout.
Two films (an h264 mp4 and a ProRes 4444 alpha mov) imported as Media. Offline tests:
`python3 -I -m unittest discover -s studio/borumi/tests`.
