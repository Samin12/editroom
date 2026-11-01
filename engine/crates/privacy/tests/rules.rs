//! The documented example rules file loads, compiles and lands in the scan options.

use cutroom_privacy::{Rules, ScanOpts};
use std::path::Path;

#[test]
fn example_rules_file_loads() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/privacy-rules.example.yaml");
    let r = Rules::load(&p).unwrap();
    assert_eq!(r.patterns.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(), ["community-price", "invoice-number"]);
    let mut o = ScanOpts::default();
    let base = o.allow.len();
    r.apply(&mut o);
    assert_eq!(o.extra_patterns.len(), 2);
    assert_eq!(o.allow.len(), base + 2);
}

#[test]
fn bad_regex_fails_on_load() {
    let d = std::env::temp_dir().join("cutroom-privacy-rules-test");
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("privacy-rules.yaml");
    std::fs::write(&f, "patterns:\n  - {name: broken, regex: '(unclosed'}\n").unwrap();
    let e = Rules::load(&f).unwrap_err().to_string();
    assert!(e.contains("broken"), "{e}");
    std::fs::write(&f, "patterns: []\nunknown: 1\n").unwrap();
    assert!(Rules::load(&f).is_err(), "unknown keys are rejected");
}
