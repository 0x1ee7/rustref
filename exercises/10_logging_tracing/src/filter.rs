//! Part 3 — `EnvFilter` directives for per-module levels.

use std::fmt;

use tracing_subscriber::EnvFilter;

/// Why a filter specification was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterError {
    /// An override had an empty target.
    EmptyTarget,
    /// A level was not one of `trace`, `debug`, `info`, `warn`, `error`, `off`
    /// (case-insensitive). `target` is `""` when the bad level is the default.
    UnknownLevel { target: String, level: String },
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::EmptyTarget => write!(f, "filter target must not be empty"),
            FilterError::UnknownLevel { target, level } if target.is_empty() => {
                write!(f, "unknown default level `{level}`")
            }
            FilterError::UnknownLevel { target, level } => {
                write!(f, "unknown level `{level}` for target `{target}`")
            }
        }
    }
}

impl std::error::Error for FilterError {}

/// Builds an [`EnvFilter`] from a default level and per-target overrides.
///
/// Behaviour (Part 3):
/// - `default` applies to every target without an override;
/// - an override `(target, level)` applies to `target` and its child module
///   paths (`shop::db` covers `shop::db::pool`);
/// - `EnvFilter` matches directive targets by plain string prefix, so the
///   override `shop::db` also covers the sibling `shop::dbx`. Document this in
///   the README and do not work around it in `build_filter`;
/// - the most specific directive wins;
/// - level names are case-insensitive; `off` silences a target;
/// - every level and target is validated; the first problem is returned.
pub fn build_filter(default: &str, overrides: &[(&str, &str)]) -> Result<EnvFilter, FilterError> {
    let _ = (default, overrides);
    todo!()
}
