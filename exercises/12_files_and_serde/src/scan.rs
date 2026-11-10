//! Tree scanning: walk a directory, hash files, build a [`Manifest`].

use std::collections::BTreeMap;
use std::path::Path;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::codec;
use crate::error::Result;
use crate::hash::Digest;

/// Per-file metadata captured by a scan.
///
/// Wire format: camelCase keys, unknown keys rejected, `mtime` as RFC 3339 and
/// `sha256` as 64 hex characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEntry {
    /// Length in bytes.
    pub size: u64,
    /// Modification time, UTC, truncated to microseconds.
    #[serde(with = "codec::rfc3339")]
    pub mtime: Timestamp,
    /// SHA-256 of the contents.
    #[serde(with = "codec::hex_digest")]
    pub sha256: Digest,
}

/// Relative POSIX path (`pkg/mod.py`) to metadata. Sorted by key.
pub type Manifest = BTreeMap<String, FileEntry>;

/// Recursively scans `root`, skipping paths matched by `ignore`.
///
/// Symlinks and other non-regular files are skipped. Keys use `/` separators on
/// every platform.
///
/// # Errors
/// * [`Error::RootNotFound`](crate::error::Error::RootNotFound) /
///   [`Error::NotADirectory`](crate::error::Error::NotADirectory) for a bad root.
/// * [`Error::NonUtf8Path`](crate::error::Error::NonUtf8Path) for a file name that is not UTF-8.
/// * [`Error::Io`](crate::error::Error::Io) for any other filesystem failure.
pub fn scan_tree(_root: &Path, _ignore: &[&str]) -> Result<Manifest> {
    todo!()
}
