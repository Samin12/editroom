//! Secret detectors over OCR'd words.
//!
//! Words are regrouped into lines from their boxes (tesseract's own grouping is unreliable on
//! screen UIs). Each line is searched as spaced text, as space-free joins of adjacent words (OCR
//! splits long tokens at `-`, `_` and kerning gaps), with OCR digit confusions undone (cards and
//! phones), and across wrapped lines (a key wrapping at the terminal edge). Values are never kept:
//! a sighting carries a hash of the value plus a redacted preview.

use crate::ocr::Word;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// An extra detector: `regex` (Rust syntax) reports `kind`. The secret is the capture group named
/// `v`, else group 1, else the whole match. It runs on spaced lines and on space-free joins.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pattern {
    pub kind: String,
    pub regex: String,
}

/// One sighting in one frame.
#[derive(Clone, PartialEq)]
pub(crate) struct Obs {
    pub kind: String,
    /// Hash of the normalized value (the value itself is never kept).
    pub key: u64,
    pub preview: String,
    /// Pixel box `[x0, y0, x1, y1]` in the OCR frame.
    pub rect: [f32; 4],
}

impl std::fmt::Debug for Obs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {:?}", self.kind, self.preview, self.rect)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Check {
    None,
    /// A provider token: `body` = chars after the first `prefix` chars must be this long and look random.
    Token { prefix: usize, body: usize },
    Email,
    Phone,
    Card,
    /// Generic `prefix_xxx` token: long random body after the last `_`/`-`.
    Prefixed,
    /// Value after a keyword (key/token/secret/password).
    Keyword,
    Query,
}

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    /// Words joined by single spaces.
    Spaced,
    /// Adjacent words joined with nothing between them.
    Joined,
    /// Spaced, with OCR letter-for-digit confusions undone in mostly-digit words.
    Digits,
    /// A label line plus the line under it ("API key" above its value).
    Label,
    /// Spaced, but words with no real gap between them (or meeting at `.@-_`) are joined:
    /// emails split by OCR, without gluing the words before them on.
    Tight,
}

struct Rule {
    kind: String,
    re: Regex,
    check: Check,
    units: &'static [Unit],
}

const SPACED_JOINED: &[Unit] = &[Unit::Spaced, Unit::Joined];
const SPACED_TIGHT: &[Unit] = &[Unit::Spaced, Unit::Tight];
const DIGITS: &[Unit] = &[Unit::Digits];
const KEYWORD_UNITS: &[Unit] = &[Unit::Spaced, Unit::Label];

/// Screens that are private as a whole (an inbox, a bank page): enough of these labels on one
/// frame flags the frame's content.
struct Fingerprint {
    kind: &'static str,
    labels: &'static [&'static str],
    min_labels: usize,
    /// Also needs this many money amounts; the flagged rows are the ones holding amounts.
    min_amounts: usize,
}

const FINGERPRINTS: &[Fingerprint] = &[
    Fingerprint {
        kind: "inbox",
        labels: &["Inbox", "Starred", "Snoozed", "Sent", "Drafts", "Compose", "Primary", "Promotions", "Social", "Updates", "Spam", "Important", "Scheduled", "Outbox", "Junk"],
        min_labels: 4,
        min_amounts: 0,
    },
    Fingerprint {
        kind: "finance",
        labels: &["Balance", "Transactions", "Available", "Pending", "Payouts", "Deposit", "Deposits", "Withdrawal", "Withdrawals", "Statement", "Statements", "Transfers", "Routing", "Spending", "Income", "Posted"],
        min_labels: 2,
        min_amounts: 4,
    },
];

pub(crate) struct Detectors {
    rules: Vec<Rule>,
    allow: Allow,
    skip: HashSet<String>,
    amount: Regex,
}

/// Values never reported.
struct Allow {
    exact: Vec<String>,
    domains: Vec<String>,
    res: Vec<Regex>,
}

/// File extensions that look like TLDs in `icon@2x.png`.
const NOT_TLD: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp", "ico", "js", "ts", "tsx", "jsx", "css", "json", "txt", "md", "py", "rs", "go", "mov", "mp4", "pdf", "html", "yaml", "yml", "toml", "lock", "zip", "app", "dmg", "heic"];

/// Digests and hashes are random-looking but public.
const NOT_PREFIX: &[&str] = &["sha1", "sha256", "sha384", "sha512", "md5", "base64", "utf8", "data", "image", "font", "x64", "arm64", "uuid"];

impl Detectors {
    pub(crate) fn new(extra: &[Pattern], allow: &[String], skip: &[String]) -> Result<Detectors> {
        let mut rules = vec![];
        let mut add = |kind: &str, re: &str, check: Check, units: &'static [Unit]| -> Result<()> {
            rules.push(Rule { kind: kind.into(), re: Regex::new(re).map_err(|e| anyhow!("detector {kind}: {e}"))?, check, units });
            Ok(())
        };
        // OpenAI / Anthropic `sk-`, `sk-proj-`, `sk-ant-api03-`; Stripe `sk_live_`, `rk_live_`; OCR reads s as 5.
        add("api_key", r"[sSrR5][kK][-_][A-Za-z0-9_\-]{18,}", Check::Token { prefix: 3, body: 18 }, SPACED_JOINED)?;
        // GitHub; the underscore is often lost by OCR.
        add("api_key", r"(?:gh[pousr]_?[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})", Check::Token { prefix: 3, body: 30 }, SPACED_JOINED)?;
        // Google (AIza; I/l/1 confusion).
        add("api_key", r"A[Il1|][zZ]a[0-9A-Za-z_\-]{30,}", Check::Token { prefix: 4, body: 30 }, SPACED_JOINED)?;
        // Slack.
        add("token", r"xox[abposr]-[A-Za-z0-9\-]{10,}", Check::Token { prefix: 5, body: 10 }, SPACED_JOINED)?;
        // AWS access key id.
        add("api_key", r"(?:AKIA|ASIA|AK1A|A5IA)[0-9A-Z]{16}", Check::Token { prefix: 4, body: 16 }, SPACED_JOINED)?;
        // Stripe webhook secret, Multica `mul_`.
        add("api_key", r"(?:whsec|mul)_[A-Za-z0-9]{16,}", Check::Token { prefix: 4, body: 16 }, SPACED_JOINED)?;
        // JWT (base64 of `{"`), dots sometimes read as commas.
        add("token", r"ey[JjI1l][A-Za-z0-9_\-]{15,}(?:[.,][A-Za-z0-9_\-]{4,}){0,2}", Check::Token { prefix: 3, body: 20 }, SPACED_JOINED)?;
        // Bearer <token>.
        add("token", r"[Bb]earer[\s:]*([A-Za-z0-9_\-.=~+/]{16,})", Check::Token { prefix: 0, body: 16 }, SPACED_JOINED)?;
        // Generic `prefix_` / `prefix-` tokens (pplx-, gsk_, hf_, glpat-, npm_, fc-, ...).
        add("api_key", r"[a-z][a-z0-9]{1,9}(?:[_\-][a-z0-9]{2,8}){0,2}[_\-][A-Za-z0-9]{20,}", Check::Prefixed, SPACED_JOINED)?;
        // Secrets in URLs (OAuth callbacks, signed links).
        add("token", r"(?i)[?&#](?:code|access_token|id_token|refresh_token|token|session|sid|api_key|apikey|key|client_secret|oauth_token|signature|sig)=([A-Za-z0-9_\-.%~+/]{12,})", Check::Query, SPACED_JOINED)?;
        // Keyword then value: `OPENAI_API_KEY=...`, `Secret: ...`, `password ...`.
        add(
            "api_key",
            r#"(?i)(?:^|[^a-z])((?:api|secret|access|private|client|auth|refresh|session)?[\s_\-]?(?:key|token|secret|password|passwd|pwd))["'\]]?\s*[:=]?\s*["']?([^\s"'<>,;]{8,})"#,
            Check::Keyword,
            KEYWORD_UNITS,
        )?;
        add("email", r"[A-Za-z0-9][A-Za-z0-9._%+\-]*@[A-Za-z0-9][A-Za-z0-9\-]*(?:\.[A-Za-z0-9\-]+)*[.,][A-Za-z]{2,24}", Check::Email, SPACED_TIGHT)?;
        add("phone", r"(?:^|[^0-9A-Za-z+])((?:\+\d{1,3}[\s.\-]?)?(?:\(\d{3}\)\s?|\d{3}[\s.\-])\d{3}[\s.\-]\d{4})(?:$|[^0-9A-Za-z])", Check::Phone, DIGITS)?;
        add("phone", r"(?:^|[^0-9A-Za-z+])(\+\d{1,3}(?:[\s.\-]?\d{2,5}){2,4})(?:$|[^0-9A-Za-z])", Check::Phone, DIGITS)?;
        add("card", r"(?:^|[^0-9])((?:\d[ \-]?){12,18}\d)(?:$|[^0-9])", Check::Card, DIGITS)?;
        for p in extra {
            add(&p.kind, &p.regex, Check::None, SPACED_JOINED)?;
        }
        let mut exact = vec![];
        let mut domains = vec![];
        let mut res = vec![];
        for a in allow {
            let a = a.trim();
            if a.len() > 2 && a.starts_with('/') && a.ends_with('/') {
                res.push(Regex::new(&a[1..a.len() - 1]).map_err(|e| anyhow!("allow {a:?}: {e}"))?);
            } else if let Some(d) = a.strip_prefix('@') {
                domains.push(d.to_lowercase());
            } else if !a.is_empty() {
                exact.push(a.to_lowercase());
            }
        }
        Ok(Detectors {
            rules,
            allow: Allow { exact, domains, res },
            skip: skip.iter().map(|s| s.to_lowercase()).collect(),
            amount: Regex::new(r"^[-+−(]?[$€£¥][\d,]*\d(?:\.\d\d)?\)?$|^[-+−]?\d[\d,]*\.\d\d\s?(?:USD|EUR|GBP|CAD)?$").unwrap(),
        })
    }

    /// Every secret visible in one frame's words.
    pub(crate) fn detect(&self, words: &[Word]) -> Vec<Obs> {
        let segs = segments(words);
        let mut out: Vec<Obs> = vec![];
        let mut units: Vec<(Unit, TextUnit)> = vec![];
        for s in &segs {
            units.push((Unit::Spaced, TextUnit::spaced(words, &s.words, false)));
            units.push((Unit::Digits, TextUnit::spaced(words, &s.words, true)));
            units.push((Unit::Tight, TextUnit::tight(words, &s.words, s.h)));
            for run in runs(words, &s.words, s.h) {
                if run.len() > 1 {
                    units.push((Unit::Joined, TextUnit::joined(words, &run)));
                }
            }
        }
        // wrapped tokens and label-above-value pairs
        for (i, a) in segs.iter().enumerate() {
            for b in segs[i + 1..].iter().take(40) {
                let (ab, bb) = (seg_box(words, a), seg_box(words, b));
                let h = a.h.max(b.h);
                if bb[1] - ab[3] > 2.5 * h {
                    continue;
                }
                if bb[1] < ab[3] - 0.3 * h {
                    continue;
                }
                let first_a = &words[a.words[0]];
                let first_b = &words[b.words[0]];
                // left-aligned line directly below: a wrapped token continues there
                if bb[1] - ab[3] < 0.9 * h && first_b.b[0] <= first_a.b[0] + 2.0 * h && first_b.b[0] >= first_a.b[0] - 40.0 * h {
                    let tail = runs(words, &a.words, a.h).pop().unwrap_or_default();
                    let head = runs(words, &b.words, b.h).into_iter().next().unwrap_or_default();
                    let last = &words[*tail.last().unwrap_or(&a.words[a.words.len() - 1])];
                    if last.text.chars().count() >= 6 && !head.is_empty() {
                        let both: Vec<usize> = tail.iter().chain(head.iter()).copied().collect();
                        units.push((Unit::Joined, TextUnit::joined(words, &both)));
                    }
                }
                // a short label line, its value under it
                if a.words.len() <= 4 && (first_b.b[0] - first_a.b[0]).abs() < 3.0 * h {
                    let mut ids = a.words.clone();
                    ids.extend(&b.words);
                    units.push((Unit::Label, TextUnit::spaced(words, &ids, false)));
                }
            }
        }
        for rule in &self.rules {
            if self.skip.contains(&rule.kind) {
                continue;
            }
            for (u, tu) in &units {
                if !rule.units.contains(u) {
                    continue;
                }
                for m in tu.find(&rule.re, rule.check) {
                    let Some((kind, value)) = self.accept(rule, &m.value, &m.label) else { continue };
                    let ids = m.ids;
                    let key = hash(&kind, &norm_key(&kind, &value));
                    let preview = redact(&kind, &value);
                    for rect in rects_by_row(words, &ids) {
                        push_obs(&mut out, Obs { kind: kind.clone(), key, preview: preview.clone(), rect });
                    }
                }
            }
        }
        for fp in FINGERPRINTS {
            if !self.skip.contains(fp.kind) {
                if let Some(o) = self.fingerprint(fp, words, &segs) {
                    push_obs(&mut out, o);
                }
            }
        }
        consolidate(out)
    }

    /// Run the rule's check; returns the reported kind and the normalized value.
    fn accept(&self, rule: &Rule, raw: &str, label: &str) -> Option<(String, String)> {
        let v = raw.trim_matches(|c: char| matches!(c, '.' | ',' | ':' | ';' | ')' | '(' | '"' | '\'' | '`' | ']' | '['));
        let kind = rule.kind.clone();
        let ok = match rule.check {
            Check::None => !v.is_empty(),
            Check::Token { prefix, body } => {
                let b: String = v.chars().skip(prefix).collect();
                b.chars().count() >= body && looks_random(&b, 3.0)
            }
            Check::Prefixed => {
                let cut = v.rfind(['_', '-']).unwrap_or(0);
                let (pre, b) = v.split_at(cut);
                let first = pre.split(['_', '-']).next().unwrap_or("");
                !NOT_PREFIX.contains(&first) && b.len() > 20 && looks_random(&b[1..], 3.5) && b[1..].chars().any(|c| c.is_ascii_uppercase()) && b[1..].chars().any(|c| c.is_ascii_lowercase())
            }
            Check::Query => looks_random(v, 3.0),
            Check::Keyword => return self.keyword(label, v),
            Check::Email => {
                let v = v.replace(',', ".");
                let tld = v.rsplit('.').next().unwrap_or("").to_lowercase();
                let ok = !NOT_TLD.contains(&tld.as_str()) && v.split('@').next().is_some_and(|l| !l.is_empty());
                return (ok && !self.allowed("email", &v)).then(|| ("email".to_string(), v.to_lowercase()));
            }
            Check::Phone => phone_ok(v),
            Check::Card => card_ok(v),
        };
        (ok && !self.allowed(&kind, v)).then(|| (kind, v.to_string()))
    }

    /// The value after a keyword; the keyword decides the kind.
    fn keyword(&self, label: &str, v: &str) -> Option<(String, String)> {
        let lower = v.to_lowercase();
        if lower.starts_with("http") || v.starts_with('/') || v.starts_with('~') || v.starts_with("./") || lower.contains("://") {
            return None;
        }
        let bullets = v.chars().filter(|c| matches!(c, '•' | '*' | '·' | '●' | '∙')).count();
        if bullets * 2 >= v.chars().count() {
            return None;
        }
        let label = label.to_lowercase();
        let n = v.chars().count();
        let (kind, ok) = if label.contains("pass") || label.contains("pwd") {
            // passwords are short: a letter plus a digit or symbol, not a plain word
            let sym = v.chars().any(|c| !c.is_alphanumeric());
            let digit = v.chars().any(|c| c.is_ascii_digit());
            ("password", n >= 8 && v.chars().any(|c| c.is_alphabetic()) && (digit || sym) && entropy(v) >= 2.5)
        } else if label.contains("token") {
            ("token", n >= 16 && looks_random(v, 3.2))
        } else {
            ("api_key", n >= 16 && looks_random(v, 3.2))
        };
        (ok && !self.allowed(kind, v)).then(|| (kind.to_string(), v.to_string()))
    }

    fn allowed(&self, kind: &str, v: &str) -> bool {
        let l = v.to_lowercase();
        if self.allow.exact.iter().any(|a| *a == l || (kind == "email" && a.len() >= 10 && a.contains('@') && lev1(a, &l))) {
            return true;
        }
        if kind == "email" {
            let d = l.rsplit('@').next().unwrap_or("");
            if self.allow.domains.iter().any(|a| d == a || d.ends_with(&format!(".{a}"))) {
                return true;
            }
        }
        self.allow.res.iter().any(|r| r.is_match(v))
    }

    fn fingerprint(&self, fp: &Fingerprint, words: &[Word], segs: &[Seg]) -> Option<Obs> {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut top = f32::MAX;
        for w in words {
            let t = w.text.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(l) = fp.labels.iter().find(|l| **l == t) {
                seen.insert(l);
                top = top.min(w.b[1] - 2.0 * w.h());
            }
        }
        if seen.len() < fp.min_labels {
            return None;
        }
        let mut rect: Option<[f32; 4]> = None;
        if fp.min_amounts == 0 {
            for w in words.iter().filter(|w| w.b[1] >= top) {
                rect = Some(union(rect, w.b));
            }
        } else {
            let mut rows: Vec<usize> = vec![];
            for s in segs {
                if s.words.iter().any(|&i| self.amount.is_match(words[i].text.trim())) {
                    rows.push(s.row);
                }
            }
            if rows.len() < fp.min_amounts {
                return None;
            }
            for s in segs.iter().filter(|s| rows.contains(&s.row)) {
                for &i in &s.words {
                    rect = Some(union(rect, words[i].b));
                }
            }
        }
        let rect = rect?;
        Some(Obs { kind: fp.kind.into(), key: hash(fp.kind, ""), preview: format!("{} screen", fp.kind), rect })
    }
}

/// Keep one sighting per thing: drop a sighting of the same kind whose box mostly overlaps one we have.
fn push_obs(out: &mut Vec<Obs>, o: Obs) {
    if let Some(x) = out.iter_mut().find(|x| same_thing(x, &o)) {
        // widen the kept one so it covers both
        x.rect = union(Some(x.rect), o.rect);
        return;
    }
    out.push(o);
}

fn same_thing(x: &Obs, o: &Obs) -> bool {
    x.kind == o.kind && ((x.key == o.key && iou(&x.rect, &o.rect) > 0.3) || contains_most(&x.rect, &o.rect) || contains_most(&o.rect, &x.rect))
}

/// Widening can make a kept sighting cover an earlier one: merge until stable.
fn consolidate(mut v: Vec<Obs>) -> Vec<Obs> {
    let mut i = 0;
    while i < v.len() {
        let mut j = i + 1;
        let mut merged = false;
        while j < v.len() {
            if same_thing(&v[i], &v[j]) {
                let o = v.remove(j);
                v[i].rect = union(Some(v[i].rect), o.rect);
                merged = true;
            } else {
                j += 1;
            }
        }
        if !merged {
            i += 1;
        }
    }
    v
}

fn contains_most(a: &[f32; 4], b: &[f32; 4]) -> bool {
    let ix = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let iy = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    ix * iy >= 0.8 * (b[2] - b[0]) * (b[3] - b[1])
}

pub(crate) fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let ix = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let iy = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let i = ix * iy;
    let u = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - i;
    if u <= 0.0 {
        0.0
    } else {
        i / u
    }
}

pub(crate) fn union(a: Option<[f32; 4]>, b: [f32; 4]) -> [f32; 4] {
    match a {
        None => b,
        Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
    }
}

fn hash(kind: &str, v: &str) -> u64 {
    let mut h = DefaultHasher::new();
    kind.hash(&mut h);
    v.hash(&mut h);
    h.finish()
}

fn norm_key(kind: &str, v: &str) -> String {
    match kind {
        "card" | "phone" => v.chars().filter(|c| c.is_ascii_digit()).collect(),
        "email" => v.to_lowercase(),
        _ => v.to_string(),
    }
}

/// `sk-ant...`; emails `sa***@g***`. Never more than half of the value.
pub fn redact(kind: &str, v: &str) -> String {
    if kind == "email" {
        if let Some((l, d)) = v.split_once('@') {
            let l: String = l.chars().take(2.min(l.chars().count().saturating_sub(1)).max(1)).collect();
            let d: String = d.chars().take(1).collect();
            return format!("{l}***@{d}***");
        }
    }
    let n = 6.min(v.chars().count() / 2);
    format!("{}...", v.chars().take(n).collect::<String>())
}

/// Shannon entropy in bits per char.
pub(crate) fn entropy(s: &str) -> f64 {
    let mut counts = std::collections::HashMap::new();
    let mut n = 0.0;
    for c in s.chars() {
        *counts.entry(c).or_insert(0.0) += 1.0;
        n += 1.0;
    }
    if n == 0.0 {
        return 0.0;
    }
    counts.values().map(|&k: &f64| -(k / n) * (k / n).log2()).sum()
}

/// Random-looking: letters and digits mixed, high entropy.
fn looks_random(s: &str, min_bits: f64) -> bool {
    let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
    let letters = s.chars().filter(|c| c.is_ascii_alphabetic()).count();
    letters >= 4 && digits >= 2 && entropy(s) >= min_bits
}

fn phone_ok(v: &str) -> bool {
    let d: Vec<u32> = v.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.iter().all(|&x| x == d[0]) {
        return false;
    }
    if v.trim_start().starts_with('+') {
        if (8..=15).contains(&d.len()) {
            // +1 numbers follow the North American plan
            if d[0] == 1 && d.len() == 11 {
                return d[1] >= 2 && d[4] >= 2;
            }
            return true;
        }
        return false;
    }
    let d = if d.len() == 11 && d[0] == 1 { &d[1..] } else { &d[..] };
    d.len() == 10 && d[0] >= 2 && d[3] >= 2
}

fn card_ok(v: &str) -> bool {
    let d: Vec<u32> = v.chars().filter_map(|c| c.to_digit(10)).collect();
    if !(13..=19).contains(&d.len()) || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    let p2 = d[0] * 10 + d[1];
    let p4 = p2 * 100 + d[2] * 10 + d[3];
    let network = d[0] == 4 || (51..=55).contains(&p2) || (2221..=2720).contains(&p4) || p2 == 34 || p2 == 37 || p4 == 6011 || p2 == 65 || p2 == 36 || p2 == 38 || p2 == 30 || p2 == 35 || p2 == 62;
    if !network {
        return false;
    }
    if luhn(&d) {
        return true;
    }
    // 4-4-4-4 grouping with a plausible network is a card even when OCR broke a digit
    let groups: Vec<usize> = v.split([' ', '-']).filter(|g| !g.is_empty()).map(|g| g.len()).collect();
    groups == [4, 4, 4, 4]
}

pub(crate) fn luhn(d: &[u32]) -> bool {
    let mut sum = 0;
    for (i, &x) in d.iter().rev().enumerate() {
        let y = if i % 2 == 1 { x * 2 } else { x };
        sum += if y > 9 { y - 9 } else { y };
    }
    sum % 10 == 0
}

/// Levenshtein distance <= 1.
fn lev1(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            i += 1;
            j += 1;
            continue;
        }
        edits += 1;
        if edits > 1 {
            return false;
        }
        match a.len().cmp(&b.len()) {
            std::cmp::Ordering::Greater => i += 1,
            std::cmp::Ordering::Less => j += 1,
            std::cmp::Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }
    edits + (a.len() - i) + (b.len() - j) <= 1
}

/// OCR look-alikes, one char for one char.
fn norm_char(c: char) -> char {
    match c {
        '—' | '–' | '‐' | '‑' | '−' | '⁃' => '-',
        '©' | '®' | '＠' => '@',
        '‘' | '’' | '`' | '´' => '\'',
        '“' | '”' => '"',
        '|' => 'l',
        _ => c,
    }
}

/// Letters OCR reads in place of digits.
fn digit_char(c: char) -> char {
    match c {
        'O' | 'o' | 'Q' | 'D' => '0',
        'l' | 'I' | '|' | 'i' | '!' => '1',
        'S' | 's' => '5',
        'B' => '8',
        'Z' | 'z' => '2',
        _ => c,
    }
}

fn digit_word(t: &str) -> String {
    let alnum = t.chars().filter(|c| c.is_alphanumeric()).count();
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    if alnum >= 3 && digits * 10 >= alnum * 6 {
        t.chars().map(digit_char).collect()
    } else {
        t.to_string()
    }
}

struct Match {
    value: String,
    /// The keyword before the value (keyword rules).
    label: String,
    ids: Vec<usize>,
}

/// Text built from words, with each word's byte span.
struct TextUnit {
    text: String,
    spans: Vec<(usize, usize, usize)>,
}

impl TextUnit {
    fn spaced(words: &[Word], ids: &[usize], digits: bool) -> TextUnit {
        let mut text = String::new();
        let mut spans = vec![];
        for &i in ids {
            if !text.is_empty() {
                text.push(' ');
            }
            let s = text.len();
            let t: String = words[i].text.chars().map(norm_char).collect();
            text.push_str(&if digits { digit_word(&t) } else { t });
            spans.push((s, text.len(), i));
        }
        TextUnit { text, spans }
    }

    fn tight(words: &[Word], ids: &[usize], h: f32) -> TextUnit {
        let mut text = String::new();
        let mut spans = vec![];
        let mut prev: Option<&Word> = None;
        for &i in ids {
            let w = &words[i];
            if let Some(p) = prev {
                let glued = w.b[0] - p.b[2] < 0.18 * h || p.text.ends_with(['.', '-', '_', '@', '+']) || w.text.starts_with(['.', '-', '_', '@']);
                if !glued {
                    text.push(' ');
                }
            }
            let s = text.len();
            text.extend(w.text.chars().map(norm_char));
            spans.push((s, text.len(), i));
            prev = Some(w);
        }
        TextUnit { text, spans }
    }

    fn joined(words: &[Word], ids: &[usize]) -> TextUnit {
        let mut text = String::new();
        let mut spans = vec![];
        for &i in ids {
            let s = text.len();
            text.extend(words[i].text.chars().map(norm_char));
            spans.push((s, text.len(), i));
        }
        TextUnit { text, spans }
    }

    /// Matches that start at a word start or after a non-alphanumeric char.
    fn find(&self, re: &Regex, check: Check) -> Vec<Match> {
        let mut out = vec![];
        let mut at = 0;
        let bytes = self.text.as_bytes();
        while at < self.text.len() {
            let Some(c) = re.captures_at(&self.text, at) else { break };
            let whole = c.get(0).unwrap();
            // keyword rules capture (keyword, value); others the value in `v` or group 1
            let m = if check == Check::Keyword { c.get(2) } else { c.name("v").or_else(|| c.get(1)) }.unwrap_or(whole);
            let label = if check == Check::Keyword { c.get(1).map(|g| g.as_str().to_string()).unwrap_or_default() } else { String::new() };
            let start = m.start();
            let boundary = start == 0 || !(bytes[start - 1] as char).is_ascii_alphanumeric() || self.spans.iter().any(|s| s.0 == start);
            if boundary && m.end() > m.start() {
                let ids: Vec<usize> = self.spans.iter().filter(|s| s.0 < m.end() && s.1 > m.start()).map(|s| s.2).collect();
                out.push(Match { value: m.as_str().to_string(), label, ids });
                at = whole.end().max(at + 1);
            } else {
                // try again from the next char after this match's start
                at = next_char(&self.text, whole.start());
            }
        }
        out
    }
}

fn next_char(s: &str, i: usize) -> usize {
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

/// Split matched words by row; one box per row.
fn rects_by_row(words: &[Word], ids: &[usize]) -> Vec<[f32; 4]> {
    let mut rows: Vec<[f32; 4]> = vec![];
    for &i in ids {
        let b = words[i].b;
        let yc = (b[1] + b[3]) / 2.0;
        if let Some(r) = rows.iter_mut().find(|r| yc > r[1] && yc < r[3]) {
            *r = union(Some(*r), b);
        } else {
            rows.push(b);
        }
    }
    rows
}

/// A run of words on one line: one row, no column gap.
pub(crate) struct Seg {
    pub row: usize,
    pub words: Vec<usize>,
    pub h: f32,
}

fn seg_box(words: &[Word], s: &Seg) -> [f32; 4] {
    let mut r = None;
    for &i in &s.words {
        r = Some(union(r, words[i].b));
    }
    r.unwrap_or([0.0; 4])
}

/// Rebuild text lines from word boxes: rows by vertical overlap, split at column gaps.
pub(crate) fn segments(words: &[Word]) -> Vec<Seg> {
    struct Row {
        yc: f32,
        h: f32,
        members: Vec<usize>,
    }
    let mut idx: Vec<usize> = (0..words.len()).filter(|&i| !words[i].text.trim().is_empty()).collect();
    idx.sort_by(|&a, &b| words[a].yc().total_cmp(&words[b].yc()));
    let mut rows: Vec<Row> = vec![];
    for &i in &idx {
        let w = &words[i];
        let (yc, h) = (w.yc(), w.h().max(1.0));
        let mut best: Option<(usize, f32)> = None;
        let lo = rows.len().saturating_sub(60);
        for (ri, r) in rows.iter().enumerate().skip(lo) {
            let dy = (r.yc - yc).abs();
            if dy >= 0.5 * h.min(r.h) || h / r.h > 2.2 || r.h / h > 2.2 {
                continue;
            }
            // overlapping boxes on one row are kept together: tesseract's sparse mode returns a
            // word plus pieces of it ("jdoe.private" spanning the whole email, then "@", ...)
            if best.is_none_or(|(_, d)| dy < d) {
                best = Some((ri, dy));
            }
        }
        match best {
            Some((ri, _)) => {
                let r = &mut rows[ri];
                let n = r.members.len() as f32;
                r.yc = (r.yc * n + yc) / (n + 1.0);
                r.h = (r.h * n + h) / (n + 1.0);
                r.members.push(i);
            }
            None => rows.push(Row { yc, h, members: vec![i] }),
        }
    }
    rows.sort_by(|a, b| a.yc.total_cmp(&b.yc));
    let mut out = vec![];
    for (ri, r) in rows.into_iter().enumerate() {
        let mut m = r.members;
        m.sort_by(|&a, &b| words[a].b[0].total_cmp(&words[b].b[0]));
        let mut cur: Vec<usize> = vec![];
        for i in m {
            if let Some(&p) = cur.last() {
                if words[i].b[0] - words[p].b[2] > 3.0 * r.h {
                    out.push(Seg { row: ri, words: std::mem::take(&mut cur), h: r.h });
                }
            }
            cur.push(i);
        }
        if !cur.is_empty() {
            out.push(Seg { row: ri, words: cur, h: r.h });
        }
    }
    out
}

/// Split a line where the gap is wider than a space: candidates for joining.
fn runs(words: &[Word], ids: &[usize], h: f32) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![];
    for &i in ids {
        match out.last_mut() {
            Some(r) if words[i].b[0] - words[*r.last().unwrap()].b[2] <= 1.0 * h => r.push(i),
            _ => out.push(vec![i]),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(texts: &[&str], y: f32) -> Vec<Word> {
        let mut x = 10.0;
        texts
            .iter()
            .map(|t| {
                let w = t.chars().count() as f32 * 12.0;
                let word = Word { text: t.to_string(), conf: 90.0, b: [x, y, x + w, y + 22.0] };
                x += w + 8.0;
                word
            })
            .collect()
    }

    fn det() -> Detectors {
        Detectors::new(&[], &["hello@cutroom.dev".into(), "@example.com".into()], &[]).unwrap()
    }

    fn kinds(words: &[Word]) -> Vec<String> {
        let mut k: Vec<String> = det().detect(words).into_iter().map(|o| o.kind).collect();
        k.sort();
        k
    }

    // fake values are assembled at runtime so no literal looks like a real credential
    fn fake_sk() -> String {
        ["sk", "-proj-", "Q7f2Lx9mZ4rT1vB8nK3wY6hJ0cD5sAeP"].concat()
    }

    #[test]
    fn finds_keys_split_by_ocr() {
        let k = fake_sk();
        assert_eq!(kinds(&line(&["OPENAI_API_KEY=", &k], 0.0)), ["api_key"]);
        // split in three by OCR
        let (a, b) = k.split_at(12);
        let (b, c) = b.split_at(9);
        assert_eq!(kinds(&line(&["key:", a, b, c], 0.0)), ["api_key"]);
        // em dash for the hyphen
        assert_eq!(kinds(&line(&[&k.replacen('-', "—", 1)], 0.0)), ["api_key"]);
    }

    #[test]
    fn finds_wrapped_token() {
        let k = fake_sk();
        let (a, b) = k.split_at(14);
        let mut w = line(&["$", "export", a], 0.0);
        w.extend(line(&[b], 26.0));
        let o = det().detect(&w);
        assert!(o.iter().any(|o| o.kind == "api_key"), "{o:?}");
        assert_eq!(o.iter().filter(|o| o.kind == "api_key").count(), 2, "one box per line: {o:?}");
    }

    #[test]
    fn emails_cards_phones() {
        assert_eq!(kinds(&line(&["From:", "Jane", "<jane.doe.test@examplemail.net>"], 0.0)), ["email"]);
        assert!(kinds(&line(&["hello@cutroom.dev"], 0.0)).is_empty(), "allowlisted");
        assert!(kinds(&line(&["Hello@Cutroom.dev"], 0.0)).is_empty(), "allowlist is case-insensitive");
        assert!(kinds(&line(&["hello@cutroom.dew"], 0.0)).is_empty(), "allowlist tolerates one OCR error");
        assert!(kinds(&line(&["bob@mail.example.com"], 0.0)).is_empty(), "allowlisted domain");
        assert!(kinds(&line(&["icon@2x.png"], 0.0)).is_empty());
        assert_eq!(kinds(&line(&["4242", "4242", "4242", "4242"], 0.0)), ["card"]);
        assert_eq!(kinds(&line(&["4242", "4242", "4242", "4Z42"], 0.0)), ["card"], "digit confusion");
        assert!(kinds(&line(&["1234", "5678", "9012", "3456"], 0.0)).is_empty(), "no network prefix");
        assert_eq!(kinds(&line(&["Phone:", "+1", "(415)", "555-0134"], 0.0)), ["phone"]);
        assert_eq!(kinds(&line(&["call", "415-555-0134"], 0.0)), ["phone"]);
        assert!(kinds(&line(&["Order", "#48213", "shipped", "2024-10-31"], 0.0)).is_empty());
    }

    #[test]
    fn split_emails_keep_their_own_words() {
        let o = det().detect(&line(&["Reply", "to", "jane.doe", "@examplemail.net"], 0.0));
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].preview, "ja***@e***");
        // tesseract's sparse mode: a word box spanning the whole email, plus its pieces inside it
        let w = |t: &str, x0: f32, x1: f32| Word { text: t.into(), conf: 90.0, b: [x0, 1704.0, x1, 1725.0] };
        let words = vec![w("Signed", 121.0, 187.0), w("in", 196.0, 210.0), w("as", 219.0, 240.0), w("jdoe.private", 247.0, 500.0), w("@", 370.0, 378.0), w("mailbox.test", 387.0, 500.0)];
        let o = det().detect(&words);
        assert_eq!(o.len(), 1, "{o:?}");
        assert_eq!(o[0].preview, "jd***@m***");
        assert_eq!(o[0].rect[0], 247.0, "the words before it are not part of it");
    }

    #[test]
    fn tokens() {
        let jwt = ["ey", "JhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", ".", "eyJzdWIiOiIxMjM0NTY3ODkwIn0", ".", "dozjgNryP4J3jVmNHl0w5N"].concat();
        assert_eq!(kinds(&line(&["Authorization:", "Bearer", &jwt], 0.0)), ["token"]);
        let gh = ["gh", "p_", "A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8"].concat();
        assert_eq!(kinds(&line(&[&gh], 0.0)), ["api_key"]);
        let (pre, body) = gh.split_once('_').unwrap();
        assert_eq!(kinds(&line(&[pre, body], 0.0)), ["api_key"], "underscore lost");
        let url = ["https://app.example.net/callback?co", "de=4/0AeanS0b8Kq2Zx7Lm1Np9Rt3Vw5Yz"].concat();
        assert_eq!(kinds(&line(&[&url], 0.0)), ["token"]);
        let pplx = ["pplx", "-", "8fK2mQ9xL4tR7vB1nZ6wY3hJ0cD5sAeP2"].concat();
        assert_eq!(kinds(&line(&[&pplx], 0.0)), ["api_key"]);
        let labelled = ["Q7f2Lx9mZ4rT1vB8", "nK3wY6hJ"].concat();
        let mut w = line(&["Client", "secret"], 0.0);
        w.extend(line(&[&labelled], 30.0));
        assert_eq!(kinds(&w), ["api_key"], "label above value");
    }

    #[test]
    fn decoys_are_quiet() {
        for t in [
            "Settings General Notifications Privacy",
            "The quick brown fox jumps over the lazy dog",
            "Version 1.2.3 build 4567 released 2024-10-31",
            "Total: $1,234.56 due 11/30",
            "git commit -m fix-tokenizer-settings",
            "risk-free investment strategies explained",
            "skeleton key tokenizer secret santa",
            "Password reset link sent",
            "sha512-Q7f2Lx9mZ4rT1vB8nK3wY6hJ0cD5sAeP",
            "https://github.com/Samin12/cutroom/pull/12",
            "Request ID 3f2b8c1e 9a7d 4e6f",
            "Order #48213 shipped",
            "12:03:45 PM",
        ] {
            let w = line(&t.split(' ').collect::<Vec<_>>(), 0.0);
            assert!(det().detect(&w).is_empty(), "false positive on decoy {t:?}");
        }
    }

    #[test]
    fn previews_are_redacted() {
        assert_eq!(redact("email", "jane.doe@examplemail.net"), "ja***@e***");
        assert_eq!(redact("api_key", &fake_sk()), "sk-pro...");
        assert_eq!(redact("phone", "123"), "1...");
        let o = det().detect(&line(&[&fake_sk()], 0.0));
        assert_eq!(o[0].preview, "sk-pro...");
        assert!(!format!("{o:?}").contains(&fake_sk()));
    }

    #[test]
    fn luhn_and_lev() {
        assert!(luhn(&[4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 2]));
        assert!(!luhn(&[4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 2, 4, 3]));
        assert!(lev1("abcdef", "abcxef") && lev1("abc", "abcd") && !lev1("abc", "axy"));
    }
}
