//! Manifest diffing and a path-safe apply step.

use std::path::Path;

use crate::error::Result;
use crate::scan::Manifest;

/// Changes needed to turn one manifest into another. Borrows the keys from the
/// manifests it was computed from. All lists are sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestDiff<'a> {
    /// In the new manifest only.
    pub added: Vec<&'a str>,
    /// In the old manifest only.
    pub removed: Vec<&'a str>,
    /// In both, but size or SHA-256 differs. An mtime-only change is not a modification.
    pub modified: Vec<&'a str>,
}

impl ManifestDiff<'_> {
    /// `true` when there is nothing to do.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// Computes what changed from `old` to `new`.
#[must_use]
pub fn diff_manifests<'a>(_old: &'a Manifest, _new: &'a Manifest) -> ManifestDiff<'a> {
    todo!()
}

/// Checks that a manifest key is a safe relative POSIX path below some root.
///
/// Rejects: empty keys or segments (`""`, `a//b`, `a/`), `.`/`..` segments,
/// backslashes, absolute paths, and Windows drive or UNC prefixes.
///
/// # Errors
/// [`Error::UnsafePath`](crate::error::Error::UnsafePath) describing the first problem found.
pub fn validate_relative_path(_key: &str) -> Result<()> {
    todo!()
}

/// Result of [`apply_diff`]. Borrows the keys from the diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyResult<'a> {
    /// Keys copied from source to destination (added and modified).
    pub copied: Vec<&'a str>,
    /// Keys that existed under the destination and were removed.
    pub deleted: Vec<&'a str>,
}

/// Applies `diff` by copying from `source` into `dest`.
///
/// Every key is validated with [`validate_relative_path`] before the first disk
/// change. Copies go through an atomic write and keep the source mtime. Removed
/// files that are already absent are not reported. After a removal, empty parent
/// directories are pruned up to, but not including, `dest`.
///
/// # Errors
/// [`Error::UnsafePath`](crate::error::Error::UnsafePath) before touching disk, or
/// [`Error::Io`](crate::error::Error::Io) on filesystem failure.
pub fn apply_diff<'a>(
    _diff: &ManifestDiff<'a>,
    _source: &Path,
    _dest: &Path,
) -> Result<ApplyResult<'a>> {
    todo!()
}
