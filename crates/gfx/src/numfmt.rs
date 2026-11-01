//! Count-up formatting for `number` beats: "$10,075", "4.76M", "62%" keep their shape while the
//! value counts from zero.

/// A value split into prefix, number and suffix ("$" "10,075" "").
#[derive(Clone, Debug, PartialEq)]
pub struct NumFmt {
    pub prefix: String,
    pub suffix: String,
    pub value: f64,
    pub decimals: usize,
    pub commas: bool,
}

const CURRENCY: &[char] = &['$', '€', '£', '¥', '₹', '₩', '₿'];

impl NumFmt {
    /// `None` when the value should not count (no leading digit or currency sign, digits in
    /// the suffix like "1:30", or a bare year like "2024").
    pub fn parse(s: &str) -> Option<NumFmt> {
        let s = s.trim();
        let chars: Vec<char> = s.chars().collect();
        let first = *chars.first()?;
        if !(first.is_ascii_digit() || CURRENCY.contains(&first) || ((first == '+' || first == '-' || first == '~') && chars.len() > 1)) {
            return None;
        }
        let mut i = 0;
        while i < chars.len() && (CURRENCY.contains(&chars[i]) || matches!(chars[i], '+' | '-' | '~' | ' ')) {
            i += 1;
        }
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        let prefix: String = chars[..i].iter().collect();
        let start = i;
        let mut seen_dot = false;
        while i < chars.len() {
            let c = chars[i];
            let group = c == ',' && !seen_dot && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit());
            if c.is_ascii_digit() || group {
                i += 1;
            } else if c == '.' && !seen_dot && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                seen_dot = true;
                i += 1;
            } else {
                break;
            }
        }
        let num: String = chars[start..i].iter().collect();
        let suffix: String = chars[i..].iter().collect();
        if suffix.chars().any(|c| c.is_ascii_digit()) {
            return None;
        }
        let commas = num.contains(',');
        let clean: String = num.chars().filter(|c| *c != ',').collect();
        let value: f64 = clean.parse().ok()?;
        let decimals = clean.split('.').nth(1).map(|d| d.len()).unwrap_or(0);
        if prefix.is_empty() && suffix.is_empty() && decimals == 0 && !commas && clean.len() == 4 && (1900.0..=2100.0).contains(&value) {
            return None;
        }
        Some(NumFmt { prefix, suffix, value, decimals, commas })
    }

    /// The number part for value `v`, same decimals and grouping as the original.
    pub fn number(&self, v: f64) -> String {
        let s = format!("{:.*}", self.decimals, v.max(0.0));
        if !self.commas {
            return s;
        }
        let (int, frac) = match s.split_once('.') {
            Some((a, b)) => (a.to_string(), Some(b.to_string())),
            None => (s.clone(), None),
        };
        let mut out = String::new();
        for (k, c) in int.chars().enumerate() {
            if k > 0 && (int.len() - k) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        if let Some(f) = frac {
            out.push('.');
            out.push_str(&f);
        }
        out
    }

    #[cfg(test)]
    pub fn full(&self, v: f64) -> String {
        format!("{}{}{}", self.prefix, self.number(v), self.suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats() {
        let m = NumFmt::parse("$10,075").unwrap();
        assert_eq!((m.prefix.as_str(), m.suffix.as_str(), m.value, m.commas), ("$", "", 10075.0, true));
        assert_eq!(m.full(5037.6), "$5,038");
        assert_eq!(m.full(10075.0), "$10,075");
        assert_eq!(m.full(999.0), "$999");
        let c = NumFmt::parse("4.76M").unwrap();
        assert_eq!(c.full(2.381), "2.38M");
        assert_eq!(c.full(0.0), "0.00M");
        let p = NumFmt::parse("62%").unwrap();
        assert_eq!(p.full(31.2), "31%");
        assert_eq!(NumFmt::parse("10x").unwrap().suffix, "x");
        assert_eq!(NumFmt::parse("2,000/mo").unwrap().full(1000.0), "1,000/mo");
        assert!(NumFmt::parse("1:30").is_none());
        assert!(NumFmt::parse("2024").is_none());
        assert!(NumFmt::parse("Day one").is_none());
        assert_eq!(NumFmt::parse("1,234,567").unwrap().full(1234567.0), "1,234,567");
    }
}
