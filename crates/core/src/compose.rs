//! What every output instant shows. The renderer draws exactly these segments and `check` lints
//! exactly these segments, so the checker can never disagree with the picture.

use crate::model::{Source, Track};
use crate::resolve::Plan;
use crate::spec::{BeatKind, EmptyPanel, Lane, LayoutMode};
use crate::timemap::Piece;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Seg {
    pub out0: f64,
    pub out1: f64,
    /// Source time at `out0` (None on a chapter card).
    pub src0: Option<f64>,
    /// Layout asked for (default or a `layout` beat).
    pub requested: LayoutMode,
    /// Layout drawn (an empty panel falls back to the camera).
    pub mode: LayoutMode,
    /// Chapter card (index into plan.chapters) when this is a card.
    pub card: Option<usize>,
    /// Stage beat filling the panel (index into plan.beats).
    pub stage: Option<usize>,
    /// Index into source.clips.
    pub screen: Option<usize>,
    pub camera: Option<usize>,
    /// The panel had nothing to show here (no screen, no stage beat).
    pub empty_panel: bool,
}

impl Seg {
    pub fn src_at(&self, t: f64) -> Option<f64> {
        self.src0.map(|s| s + (t - self.out0))
    }
}

fn clip_idx_at(src: &Source, track: Track, t: f64) -> Option<usize> {
    src.clips
        .iter()
        .enumerate()
        .filter(|(_, c)| c.track == track && c.contains(t))
        .max_by(|a, b| a.1.start_ms.total_cmp(&b.1.start_ms))
        .map(|(i, _)| i)
}

pub fn segments(plan: &Plan, src: &Source) -> Vec<Seg> {
    let frame = 1000.0 / plan.fps;
    let mut cuts: Vec<f64> = vec![0.0, plan.duration_ms()];
    for p in &plan.map.pieces {
        cuts.push(p.out0());
        cuts.push(p.out1());
        if let Piece::Src { src0, src1, out0 } = *p {
            for c in &src.clips {
                if !c.track.is_video() {
                    continue;
                }
                for e in [c.start_ms, c.end_ms()] {
                    if e > src0 && e < src1 {
                        cuts.push(out0 + (e - src0));
                    }
                }
            }
        }
    }
    for b in &plan.beats {
        if matches!(b.lane, Lane::Stage | Lane::Layout) {
            cuts.push(b.start_ms);
            cuts.push(b.end_ms);
        }
    }
    for c in &plan.chapters {
        if let Some((a, b)) = c.card {
            cuts.push(a);
            cuts.push(b);
        }
    }
    cuts.retain(|t| *t >= 0.0 && *t <= plan.duration_ms());
    cuts.sort_by(|a, b| a.total_cmp(b));
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut out: Vec<Seg> = vec![];
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b - a < 1e-6 {
            continue;
        }
        let mid = (a + b) / 2.0;
        let src0 = plan.map.to_src(a).or_else(|| plan.map.to_src(mid).map(|s| s - (mid - a)));
        let card = plan.chapters.iter().position(|c| c.card.map(|(x, y)| mid >= x && mid < y).unwrap_or(false));
        // inserted cards fill a gap; overlay cards are drawn over the segment
        let stage = plan
            .beats
            .iter()
            .enumerate()
            .filter(|(_, x)| x.lane == Lane::Stage && x.start_ms <= mid && mid < x.end_ms)
            .max_by(|p, q| p.1.start_ms.total_cmp(&q.1.start_ms))
            .map(|(i, _)| i);
        let requested = plan
            .beats
            .iter()
            .filter(|x| x.start_ms <= mid && mid < x.end_ms)
            .filter_map(|x| match x.kind {
                BeatKind::Layout { mode } => Some((x.start_ms, mode)),
                _ => None,
            })
            .max_by(|p, q| p.0.total_cmp(&q.0))
            .map(|x| x.1)
            .unwrap_or(plan.layout.default);
        let smid = plan.map.to_src(mid);
        let screen = smid.and_then(|s| clip_idx_at(src, Track::Screen, s));
        let camera = smid.and_then(|s| clip_idx_at(src, Track::Camera, s));
        let in_gap = smid.is_none();
        let empty_panel = !in_gap && requested == LayoutMode::Panel && screen.is_none() && stage.is_none();
        let mut mode = requested;
        if empty_panel && plan.layout.empty_panel == EmptyPanel::Camera {
            mode = LayoutMode::Camera;
        }
        if mode == LayoutMode::Screen && screen.is_none() && stage.is_none() && !in_gap {
            mode = LayoutMode::Camera;
        }
        let seg = Seg { out0: a, out1: b, src0, requested, mode, card, stage, screen, camera, empty_panel };
        // merge with the previous segment when nothing changes (and the source is continuous)
        if let Some(last) = out.last_mut() {
            let cont = match (last.src0, seg.src0) {
                (Some(s0), Some(s1)) => (s0 + (last.out1 - last.out0) - s1).abs() < 0.01,
                (None, None) => true,
                _ => false,
            };
            if cont && last.requested == seg.requested && last.mode == seg.mode && last.card == seg.card && last.stage == seg.stage && last.screen == seg.screen && last.camera == seg.camera && last.empty_panel == seg.empty_panel {
                last.out1 = seg.out1;
                continue;
            }
        }
        out.push(seg);
    }
    // drop sub-frame slivers into their neighbour (no output frame samples them)
    let mut merged: Vec<Seg> = vec![];
    for s in out {
        let has_frame = {
            let k0 = (s.out0 / frame).ceil();
            k0 * frame < s.out1 - 1e-9
        };
        if !has_frame {
            if let Some(last) = merged.last_mut() {
                last.out1 = s.out1;
                continue;
            }
        }
        merged.push(s);
    }
    merged
}

/// Total ms with an empty panel, and the ranges.
pub fn empty_panel_ranges(segs: &[Seg]) -> Vec<(f64, f64)> {
    let mut r: Vec<(f64, f64)> = vec![];
    for s in segs.iter().filter(|s| s.empty_panel) {
        match r.last_mut() {
            Some(l) if (l.1 - s.out0).abs() < 1e-6 => l.1 = s.out1,
            _ => r.push((s.out0, s.out1)),
        }
    }
    r
}
