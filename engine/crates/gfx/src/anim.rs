//! Easing and the house motion grammar (velour): rise (~24 design px) + fade entrances with a
//! confident ease-out, overshoot pops for chips, short fades out at the end of every beat.

/// Entrance duration (ms).
pub const ENTER_MS: f64 = 350.0;
/// Exit fade at the end of a beat (ms).
pub const EXIT_MS: f64 = 250.0;

/// CSS-style cubic bezier easing through (0,0), (x1,y1), (x2,y2), (1,1).
pub fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let (cx, bx) = (3.0 * x1, 3.0 * (x2 - x1) - 3.0 * x1);
    let ax = 1.0 - cx - bx;
    let (cy, by) = (3.0 * y1, 3.0 * (y2 - y1) - 3.0 * y1);
    let ay = 1.0 - cy - by;
    let sx = |s: f32| ((ax * s + bx) * s + cx) * s;
    let dx = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    // Newton, then bisection as a fallback.
    let mut s = t;
    for _ in 0..8 {
        let x = sx(s) - t;
        if x.abs() < 1e-5 {
            return ((ay * s + by) * s + cy) * s;
        }
        let d = dx(s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= x / d;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    s = t;
    for _ in 0..30 {
        let x = sx(s);
        if (x - t).abs() < 1e-5 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) * 0.5;
    }
    ((ay * s + by) * s + cy) * s
}

/// cubic-bezier(0.16, 1, 0.3, 1): fast start, long settle.
pub fn ease_out(t: f32) -> f32 {
    bezier(0.16, 1.0, 0.3, 1.0, t)
}

/// cubic-bezier(0.65, 0, 0.35, 1).
pub fn ease_in_out(t: f32) -> f32 {
    bezier(0.65, 0.0, 0.35, 1.0, t)
}

/// Back-out with ~8% overshoot (chips, stamps).
pub fn back_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t == 0.0 {
        return 0.0;
    }
    let s = 1.70158 * 1.2;
    let u = t - 1.0;
    1.0 + (s + 1.0) * u * u * u + s * u * u
}

/// Linear 0..1 progress of `t` over `[start, start + dur]`.
pub fn prog(t: f64, start: f64, dur: f64) -> f32 {
    if dur <= 0.0 {
        return if t >= start { 1.0 } else { 0.0 };
    }
    ((t - start) / dur).clamp(0.0, 1.0) as f32
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Geometric zoom: the only zoom that reads as constant speed.
pub fn geo_zoom(t: f32, z0: f32, z1: f32) -> f32 {
    z0 * (z1 / z0).powf(t.clamp(0.0, 1.0))
}

/// A beat's local clock: `lt` ms since the beat started, `dur` its length.
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub lt: f64,
    pub dur: f64,
}

impl Clock {
    pub fn new(t_ms: f64, start: f64, end: f64) -> Clock {
        Clock { lt: t_ms - start, dur: (end - start).max(1.0) }
    }

    /// Entrance length, shortened for very short beats.
    pub fn enter_ms(&self) -> f64 {
        ENTER_MS.min(self.dur * 0.4)
    }

    /// Eased 0..1 entrance of an element appearing `delay` ms into the beat.
    pub fn enter(&self, delay: f64) -> f32 {
        ease_out(prog(self.lt, delay, self.enter_ms()))
    }

    /// 1 while the beat holds, falling to 0 over the last `EXIT_MS`.
    pub fn exit(&self) -> f32 {
        let d = EXIT_MS.min(self.dur * 0.3);
        1.0 - ease_in_out(prog(self.lt, self.dur - d, d))
    }

    /// Opacity and vertical offset (px; positive = below its rest position) of an element
    /// entering at `delay` that rises `dist` px (it lifts a little as it fades out too).
    pub fn rise(&self, delay: f64, dist: f32) -> (f32, f32) {
        let p = self.enter(delay);
        let x = self.exit();
        (p * x, (1.0 - p) * dist - (1.0 - x) * dist * 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_shapes() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.2) > 0.6, "ease out starts fast: {}", ease_out(0.2));
        assert!((ease_in_out(0.5) - 0.5).abs() < 0.02);
        let peak = (1..100).map(|i| back_out(i as f32 / 100.0)).fold(0.0f32, f32::max);
        assert!(peak > 1.04 && peak < 1.15, "overshoot {peak}");
        assert!((geo_zoom(1.0, 1.0, 1.06) - 1.06).abs() < 1e-6);
    }

    #[test]
    fn clock_envelope() {
        let c = Clock::new(1000.0, 1000.0, 4000.0);
        assert_eq!(c.rise(0.0, 24.0).0, 0.0);
        let c = Clock::new(2000.0, 1000.0, 4000.0);
        assert_eq!(c.rise(0.0, 24.0), (1.0, 0.0));
        let c = Clock::new(4000.0, 1000.0, 4000.0);
        assert!(c.exit() < 1e-3);
    }
}
