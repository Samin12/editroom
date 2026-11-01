use super::*;
use cutroom_core::{Clip, Media, Track, Word};

/// A one-take source from text: 250 ms words, 50 ms apart; `|` is a 1.2 s pause. The take runs
/// from 0 to one second after the last word; words start at `lead_ms`.
fn source_from(takes: &[(&str, f64, f64)]) -> Source {
    let mut s = Source { version: 1, name: "t".into(), ..Default::default() };
    let mut t0 = 0.0;
    for (k, (text, lead_ms, step)) in takes.iter().enumerate() {
        let mut t = t0 + lead_ms;
        for tok in text.split_whitespace() {
            if tok == "|" {
                t += 1200.0;
                continue;
            }
            s.words.push(Word { text: tok.into(), start_ms: t, end_ms: t + step - 50.0 });
            t += step;
        }
        let end = t + 1000.0;
        let id = format!("mic{k}");
        s.media.push(Media { id: id.clone(), path: format!("{id}.wav").into(), width: 0, height: 0, fps: 0.0, duration_ms: end - t0 });
        s.clips.push(Clip { track: Track::Mic, media: id, start_ms: t0, dur_ms: end - t0, media_in_ms: 0.0, take: k as u32 });
        t0 = end;
    }
    s.duration_ms = t0;
    s
}

fn source(text: &str) -> Source {
    source_from(&[(text, 0.0, 300.0)])
}

fn has(ps: &[Proposal], why: Why, a: usize, b: usize) -> bool {
    ps.iter().any(|p| p.why == why && p.span == Span::Words(a, b))
}

#[test]
fn partial_restart_is_cut_automatically() {
    let s = source("So now let's get you set up. I'm gonna be walking through I'm gonna be walking you through the whole setup step by step.");
    let r = detect(&s, &CutOpts::default());
    assert!(has(&r.auto, Why::Retake, 7, 11), "{:#?}", r.auto);
}

#[test]
fn restart_of_a_sentence_keeps_the_last_attempt() {
    let s = source("This is the video. So the ones who win are the people are the ones pe so the ones who win are the ones people can trust.");
    let r = detect(&s, &CutOpts::default());
    let all: Vec<Proposal> = r.auto.iter().chain(&r.review).cloned().collect();
    assert!(has(&all, Why::Retake, 4, 15), "{all:#?}");
    // the keeper is untouched
    assert!(all.iter().all(|p| !matches!(p.span, Span::Words(_, b) if b >= 16)));
}

#[test]
fn fillers_and_stutters() {
    let s = source("We can uh use it. All of this research can be can be done on autopilot.");
    let r = detect(&s, &CutOpts::default());
    assert!(has(&r.auto, Why::Filler, 2, 2), "{:#?}", r.auto);
    assert!(has(&r.auto, Why::FalseStart, 9, 10), "{:#?}", r.auto);
}

#[test]
fn emphasis_and_idioms_are_not_stutters() {
    let s = source("This is very, very important and it works really really well. Do it bit by bit by bit. Look at this. This is it.");
    let r = detect(&s, &CutOpts::default());
    let words: Vec<&Proposal> = r.auto.iter().chain(&r.review).filter(|p| matches!(p.span, Span::Words(..))).collect();
    assert!(words.is_empty(), "{words:#?}");
}

#[test]
fn parallel_sentences_are_not_auto_cut() {
    let s = source("You can use it for writing. You can use it for coding. And you can use it for research.");
    let r = detect(&s, &CutOpts::default());
    assert!(r.auto.iter().all(|p| p.why != Why::Retake), "{:#?}", r.auto);
}

#[test]
fn silences_shrink_and_dead_air_goes() {
    let s = source_from(&[("Hello there. | How are you?", 2000.0, 300.0)]);
    let r = detect(&s, &CutOpts::default());
    let w = &s.words;
    let sil: Vec<_> = r.auto.iter().filter(|p| p.why == Why::Silence).collect();
    assert_eq!(sil.len(), 1);
    let Span::Ms(a, b) = sil[0].span else { panic!() };
    assert!((a - (w[1].end_ms + 120.0)).abs() < 1e-6 && (b - (w[2].start_ms - 120.0)).abs() < 1e-6);
    let dead: Vec<_> = r.auto.iter().filter(|p| p.why == Why::DeadAir).map(|p| p.span).collect();
    assert_eq!(dead.len(), 2, "{dead:?}");
    assert_eq!(dead[0], Span::Ms(0.0, 2000.0 - 250.0));
    // never inside a word
    for (a, b) in r.ranges(&s, true) {
        assert!(w.iter().all(|x| x.end_ms <= a || x.start_ms >= b || (x.start_ms >= a && x.end_ms <= b)), "range {a}..{b} splits a word");
    }
}

#[test]
fn abandoned_take_but_not_simultaneous_takes() {
    let first = "Hey everyone, today I want to show you how to set up the agent team really quickly.";
    let again = "Hey everyone, today I want to show you how to set up the whole agent team really quickly.";
    let s = source_from(&[(first, 500.0, 300.0), (again, 800.0, 340.0)]);
    let r = detect(&s, &CutOpts::default());
    let n0 = first.split_whitespace().count();
    let all: Vec<Proposal> = r.auto.iter().chain(&r.review).cloned().collect();
    assert!(all.iter().any(|p| p.why == Why::Retake && p.span == Span::Words(0, n0 - 1)), "{all:#?}");
    // the same words at the same moments = two cameras of one performance: keep both
    let s = source_from(&[(first, 500.0, 300.0), (first, 500.0, 300.0)]);
    let r = detect(&s, &CutOpts::default());
    assert!(r.auto.iter().chain(&r.review).all(|p| p.why != Why::Retake), "{:#?}", r.auto);
}

#[test]
fn yaml_loads_into_a_plan() {
    let s = source("So now let's get you set up. I'm gonna be walking through I'm gonna be walking you through the whole uh setup. | Done.");
    let r = detect(&s, &CutOpts::default());
    let dir = std::env::temp_dir().join(format!("cutroom-cut-yaml-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    s.save(&dir.join("source.json")).unwrap();
    std::fs::write(dir.join("cuts.yaml"), to_yaml(&r)).unwrap();
    std::fs::write(dir.join("edit.yaml"), "include: [cuts.yaml]\n").unwrap();
    let loaded = cutroom_core::spec::load(&dir.join("edit.yaml")).unwrap();
    assert_eq!(loaded.spec.cuts.len(), r.auto.len());
    let plan = cutroom_core::resolve::build(&loaded, &s);
    assert_eq!(plan.errors(), 0, "{:?}", plan.issues);
    let kept: Vec<usize> = plan.words.iter().map(|w| w.id).collect();
    // w7..w11 is the abandoned attempt, w20 the "uh"
    assert!(!kept.contains(&7) && !kept.contains(&11) && !kept.contains(&20), "{kept:?}");
    assert!(kept.contains(&12) && kept.contains(&17) && kept.contains(&21) && kept.contains(&22));
    assert!(plan.duration_ms() < s.duration_ms - 2000.0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn review_md_is_compact() {
    let s = source("Intro words here to give context before it. So now let's get you set up. I'm gonna be walking through I'm gonna be walking you through the whole setup step by step and then we start.");
    let opts = CutOpts { auto_min: 1.1, ..CutOpts::default() };
    let r = detect(&s, &opts);
    assert!(r.auto.iter().all(|p| matches!(p.span, Span::Ms(..))));
    let md = review_md(&r, &s);
    let line = md.lines().find(|l| l.starts_with("r1 ")).unwrap();
    assert!(line.starts_with("r1 retake w15-w19: "), "{line}");
    assert!(line.contains("[[I'm gonna be walking through]] I'm gonna"), "{line}");
    let before = line.split("[[").next().unwrap().split(": ").nth(1).unwrap();
    assert_eq!(before.split_whitespace().count(), 15);
    assert!(md.len() < 600, "{} bytes", md.len());
}

#[test]
fn three_hours_well_under_a_second() {
    // ~32k words of varied speech with periodic restarts
    let vocab = ["the", "video", "idea", "claude", "board", "title", "thumbnail", "make", "sure", "you", "can", "see", "this", "right", "here", "and", "we", "want", "to", "build", "agent", "team", "so", "now", "let's", "go", "click", "open", "it", "works"];
    let mut text = String::new();
    let mut x: u64 = 7;
    for i in 0..32_000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        text.push_str(vocab[(x >> 33) as usize % vocab.len()]);
        text.push(if i % 13 == 12 { '.' } else { ' ' });
        if i % 13 == 12 {
            text.push(' ');
        }
    }
    let s = source(&text);
    let t = std::time::Instant::now();
    let r = detect(&s, &CutOpts::default());
    let el = t.elapsed();
    assert!(!r.auto.is_empty());
    // debug build; release is ~20 ms for 4 h (see EVAL.md)
    assert!(el.as_secs_f64() < 3.0, "{el:?}");
}
