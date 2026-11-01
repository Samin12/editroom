//! `cutroom check`: everything that can go wrong in a plan, found without rendering a frame.

use crate::compose::{empty_panel_ranges, Seg};
use crate::model::Source;
use crate::resolve::{clock, Issue, Plan, Severity};
use crate::spec::{BeatKind, EmptyPanel, Lane, LayoutMode, RectN};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Something private seen on screen (from `cutroom privacy`). Source time, screen-normalized rect.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Finding {
    pub src0_ms: f64,
    pub src1_ms: f64,
    pub rect: RectN,
    /// email | api_key | token | phone | card | custom
    pub kind: String,
    /// Redacted preview ("sk-ab12...").
    pub preview: String,
}

fn issue(sev: Severity, at: Option<f64>, msg: String) -> Issue {
    Issue { sev, at_ms: at, msg }
}

fn overlap(a: &RectN, b: &RectN) -> f64 {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    ((x1 - x0).max(0.0) * (y1 - y0).max(0.0)) / (a[2] * a[3]).max(1e-9)
}

pub fn check(plan: &Plan, src: &Source, segs: &[Seg], spec_dir: &Path, findings: &[Finding]) -> Vec<Issue> {
    let mut out = plan.issues.clone();

    // empty panel
    for (a, b) in empty_panel_ranges(segs) {
        if b - a < 1000.0 / plan.fps {
            continue;
        }
        let sev = if plan.layout.empty_panel == EmptyPanel::Error { Severity::Error } else { Severity::Info };
        out.push(issue(
            sev,
            Some(a),
            format!("empty panel {}-{} ({:.1}s): no screen recording and no stage beat{}", clock(a), clock(b), (b - a) / 1000.0, if sev == Severity::Info { "; showing the full camera" } else { "" }),
        ));
    }
    // camera tile with no camera
    let mut no_cam = 0.0;
    let mut first = None;
    for s in segs.iter().filter(|s| s.mode == LayoutMode::Panel && s.card.is_none() && s.camera.is_none() && s.src0.is_some()) {
        no_cam += s.out1 - s.out0;
        first.get_or_insert(s.out0);
    }
    if no_cam > 1000.0 {
        out.push(issue(Severity::Warn, first, format!("{:.1}s of panel layout has no camera (tile hidden)", no_cam / 1000.0)));
    }

    // beats
    let stage: Vec<_> = plan.beats.iter().filter(|b| b.lane == Lane::Stage).collect();
    for w in stage.windows(2) {
        if w[1].start_ms < w[0].end_ms - 1.0 {
            out.push(issue(Severity::Warn, Some(w[1].start_ms), format!("{} ({}) starts before {} ({}) ends: it cuts the earlier one short", w[1].kind.name(), w[1].origin, w[0].kind.name(), w[0].origin)));
        }
    }
    for b in &plan.beats {
        let d = b.end_ms - b.start_ms;
        if b.lane == Lane::Stage && d < 1500.0 {
            out.push(issue(Severity::Warn, Some(b.start_ms), format!("{} ({}) is on screen {:.1}s; beats need about 2s to read", b.kind.name(), b.origin, d / 1000.0)));
        }
        let path = match &b.kind {
            BeatKind::Image { src, .. } | BeatKind::Video { src, .. } => Some(src),
            _ => None,
        };
        if let Some(p) = path {
            if !spec_dir.join(p).exists() {
                out.push(issue(Severity::Error, Some(b.start_ms), format!("{} ({}): file {p:?} not found", b.kind.name(), b.origin)));
            }
        }
        // overlays on the screen need the screen to be visible
        if matches!(b.kind, BeatKind::Ring { .. } | BeatKind::Highlight { .. } | BeatKind::Arrow { .. } | BeatKind::Zoom { .. } | BeatKind::Blur { .. }) {
            let mid = (b.start_ms + b.end_ms) / 2.0;
            if let Some(s) = segs.iter().find(|s| s.out0 <= mid && mid < s.out1) {
                if s.screen.is_none() || s.stage.is_some() || s.mode == LayoutMode::Camera {
                    out.push(issue(Severity::Warn, Some(b.start_ms), format!("{} ({}) at {}: the screen recording is not visible there", b.kind.name(), b.origin, clock(b.start_ms))));
                }
            }
        }
        for r in rects(&b.kind) {
            if r[2] <= 0.0 || r[3] <= 0.0 || r[0] < -0.01 || r[1] < -0.01 || r[0] + r[2] > 1.01 || r[1] + r[3] > 1.01 {
                out.push(issue(Severity::Error, Some(b.start_ms), format!("{} ({}): rect {r:?} must be [x, y, w, h] inside 0..1", b.kind.name(), b.origin)));
            }
        }
    }
    for c in &plan.chapters {
        if let Some((a, b)) = c.card {
            if b - a > 2000.0 + 1.0 {
                out.push(issue(Severity::Warn, Some(a), format!("chapter card {:?} is {:.1}s; keep cards at 2s max", c.title, (b - a) / 1000.0)));
            }
        }
    }

    // privacy: every finding that is on screen in the output must be blurred or hidden
    for f in findings {
        // walk the output segments overlapping the finding
        let mut exposed: Option<(f64, f64)> = None;
        for s in segs {
            let (Some(s0), Some(sc)) = (s.src0, s.screen) else { continue };
            let s1 = s0 + (s.out1 - s.out0);
            let a = f.src0_ms.max(s0);
            let b = f.src1_ms.min(s1);
            if b <= a {
                continue;
            }
            // screen clip must be the one the finding is on: findings are per source time, so any
            // screen clip at that time is the one OCR saw
            let _ = sc;
            if s.stage.is_some() || s.card.is_some() || matches!(s.mode, LayoutMode::Camera) {
                continue;
            }
            let oa = s.out0 + (a - s0);
            let ob = s.out0 + (b - s0);
            // covered when blur beats cover the whole exposed window
            let covered = covered_by_blur(plan, &f.rect, oa, ob);
            if !covered {
                exposed = Some(match exposed {
                    Some((x, y)) => (x.min(oa), y.max(ob)),
                    None => (oa, ob),
                });
            }
        }
        if let Some((a, b)) = exposed {
            out.push(issue(Severity::Error, Some(a), format!("privacy: {} {} visible {}-{} at screen rect {:?}; add a blur beat", f.kind, f.preview, clock(a), clock(b), round_rect(&f.rect))));
        }
    }
    let _ = src;
    out.sort_by(|a, b| a.sev.cmp(&b.sev).then(a.at_ms.unwrap_or(0.0).total_cmp(&b.at_ms.unwrap_or(0.0))));
    out
}

fn round_rect(r: &RectN) -> [f64; 4] {
    [r[0], r[1], r[2], r[3]].map(|v| (v * 1000.0).round() / 1000.0)
}

fn rects(k: &BeatKind) -> Vec<RectN> {
    match k {
        BeatKind::Ring { rect } | BeatKind::Highlight { rect, .. } | BeatKind::Zoom { rect } => vec![*rect],
        BeatKind::Blur { rects } => rects.clone(),
        _ => vec![],
    }
}

/// Is `rect` blurred during the whole output window [a, b)?
fn covered_by_blur(plan: &Plan, rect: &RectN, a: f64, b: f64) -> bool {
    let mut spans: Vec<(f64, f64)> = plan
        .beats
        .iter()
        .filter(|x| match &x.kind {
            BeatKind::Blur { rects } => rects.iter().any(|r| overlap(rect, r) > 0.95),
            _ => false,
        })
        .map(|x| (x.start_ms, x.end_ms))
        .collect();
    spans.sort_by(|p, q| p.0.total_cmp(&q.0));
    let frame = 1000.0 / plan.fps;
    let mut t = a;
    for (x, y) in spans {
        if x > t + frame / 2.0 {
            break;
        }
        t = t.max(y);
        if t >= b - frame / 2.0 {
            return true;
        }
    }
    t >= b - frame / 2.0
}

/// One line per issue: "error 12:03.4 message".
pub fn format(issues: &[Issue]) -> String {
    let mut s = String::new();
    for i in issues {
        let sev = match i.sev {
            Severity::Error => "error",
            Severity::Warn => "warn ",
            Severity::Info => "info ",
        };
        s.push_str(&format!("{sev} {:>9} {}\n", i.at_ms.map(clock).unwrap_or_else(|| "-".into()), i.msg));
    }
    s
}
