//! Every prompt `cutroom agent` sends. Answers are small YAML documents validated by the engine.

pub const SYSTEM: &str = "You are the editor of Samin Yasar's YouTube videos (AI + YouTube growth tutorials, a talking head with screen recordings). You answer with exactly the requested format and nothing else: no prose, no code fences.";

/// The system prompt plus the creator's own notes (`<dir>/style.md`), when there are any.
pub fn system(style: Option<&str>) -> String {
    match style.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => format!("{SYSTEM}\n\nThe creator's own notes on his style (follow them):\n{s}"),
        None => SYSTEM.to_string(),
    }
}

/// How the transcripts below are written; shared by every pass that reads one.
pub const TRANSCRIPT_KEY: &str = "Transcript format: one line per sentence. Each line starts with the id of its first word and [talk] (he is on camera, the panel next to him is empty) or [screen] (his screen recording is showing). A `wN` marker every few words gives exact ids: the word right after `w812` is w812, the next one w813, and so on.";

pub const EDITORIAL_PROMPT: &str = r#"You are cutting a raw recording into a tight video. Below is a stretch of the transcript AFTER the automatic cuts (long pauses shrunk, obvious stutters gone). Find everything a sharp human editor would still cut.

Marks inside the text:
- [[rN ...]] is a candidate the detector was unsure about (an earlier attempt, a stutter). Put rN in `accept` when it should go.
- ⏸3.2s is a silence the detector removed (here 3.2 s). On [screen] lines that silence is usually him doing something on screen.

CUT:
1. Retakes. The #1 pattern is the serial restart: he starts a line, stumbles, restarts from the top and gets a little further each time, until one clean pass. Of a run of restarts of the same line, keep only the single most complete, fluent pass and cut ALL the others; a run can be 3 restarts or 30. A later complete take beats an earlier complete take (he re-said it because he rejected it); a complete take beats a later incomplete one. Keeping two or three versions of the same sentence "to be safe" is the bug you are here to fix.
2. Restart vs next point: a restart re-attempts the SAME words; a next point moves FORWARD with NEW content. Never cut a complete, fluent line just because it opens like its neighbour. When no take of a line is clean on its own, keep the fewest consecutive pieces that read as one fluent line.
3. False starts and abandoned half sentences ("we're gonna look into And then", a lone "is" or "we want to" before a restart), stumbles that are corrected right away, remarks to himself ("wait, let me redo that", "maybe I don't need this paragraph"), testing and counting in.
4. Repeats: the same point made twice in a row with nothing new; keep the clearer one.
5. Section restarts: a run of lines that the next stretch redoes from the top ("So let's actually try it out ..." and later "Cool, so now let's actually make our first ..."). Keep the later, complete version and cut the whole earlier run.
6. Filler runs the transcript shows ("so, so, so", "okay okay so", "you know, like").

NEVER CUT:
- anything said only once that the video needs: a claim, a number, a name, a step, a reason, or a line a later line refers back to;
- jokes, laughs and asides that land: his personality stays;
- demo playback: words spoken by a video or app he plays on screen (they often repeat what he just typed or read out; that is NOT a retake);
- the setup of a screen action ("watch what happens when I click this").

Silent screen actions (⏸ on [screen] lines): by default the silence stays cut. Only when the narration depends on a visible action inside it ("and you'll see it appear", "now I click Save") keep up to 2 s with `pauses`.

Be decisive about plain restarts; keep a fluent line only when you genuinely cannot tell whether it duplicates another. Cuts remove whole words: `from` is the first word removed, `to` the last (inclusive). `text` repeats the removed words exactly (for more than 10 words: the first 4, " ... ", the last 4); it is checked against the transcript.

Answer with YAML only, exactly this shape (empty lists are fine):
accept: [r3, r7]
cuts:
  - {from: w120, to: w131, why: retake, text: "So the cool thing is ... start a video.", note: "re-said at w132"}
pauses:
  - {after: w402, keep: 1.5, note: "the board appears"}
`why` is one of: retake, false_start, abandoned, repeat, self_talk, filler, section_restart. `note` is under 10 words.

"#;

pub const FIX_SUFFIX: &str = "\nYour previous answer:\n{answer}\nIt has these problems; answer again with the whole corrected YAML:\n{errors}\n";

pub fn fix(prompt: &str, answer: &str, errors: &[String]) -> String {
    format!("{prompt}{}", FIX_SUFFIX.replace("{answer}", answer.trim()).replace("{errors}", &errors.join("\n")))
}

pub const REVIEW_PROMPT: &str = "These are ambiguous cut candidates from a raw recording. [[...]] marks the proposed cut: an earlier attempt, a stutter or a restart; he always keeps the LAST attempt. Cut it when the bracketed words are said again right after (a retake), are a false start, or are a broken half sentence. Keep it when cutting would drop meaning that is not repeated, or the repetition is intentional emphasis.\n\nAnswer with ONE line: `accept: <numbers>` listing the items to cut, as comma-separated numbers and ranges (e.g. `accept: 1,3,7-9`), or `accept: none`.\n\n";

pub const CHAPTER_PROMPT: &str = "Split this video into YouTube chapters (one per real topic change; roughly every 3 to 12 minutes; the first chapter starts at the first line). Titles: 2 to 5 words in Title Case, his own wording, no em dashes, no numbering, never \"Level N\" or \"Part N\". `short`: 1 to 2 words for an on-screen badge.\n\nAnswer with YAML only, in exactly this shape (`at` = the word id that starts the chapter's first line):\nchapters:\n  - {at: w0, title: Why YouTube Matters, short: Why}\n\nTranscript (word id, [talk] on camera or [screen] demo, sentence):\n";

/// What the visuals prompt may offer.
#[derive(Clone, Debug, Default)]
pub struct BeatsOpts {
    /// Offer `scene` (custom native animations).
    pub scenes: bool,
    /// A separate pass handles [screen] lines; this one only does [talk] lines.
    pub screen_pass: bool,
    /// Existing image files (relative to the edit folder) the model may use as-is.
    pub images: Vec<String>,
    /// Existing video clips (relative) for `video` beats.
    pub videos: Vec<String>,
}

pub fn beats_prompt(o: &BeatsOpts) -> String {
    let mut s = String::from(
        "Add motion graphics to this chapter. He sits on camera next to a panel: on [talk] lines the panel is empty and a stage visual fills it; on [screen] lines the panel shows his screen recording.\n\nKinds, one YAML line each, anchored to a word id (`until: wM` keeps it until word M ends; or `hold: 4s`):\n",
    );
    s.push_str("- {at: wN, until: wM, kind: title, kicker: \"STEP 2\", text: \"Connect *Claude* + Poppy\"}   a key idea as a headline under 7 words; *word* is drawn in the accent colour\n");
    s.push_str("- {at: wN, kind: bullets, title: \"...\", items: [{text: \"...\", at: wN}, ...]}   a list he walks through; each item appears on its own word\n");
    s.push_str("- {at: wN, kind: number, value: \"$10,075\", label: \"paid for one video\"}   a number he says; the number is the biggest thing in the frame\n");
    s.push_str("- {at: wN, kind: code, title: \"Prompt\", text: \"...\"}   a prompt or command he reads out (types on)\n");
    s.push_str("- {at: wN, kind: quote, text: \"...\", by: \"...\"}   a quote he cites\n");
    s.push_str("- {at: wN, kind: image, src: proof/<slug>.png, source: \"site.com\", caption: \"...\", proof: {claim: \"...\", find: \"...\"}}   PROOF of a claim about the outside world (a price, a stat, a product page, a tweet, a repo). The screenshot is captured later: `claim` is exactly what it must show, `find` where to get it (a URL or a precise search); `slug` is short kebab-case\n");
    if !o.images.is_empty() || !o.videos.is_empty() {
        s.push_str("- {at: wN, kind: image, src: <file>, caption: \"...\", source: \"...\"} or {at: wN, kind: video, src: <file>}   one of these existing files:\n");
        for f in o.images.iter().chain(&o.videos).take(60) {
            s.push_str(&format!("    {f}\n"));
        }
    }
    if o.scenes {
        s.push_str(
            "- {at: wN, until: wM, kind: scene, layers: [...]}   a CUSTOM animation for the 1 to 3 moments per chapter that deserve more than a template: a process, a comparison, a before/after, how things connect. Layer coordinates are 0..1 inside the panel:\n    {id: head, type: text, text: \"Claude *talks to* Poppy\", x: 0.5, y: 0.14, size: 64, weight: bold, align: center}\n    {id: a, type: box, x: 0.08, y: 0.38, w: 0.32, h: 0.26, radius: 24, fill: card, stroke: accent, label: \"Claude\", in: {at: wN, from: left}}\n    {type: arrow, from: [0.40, 0.51], to: [0.60, 0.51], color: accent, width: 6, in: {at: wN, draw: true}}\n    {type: counter, from: 0, to: 10075, prefix: \"$\", x: 0.5, y: 0.82, size: 72, in: {at: wN}}\n    also circle, line, image (src), code, highlight. Each layer enters on its own word (`in: {at: wN, from: up|down|left|right|fade|scale|pop}`) so it builds while he talks. Colours: accent, ink, muted, card, green, red, gold or #hex. Keep a scene to 3 to 8 layers.\n",
        );
    }
    if !o.screen_pass {
        s.push_str("- {at: wN, kind: callout, text: \"...\"}   a 2 to 4 word pill; the only kind allowed on [screen] lines\n");
    }
    s.push_str(
        "\nRules:\n- Proof first: when he makes a claim about something real, show the real thing while he says it, with the key number or name in the caption. His own screen is the best proof, and on [screen] lines it is already showing.\n- Honest: never show more than he said or a source supports; never invent numbers, names or facts. If he says \"might\", never write \"will\".\n- Stage kinds (title, bullets, number, code, quote, image, video, scene) only on [talk] lines.",
    );
    if o.screen_pass {
        s.push_str(" Leave [screen] lines alone: another pass marks up the screen.");
    }
    s.push_str(
        "\n- One idea per beat, 3 to 8 s on screen, never overlapping, at most one beat per ~15 s of talk, none while he is only chatting. Nothing static for more than ~4 s: lists reveal item by item, scenes build layer by layer.\n- Anchor a beat on the word where the idea starts (it lands just before he says it) and `until` the word where the idea ends.\n- Headlines in his own words, under 7 words, no em dashes.\n- Fewer, stronger beats beat many; a chapter of pure chatting gets none.\n\nAnswer with YAML only, one line per beat:\nbeats:\n  - {at: w123, until: w140, kind: title, text: \"...\"}\n\n",
    );
    s.push_str(TRANSCRIPT_KEY);
    s.push_str("\n\nChapter transcript:\n");
    s
}

pub const SCREEN_PROMPT: &str = r#"The images are frames of his screen recording while he says the lines below. Mark up the screen so the viewer looks exactly where he means:
- {at: wN, until: wM, kind: ring, rect: [x, y, w, h]}         circle the thing he names or clicks (a button, a field, a number)
- {at: wN, until: wM, kind: highlight, rect: [x, y, w, h]}    a marker over a line of text he reads or points at
- {at: wN, until: wM, kind: zoom, rect: [x, y, w, h]}         push in when the detail he talks about is too small to read (16:9-ish, at least 0.3 wide)
- {at: wN, until: wM, kind: arrow, from: [x, y], to: [x, y]}  point at something that a ring would not fit
- {at: wN, kind: callout, text: "..."}                         a 2 to 4 word pill naming the step
Coordinates are fractions of the image (0..1) from its top-left: rect is [x, y, width, height] around the thing, a little padding included. Use only positions you can see in the frame taken closest to those words (each frame is labelled with its time and the word he says then); the screen changes, so do not reuse a position across frames unless it plainly did not move.
At most 3 marks for this stretch, none overlapping in time, only where he names or points at something specific. If nothing specific is named, or you cannot locate it, answer `beats: []`.

Answer with YAML only:
beats:
  - {at: w812, until: w820, kind: ring, rect: [0.62, 0.08, 0.2, 0.06]}

"#;
