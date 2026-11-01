//! Every prompt `cutroom agent` sends. Answers are small YAML documents validated by the engine.

pub const SYSTEM: &str = "You are the editor of Samin Yasar's YouTube videos (AI + YouTube growth tutorials, a talking head with screen recordings). You answer with exactly the requested format and nothing else: no prose, no code fences.";

/// The system prompt plus the creator's own notes (`<dir>/style.md`), when there are any.
pub fn system(style: Option<&str>) -> String {
    match style.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => format!("{SYSTEM}\n\nThe creator's own notes on his style (follow them):\n{s}"),
        None => SYSTEM.to_string(),
    }
}

/// The "Names" section of `style.md`: correct spellings of people, products and tools.
pub fn names_glossary(style: Option<&str>) -> Option<String> {
    let s = style?;
    let mut out = vec![];
    let mut on = false;
    for line in s.lines() {
        let t = line.trim();
        let bare = t.trim_start_matches('#').trim().trim_end_matches(':');
        if !on {
            on = !t.starts_with(['-', '*', '+']) && bare.split(|c: char| !c.is_alphanumeric()).next().is_some_and(|w| w.eq_ignore_ascii_case("names"));
            continue;
        }
        let item = line.starts_with([' ', '\t']) || t.starts_with(['-', '*', '+']) || t.is_empty();
        if !item {
            break;
        }
        if !t.is_empty() {
            out.push(t.trim_start_matches(['-', '*', '+']).trim().to_string());
        }
    }
    (!out.is_empty()).then(|| out.join("; "))
}

/// The names rule, plus the glossary when there is one. Shared by every prompt that puts text on
/// screen.
pub fn names_rule(glossary: Option<&str>) -> String {
    let mut s = String::from("- Names: the transcript is speech-to-text and misspells people, products and tools (\"Hancho\" is Honcho, \"Grockbot\" is Grok Bot, \"Cloud Code\" is Claude Code). Correct every name from context");
    match glossary {
        Some(g) => s.push_str(&format!(" and from this glossary: {g}.")),
        None => s.push('.'),
    }
    s.push_str(" Every word you put on screen (titles, labels, captions, chapter titles, callouts) uses the correct spelling; never put a misspelled transcript word on screen.\n");
    s
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

pub fn chapter_prompt(glossary: Option<&str>) -> String {
    format!(
        "Split this video into YouTube chapters: one per real topic change, roughly every 3 to 12 minutes. When he numbers his points (tip 1, step 2, part 3), the chapters are exactly those numbered points in his order, so chapter n is his point n.\n\
The first chapter starts AFTER his hook and intro (the opening where he says what the video is and why to watch it): a chapter card there would cut into his first sentence. The intro gets no chapter; name it in `output.intro` (2 to 5 words), which becomes YouTube's 0:00 chapter.\n\
`output.kicker` is what the chapters are called on screen: Tip, Step or Part when he numbers his points that way (the card then shows \"TIP 3 OF 10\"), or \"\" when he does not number them.\n\
Titles: 2 to 5 words in Title Case, his own wording, no em dashes, no numbers (the kicker carries them). `short`: 1 to 2 words for an on-screen badge.\n{}\n\
Answer with YAML only, in exactly this shape (`at` = the word id that starts the chapter's first line):\n\
output: {{kicker: Tip, intro: Why Voice Beats Typing}}\n\
chapters:\n  - {{at: w212, title: Talk Instead Of Typing, short: Voice}}\n\n\
Transcript (word id, [talk] on camera or [screen] demo, sentence):\n",
        names_rule(glossary)
    )
}

/// Where a chapter's visuals sit in the video, so on-screen numbers follow the chapter order.
pub fn chapter_context(n: usize, total: usize, title: &str, kicker: &str) -> String {
    let k = kicker.trim();
    if n == 0 {
        return format!("This stretch is the intro, before chapter 1{}. Nothing on screen here carries a chapter number.\n\n", if k.is_empty() { String::new() } else { format!(" ({} 1)", k.to_uppercase()) });
    }
    let mut s = format!("This is chapter {n} of {total}: {title:?}.");
    if k.is_empty() {
        s.push_str(" He does not number his chapters: no \"tip n\" / \"step n\" kickers on screen.");
    } else {
        let up = k.to_uppercase();
        s.push_str(&format!(" He calls his chapters {}s: a kicker or headline that numbers it says {up} {n}, never another number.", k.to_lowercase()));
    }
    s.push_str("\n\n");
    s
}

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
    /// Correct spellings from style.md ("Names").
    pub glossary: Option<String>,
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
    s.push_str("- {at: wN, kind: image, src: proof/<slug>.png, source: \"site.com\", caption: \"YouTube: the #2 search engine\", proof: {claim: \"...\", find: \"...\"}}   PROOF of a claim about the outside world (a price, a stat, a product page, a tweet, a repo). The screenshot is captured later. `caption` is what the viewer reads under it: his claim in at most 7 words, with the number or name in it. `claim` is the fact the screenshot must show, as a plain statement. `find` is where to get it (a URL or a precise search). `slug` is short kebab-case\n");
    if !o.images.is_empty() || !o.videos.is_empty() {
        s.push_str("- {at: wN, kind: image, src: <file>, caption: \"...\", source: \"...\"} or {at: wN, kind: video, src: <file>}   one of these existing files:\n");
        for f in o.images.iter().chain(&o.videos).take(60) {
            s.push_str(&format!("    {f}\n"));
        }
    }
    if o.scenes {
        s.push_str("- {at: wN, until: wM, kind: scene, layers: [...]}   a CUSTOM animation for the 1 to 3 moments per chapter that deserve more than a template: a process, a comparison, a hierarchy, how things connect. Rules and examples below.\n");
    }
    if !o.screen_pass {
        s.push_str("- {at: wN, kind: callout, text: \"...\"}   a 2 to 4 word pill; the only kind allowed on [screen] lines\n");
    }
    s.push_str(
        "\nRules:\n- Proof first: when he makes a claim about something real, show the real thing while he says it, with the key number or name in the caption. His own screen is the best proof, and on [screen] lines it is already showing.\n- Honest: on-screen words never claim more than he said. Keep his qualifiers (\"for me\", \"might\", \"about\", \"found\") and his exact verb. Wrong: he says \"for me the subscription is worth it\", the headline says \"Subscriptions beat API pricing\". Wrong: he says \"I found a hole in 15 minutes\", the headline says \"Built in 15 minutes\". Never invent numbers, names or facts.\n- Stage kinds (title, bullets, number, code, quote, image, video, scene) only on [talk] lines.",
    );
    if o.screen_pass {
        s.push_str(" Leave [screen] lines alone: another pass marks up the screen.");
    }
    s.push_str(
        "\n- One idea per beat, 3 to 8 s on screen, never overlapping, at most one beat per ~15 s of talk, none while he is only chatting. Nothing static for more than ~4 s: lists reveal item by item, scenes build layer by layer.\n- Anchor a beat on the word where the idea starts (it lands just before he says it) and `until` the word where the idea ends.\n- Headlines in his own words, under 7 words, no em dashes.\n- Fewer, stronger beats beat many; a chapter of pure chatting gets none.\n",
    );
    s.push_str(&names_rule(o.glossary.as_deref()));
    if o.scenes {
        s.push_str(SCENE_GUIDE);
    }
    s.push_str("\nAnswer with YAML only, one line per beat (a scene may span several lines):\nbeats:\n  - {at: w123, until: w140, kind: title, text: \"...\"}\n\n");
    s.push_str(TRANSCRIPT_KEY);
    s.push_str("\n\nChapter transcript:\n");
    s
}

/// How to make a `scene` look designed rather than like a wireframe, with worked examples.
pub const SCENE_GUIDE: &str = r#"
SCENES must look designed, not like a wireframe:
- Fill the panel: content spans 70 to 90% of it. Margins 0.05; heading at y 0.07; content from y 0.25 to 0.9. Boxes are big: two side by side are w 0.42 each (x 0.05 and 0.53); three in a row are w 0.27 (x 0.05, 0.365, 0.68); h 0.18 to 0.6.
- Type scale (units of a 1920-wide panel): heading `size` 88 to 100, bold. Box labels `size` 52 to 60 (a box's `sub` is drawn at 0.56 of that, so 30 to 34). Free text 36 to 56. Counters 150 to 180. Labels 1 to 3 words, subs 2 to 5 words.
- Connect what relates: give boxes and circles an `id` and draw `{type: arrow, from: <id>, to: <id>}` between them, entering on the word that links them. A hierarchy, a flow or a hub always has arrows.
- One payoff per scene, picked by content: he says a number, so a big `counter`; he names a real page, post or product AND a matching file is in the list above, so that `image` with `ring` on the key detail (only listed files; never invent a path); otherwise a heading with `style: marker` (marks its *accent* words). Vary it: not the same payoff in every scene.
- Use the whole height: content runs from y 0.25 to 0.9. One row of boxes means tall boxes (h 0.45 to 0.6) or a big element in the middle; never a band of small boxes with empty space above.
- Pick the shape his words describe: a comparison (two big cards), a hub (many into one), a hierarchy (one over many), a stack ("layers", "built on top of": bars bottom-up), a flow (A to B to C left to right), a stat beside its proof. Every box says something specific from his words; never several boxes with the same label.
- Build it on his words: every layer gets `in: {at: wN}` on the word that introduces it, at least 3 different words across the scene. The key element enters with `from: pop`.
- Finish early: the last layer (arrows included) enters at least 2 s (about 6 words) before `until`, so the finished picture holds while he talks about it. Set `until` on the word where he moves on, not on the last word you animate.
- Counters: `to` is the number; `prefix` and `suffix` are a symbol or unit only ($, +, %, x, k, M, wpm), never digits (`prefix: "250+ "` shows "250+ 250").
- Keep it moving: give the payoff a slow push-in, `keys: [{t: wN, ms: 4000, scale: 1.06}]`, so nothing sits still for more than about 4 s.
- Emphasis: the one element that matters gets `fill: accent_soft, stroke: accent, width: 4`; the rest stay plain (`fill: card2`). 4 to 9 layers. Coordinates are 0..1 of the panel from its top-left; text `x` is its centre with `align: center`.

Example, a comparison with the numbers as the payoff:
  - {at: w1200, until: w1226, kind: scene, layers: [
      {type: text, text: "By hand vs *with agents*", x: 0.06, y: 0.07, size: 96, weight: bold},
      {type: box, x: 0.05, y: 0.25, w: 0.42, h: 0.55, fill: card2, in: {at: w1202, from: left}},
      {type: text, text: "By hand", x: 0.26, y: 0.31, size: 56, weight: semibold, align: center, in: {at: w1202}},
      {type: counter, from: 0, to: 6, suffix: " h", x: 0.26, y: 0.43, size: 170, color: ink_soft, in: {at: w1206}},
      {type: text, text: "per video", x: 0.26, y: 0.66, size: 36, align: center, color: muted, in: {at: w1207}},
      {type: box, x: 0.53, y: 0.25, w: 0.42, h: 0.55, fill: accent_soft, stroke: accent, width: 4, in: {at: w1219, from: right}},
      {type: text, text: "*With agents*", x: 0.74, y: 0.31, size: 56, weight: semibold, align: center, in: {at: w1219}},
      {type: counter, from: 60, to: 20, suffix: " min", x: 0.74, y: 0.43, size: 170, in: {at: w1223}, keys: [{t: w1224, ms: 3000, scale: 1.08}]},
      {type: text, text: "per video", x: 0.74, y: 0.66, size: 36, align: center, color: muted, in: {at: w1224}}]}

Example, a hub: many things feeding one:
  - {at: w2400, until: w2415, kind: scene, layers: [
      {type: text, text: "One *inbox* for everything", x: 0.06, y: 0.07, size: 92, weight: bold, style: marker},
      {id: hub, type: circle, x: 0.5, y: 0.44, r: 0.15, fill: accent_soft, stroke: accent, width: 5, label: "Your agent", size: 56, in: {at: w2400, from: pop}, keys: [{t: w2410, ms: 4000, scale: 1.05}]},
      {id: a, type: box, x: 0.05, y: 0.72, w: 0.27, h: 0.17, label: Gmail, size: 52, in: {at: w2405}},
      {id: b, type: box, x: 0.365, y: 0.72, w: 0.27, h: 0.17, label: Slack, size: 52, in: {at: w2406}},
      {id: c, type: box, x: 0.68, y: 0.72, w: 0.27, h: 0.17, label: X, size: 52, in: {at: w2408}},
      {type: arrow, from: a, to: hub, in: {at: w2411}},
      {type: arrow, from: b, to: hub, in: {at: w2412}},
      {type: arrow, from: c, to: hub, in: {at: w2413}}]}

Example, a hierarchy: one thing directing others:
  - {at: w3500, until: w3549, kind: scene, layers: [
      {type: text, text: "The *planner* runs the team", x: 0.06, y: 0.07, size: 92, weight: bold, style: marker},
      {id: boss, type: box, x: 0.25, y: 0.27, w: 0.5, h: 0.22, fill: accent_soft, stroke: accent, width: 4, label: Planner, sub: "splits the work", size: 58, in: {at: w3503, from: pop}, keys: [{t: w3530, ms: 4000, scale: 1.04}]},
      {id: w1, type: box, x: 0.05, y: 0.66, w: 0.27, h: 0.2, label: Researcher, sub: "finds sources", size: 52, in: {at: w3511}},
      {id: w2, type: box, x: 0.365, y: 0.66, w: 0.27, h: 0.2, label: Writer, sub: "drafts the script", size: 52, in: {at: w3514}},
      {id: w3, type: box, x: 0.68, y: 0.66, w: 0.27, h: 0.2, label: Editor, sub: "cuts the video", size: 52, in: {at: w3517}},
      {type: arrow, from: boss, to: w1, in: {at: w3511}},
      {type: arrow, from: boss, to: w2, in: {at: w3514}},
      {type: arrow, from: boss, to: w3, in: {at: w3517}}]}
Example, a stack built bottom-up:
  - {at: w4100, until: w4140, kind: scene, layers: [
      {type: text, text: "Four layers, one *system*", x: 0.06, y: 0.07, size: 92, weight: bold, style: marker},
      {type: box, x: 0.1, y: 0.74, w: 0.8, h: 0.15, label: Context, sub: "who you are", size: 52, in: {at: w4102, from: up}},
      {type: box, x: 0.1, y: 0.565, w: 0.8, h: 0.15, label: Connectors, sub: "your apps", size: 52, in: {at: w4106, from: up}},
      {type: box, x: 0.1, y: 0.39, w: 0.8, h: 0.15, label: Skills, sub: "how you work", size: 52, in: {at: w4110, from: up}},
      {type: box, x: 0.1, y: 0.215, w: 0.8, h: 0.15, fill: accent_soft, stroke: accent, width: 4, label: "*Routines*", sub: "runs while you sleep", size: 52, in: {at: w4114, from: pop}, keys: [{t: w4118, ms: 4000, scale: 1.04}]}]}

Example, a stat beside its proof (the file must be one listed above):
  - {at: w5200, until: w5230, kind: scene, layers: [
      {type: image, src: proof/launchpoint-rates.png, x: 0.05, y: 0.12, w: 0.56, h: 0.76, source: "launchpoint.app", ring: [0.05, 0.2, 0.4, 0.1], in: {at: w5200, from: scale}, keys: [{t: w5203, ms: 4500, scale: 1.05}]},
      {type: text, text: "PAID FOR ONE VIDEO", x: 0.65, y: 0.3, size: 32, weight: bold, color: accent, in: {at: w5204}},
      {type: counter, to: 10075, prefix: "$", x: 0.65, y: 0.36, size: 140, align: left, in: {at: w5206}},
      {type: text, text: "LaunchPoint's rate for *one* AI video", x: 0.65, y: 0.58, w: 0.3, size: 40, color: ink_soft, in: {at: w5209}}]}

Other layer types: {type: line, points: [[x, y], ...]}, {type: image, src: <file>, x, y, w, h, ring: [x, y, w, h]}, {type: code, text: "...", x, y, w}, {type: highlight, x, y, w, h, style: marker} (list it before the layer it sits behind). Colours: accent, accent_soft, ink, ink_soft, muted, card, card2, green, red, gold or #hex.
"#;

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
