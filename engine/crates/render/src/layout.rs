//! What each stretch of the output shows where: `cutroom plan --segments`. Built from the same
//! segments and [`frame::visible`] the compositor draws from, so QA can look at the right pixels
//! (the camera tile only when there is one, the whole frame in camera layout).

use crate::frame::{self, Visible};
use cutroom_core::compose::Seg;
use cutroom_core::model::Source;
use cutroom_core::spec::{BeatKind, LayoutMode, VideoFit};
use cutroom_core::Plan;
use serde::Serialize;

/// One stretch of output with one layout.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg {
    pub start_ms: f64,
    pub end_ms: f64,
    /// `panel`, `screen`, `camera`, or `card` (a chapter card covers the frame).
    pub mode: &'static str,
    /// The camera in output pixels `[x, y, w, h]`: the tile in panel layout, the whole frame in
    /// camera layout; null when no camera is drawn.
    pub camera_rect: Option<[i32; 4]>,
    /// The screen recording in output pixels; null when it is not drawn (camera layout, a stage
    /// beat in the panel, a card).
    pub screen_rect: Option<[i32; 4]>,
    /// The stage beat filling the panel instead of the screen (`title`, `image`, `scene`, ...).
    pub stage: Option<&'static str>,
    /// A `fit: frame` video drawn over the screen and stage (under the camera tile).
    pub frame_video: Option<String>,
    /// Media ids and takes of the screen and camera clips shown in this stretch.
    pub screen_media: Vec<String>,
    pub camera_media: Vec<String>,
    pub takes: Vec<u32>,
}

fn mode_name(v: &Visible, mode: LayoutMode) -> &'static str {
    if v.card {
        return "card";
    }
    match mode {
        LayoutMode::Panel => "panel",
        LayoutMode::Screen => "screen",
        LayoutMode::Camera => "camera",
    }
}

fn push_unique<T: PartialEq>(v: &mut Vec<T>, x: T) {
    if !v.contains(&x) {
        v.push(x);
    }
}

/// The output as stretches of constant layout. Adjacent stretches with the same mode, rects,
/// stage beat and full-frame video are merged (their clips are listed together).
pub fn layout_segments(plan: &Plan, src: &Source, segs: &[Seg]) -> Vec<LayoutSeg> {
    // full-frame videos are overlays, so compose does not split at them: split here
    let fv: Vec<_> = plan.beats.iter().filter(|b| matches!(b.kind, BeatKind::Video { fit: VideoFit::Frame, .. })).collect();
    let mut out: Vec<LayoutSeg> = vec![];
    for s in segs {
        let v = frame::visible(plan, s);
        let mut edges = vec![s.out0, s.out1];
        for b in &fv {
            for e in [b.start_ms, b.end_ms] {
                if e > s.out0 && e < s.out1 {
                    edges.push(e);
                }
            }
        }
        edges.sort_by(f64::total_cmp);
        for w in edges.windows(2) {
            let (a, b) = (w[0], w[1]);
            if b - a < 1e-6 {
                continue;
            }
            let mid = (a + b) / 2.0;
            // the latest-starting one on screen, as the compositor picks it
            let frame_video = if v.card {
                None
            } else {
                fv.iter().filter(|x| x.start_ms <= mid && mid < x.end_ms).max_by(|p, q| p.start_ms.total_cmp(&q.start_ms)).and_then(|x| match &x.kind {
                    BeatKind::Video { src, .. } => Some(src.clone()),
                    _ => None,
                })
            };
            let mut seg = LayoutSeg {
                start_ms: a,
                end_ms: b,
                mode: mode_name(&v, s.mode),
                camera_rect: v.camera,
                screen_rect: v.screen,
                stage: if v.card { None } else { s.stage.map(|i| plan.beats[i].kind.name()) },
                frame_video,
                screen_media: vec![],
                camera_media: vec![],
                takes: vec![],
            };
            if let (Some(_), Some(i)) = (v.screen, s.screen) {
                push_unique(&mut seg.screen_media, src.clips[i].media.clone());
                push_unique(&mut seg.takes, src.clips[i].take);
            }
            if let (Some(_), Some(i)) = (v.camera, s.camera) {
                push_unique(&mut seg.camera_media, src.clips[i].media.clone());
                push_unique(&mut seg.takes, src.clips[i].take);
            }
            match out.last_mut() {
                Some(l)
                    if (l.end_ms - a).abs() < 1e-6
                        && l.mode == seg.mode
                        && l.camera_rect == seg.camera_rect
                        && l.screen_rect == seg.screen_rect
                        && l.stage == seg.stage
                        && l.frame_video == seg.frame_video =>
                {
                    l.end_ms = b;
                    for m in seg.screen_media {
                        push_unique(&mut l.screen_media, m);
                    }
                    for m in seg.camera_media {
                        push_unique(&mut l.camera_media, m);
                    }
                    for t in seg.takes {
                        push_unique(&mut l.takes, t);
                    }
                }
                _ => out.push(seg),
            }
        }
    }
    out
}
