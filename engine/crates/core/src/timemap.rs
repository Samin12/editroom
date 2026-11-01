//! Source -> output time. Built from the cuts (source ranges removed) and inserts (cards that add
//! output time). Every piece is a whole number of output frames, so video and audio cut at the
//! same instants and every piece boundary is a frame boundary.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Piece {
    /// Source `[src0, src1)` plays at output `out0`.
    Src { src0: f64, src1: f64, out0: f64 },
    /// `dur` ms of output with no source (a chapter card). `tag` = chapter index.
    Gap { out0: f64, dur: f64, tag: usize },
}

impl Piece {
    pub fn out0(&self) -> f64 {
        match *self {
            Piece::Src { out0, .. } | Piece::Gap { out0, .. } => out0,
        }
    }
    pub fn out1(&self) -> f64 {
        match *self {
            Piece::Src { src0, src1, out0 } => out0 + (src1 - src0),
            Piece::Gap { out0, dur, .. } => out0 + dur,
        }
    }
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct TimeMap {
    pub fps: f64,
    pub pieces: Vec<Piece>,
    pub duration_ms: f64,
}

/// Merge overlapping/adjacent ranges, sorted.
pub fn merge(mut r: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    r.retain(|(a, b)| b > a);
    r.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = vec![];
    for (a, b) in r {
        if let Some(last) = out.last_mut() {
            if a <= last.1 {
                last.1 = last.1.max(b);
                continue;
            }
        }
        out.push((a, b));
    }
    out
}

impl TimeMap {
    /// `cuts`: source ranges to remove. `inserts`: (source time, duration, tag) cards placed just
    /// before that source instant. Pieces shorter than one frame are dropped.
    pub fn build(src_dur: f64, fps: f64, cuts: &[(f64, f64)], inserts: &[(f64, f64, usize)]) -> TimeMap {
        let frame = 1000.0 / fps;
        let cuts = merge(cuts.iter().map(|&(a, b)| (a.max(0.0), b.min(src_dur))).collect());
        // kept ranges
        let mut kept = vec![];
        let mut t = 0.0;
        for &(a, b) in &cuts {
            if a > t {
                kept.push((t, a));
            }
            t = t.max(b);
        }
        if t < src_dur {
            kept.push((t, src_dur));
        }
        // split kept ranges at inserts
        let mut ins: Vec<(f64, f64, usize)> = inserts.to_vec();
        ins.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut pieces = vec![];
        let mut out = 0.0;
        let mut ii = 0;
        for (a, b) in kept {
            let mut s = a;
            while ii < ins.len() && ins[ii].0 < b {
                let (at, dur, tag) = ins[ii];
                let at = at.max(s);
                if at > s {
                    push_src(&mut pieces, &mut out, s, at, frame);
                    s = at;
                }
                let d = (dur / frame).round().max(1.0) * frame;
                pieces.push(Piece::Gap { out0: out, dur: d, tag });
                out += d;
                ii += 1;
            }
            push_src(&mut pieces, &mut out, s, b, frame);
        }
        // inserts past the end
        while ii < ins.len() {
            let (_, dur, tag) = ins[ii];
            let d = (dur / frame).round().max(1.0) * frame;
            pieces.push(Piece::Gap { out0: out, dur: d, tag });
            out += d;
            ii += 1;
        }
        TimeMap { fps, pieces, duration_ms: out }
    }

    pub fn frames(&self) -> u64 {
        (self.duration_ms * self.fps / 1000.0).round() as u64
    }

    /// Index (into `pieces`) of the Src piece containing source time `s`, or Err(index of the
    /// first Src piece starting after `s`, or pieces.len()).
    fn idx_src(&self, s: f64) -> Result<usize, usize> {
        let srcs: Vec<(usize, f64, f64)> = self.src_pieces();
        let j = srcs.partition_point(|&(_, a, _)| a <= s);
        if j > 0 {
            let (i, a, b) = srcs[j - 1];
            if s >= a && s < b {
                return Ok(i);
            }
        }
        Err(srcs.get(j).map(|x| x.0).unwrap_or(self.pieces.len()))
    }

    fn src_pieces(&self) -> Vec<(usize, f64, f64)> {
        self.pieces
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match *p {
                Piece::Src { src0, src1, .. } => Some((i, src0, src1)),
                _ => None,
            })
            .collect()
    }

    /// Is source time `s` kept?
    pub fn is_kept(&self, s: f64) -> bool {
        self.idx_src(s).is_ok()
    }

    /// Output time of source time `s`. A time inside a cut maps to the output instant of the cut.
    pub fn to_out(&self, s: f64) -> f64 {
        match self.idx_src(s) {
            Ok(i) => match self.pieces[i] {
                Piece::Src { src0, out0, .. } => out0 + (s - src0),
                _ => unreachable!(),
            },
            Err(next) => match self.pieces.get(next) {
                Some(Piece::Src { out0, .. }) => *out0,
                _ => self.duration_ms,
            },
        }
    }

    /// The piece playing at output time `o`.
    pub fn piece_at(&self, o: f64) -> Option<&Piece> {
        let i = self.pieces.partition_point(|p| p.out1() <= o);
        self.pieces.get(i)
    }

    /// Source time at output time `o` (None inside a gap or past the end).
    pub fn to_src(&self, o: f64) -> Option<f64> {
        match self.piece_at(o)? {
            Piece::Src { src0, out0, .. } => Some(src0 + (o - out0)),
            Piece::Gap { .. } => None,
        }
    }

    pub fn kept(&self) -> impl Iterator<Item = (f64, f64, f64)> + '_ {
        self.pieces.iter().filter_map(|p| match *p {
            Piece::Src { src0, src1, out0 } => Some((src0, src1, out0)),
            _ => None,
        })
    }

    pub fn kept_ms(&self) -> f64 {
        self.kept().map(|(a, b, _)| b - a).sum()
    }
}

fn push_src(pieces: &mut Vec<Piece>, out: &mut f64, a: f64, b: f64, frame: f64) {
    let n = ((b - a) / frame).round();
    if n < 1.0 {
        return;
    }
    let len = n * frame;
    pieces.push(Piece::Src { src0: a, src1: a + len, out0: *out });
    *out += len;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_and_inserts() {
        let fps = 30.0;
        let m = TimeMap::build(10_000.0, fps, &[(2000.0, 3000.0), (2500.0, 4000.0)], &[(6000.0, 1800.0, 0)]);
        // kept: [0,2000) [4000,6000) gap 1800 [6000,10000)
        assert_eq!(m.pieces.len(), 4);
        assert!((m.duration_ms - (2000.0 + 2000.0 + 1800.0 + 4000.0)).abs() < 40.0);
        assert!((m.to_out(1000.0) - 1000.0).abs() < 1e-6);
        assert!(!m.is_kept(3000.0));
        // inside a cut maps to the cut point
        let cut_point = m.to_out(2500.0);
        assert!((cut_point - m.to_out(4000.0)).abs() < 1e-6);
        // after the insert, output is shifted by the card
        let o = m.to_out(7000.0);
        assert!((o - (2000.0 + 2000.0 + 1800.0 + 1000.0)).abs() < 40.0);
        assert!(m.to_src(o).map(|s| (s - 7000.0).abs() < 1e-6).unwrap());
        // the gap has no source
        assert!(m.to_src(m.to_out(6000.0) - 900.0).is_none());
        // every boundary is on the frame grid
        for p in &m.pieces {
            let f = p.out0() * fps / 1000.0;
            assert!((f - f.round()).abs() < 1e-6, "piece starts off-grid at {}", p.out0());
        }
    }

    #[test]
    fn round_trip_many() {
        let cuts: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 * 1000.0 + 300.0, i as f64 * 1000.0 + 450.0)).collect();
        let m = TimeMap::build(200_000.0, 30.0, &cuts, &[]);
        for i in 0..2000 {
            let s = i as f64 * 100.0 + 7.0;
            if m.is_kept(s) {
                let o = m.to_out(s);
                let back = m.to_src(o).unwrap();
                assert!((back - s).abs() < 1e-6, "{s} -> {o} -> {back}");
            }
        }
    }
}
