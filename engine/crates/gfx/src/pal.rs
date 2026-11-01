//! The palette, derived from the [`Theme`] (the velour dark palette by default).

use crate::Theme;

pub type Rgb = [u8; 3];

/// Linear mix of two colours (`t` = 0 gives `a`).
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// `#RRGGBB`, `RRGGBB`, `#RGB` (case-insensitive).
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.trim().trim_start_matches('#');
    let nib = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = h.as_bytes();
    match b.len() {
        6 => {
            let mut out = [0u8; 3];
            for i in 0..3 {
                out[i] = nib(b[2 * i])? * 16 + nib(b[2 * i + 1])?;
            }
            Some(out)
        }
        3 => {
            let mut out = [0u8; 3];
            for i in 0..3 {
                let v = nib(b[i])?;
                out[i] = v * 16 + v;
            }
            Some(out)
        }
        _ => None,
    }
}

/// Every colour the templates use. The six theme colours are taken as is; the rest are derived
/// so an overridden accent or canvas keeps the family consistent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pal {
    pub canvas: Rgb,
    pub dot: Rgb,
    /// The raised panel surface stage beats sit on (#181716).
    pub surface: Rgb,
    pub card: Rgb,
    /// Second, lifted card tone (#2A2825): title bars, chips.
    pub card2: Rgb,
    pub line: Rgb,
    pub ink: Rgb,
    pub ink_soft: Rgb,
    pub muted: Rgb,
    pub faint: Rgb,
    pub accent: Rgb,
    pub accent_soft: Rgb,
    pub green: Rgb,
    pub red: Rgb,
    pub gold: Rgb,
    pub white: Rgb,
    pub black: Rgb,
}

impl Pal {
    pub fn from_theme(t: &Theme) -> Pal {
        Pal {
            canvas: t.canvas,
            dot: t.dot,
            surface: mix(t.canvas, t.card, 0.5),
            card: t.card,
            card2: mix(t.card, t.ink, 0.05),
            line: mix(t.card, t.ink, 0.1),
            ink: t.ink,
            ink_soft: mix(t.ink, t.canvas, 0.12),
            muted: t.muted,
            faint: mix(t.muted, t.canvas, 0.35),
            accent: t.accent,
            accent_soft: mix(t.canvas, t.accent, 0.19),
            green: [0x46, 0xB4, 0x82],
            red: [0xE5, 0x56, 0x4A],
            gold: [0xE0, 0xB0, 0x4F],
            white: [0xFF, 0xFF, 0xFF],
            black: [0, 0, 0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex() {
        assert_eq!(parse_hex("#E2774E"), Some([0xE2, 0x77, 0x4E]));
        assert_eq!(parse_hex("e2774e"), Some([0xE2, 0x77, 0x4E]));
        assert_eq!(parse_hex("#fff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("zzzzzz"), None);
    }

    #[test]
    fn derived_defaults_match_velour() {
        let p = Pal::from_theme(&Theme::from_style(&Default::default()));
        let close = |a: Rgb, b: Rgb| a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= 6);
        assert!(close(p.surface, [0x18, 0x17, 0x16]), "{:?}", p.surface);
        assert!(close(p.line, [0x34, 0x32, 0x2E]), "{:?}", p.line);
        assert!(close(p.accent_soft, [0x3A, 0x24, 0x1A]), "{:?}", p.accent_soft);
        assert!(close(p.faint, [0x6E, 0x68, 0x5F]), "{:?}", p.faint);
        assert!(close(p.card2, [0x2A, 0x28, 0x25]), "{:?}", p.card2);
    }
}
