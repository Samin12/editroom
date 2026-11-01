//! cutroom-ingest: turn a raw recording into a cutroom [`Source`](cutroom_core::Source).
//!
//! * [`borumi()`] reads a Borumi project (`*.bmprojbundle`), never writing to it: the SQLite
//!   database is snapshotted with the backup API and only the copy is read.
//!   [`BorumiMode::Raw`] gives every narration take's full recording (what was filmed);
//!   [`BorumiMode::Timeline`] gives what is on the Borumi timeline now.
//! * [`files()`] builds one take from plain camera / screen / mic files, transcribing with
//!   whisper-cli when no transcript JSON is given.

pub mod borumi;
pub mod files;
pub mod probe;
pub mod transcript;

pub use borumi::{borumi, borumi_db, BorumiMode, Project};
pub use files::files;

use anyhow::Result;
use std::path::PathBuf;

/// A fresh private temp folder (`$TMPDIR/cutroom-<tag>-<pid>-<n>`); callers remove it.
pub(crate) fn temp_dir(tag: &str) -> Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    let d = std::env::temp_dir().join(format!("cutroom-{tag}-{}-{}-{nanos}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&d)?;
    Ok(d)
}
