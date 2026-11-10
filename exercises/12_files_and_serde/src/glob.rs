//! Minimal POSIX-style glob matching for ignore rules.
//!
//! Supported syntax: `*` (any run of characters within one path segment),
//! `?` (one character within a segment) and `**` (as a whole segment, any number
//! of segments including zero). Matching is case-sensitive.

/// Whole-path match in the spirit of `PurePosixPath.full_match`.
///
/// The pattern must match the entire `path`, segment by segment.
#[must_use]
pub fn full_match(_pattern: &str, _path: &str) -> bool {
    todo!()
}

/// Ignore rule check for one relative POSIX path.
///
/// * A pattern **without** `/` matches if any path component matches it.
/// * A pattern **with** `/` must match the whole path (anchored at the root).
#[must_use]
pub fn is_ignored(_rel_path: &str, _patterns: &[&str]) -> bool {
    todo!()
}
