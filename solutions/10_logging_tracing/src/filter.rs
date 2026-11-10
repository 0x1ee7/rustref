//! Part 3 — `EnvFilter` directives for per-module levels.

use std::fmt;

use tracing::level_filters::LevelFilter;
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

/// Parses a level name case-insensitively.
fn parse_level(level: &str) -> Option<LevelFilter> {
    match level.to_ascii_lowercase().as_str() {
        "trace" => Some(LevelFilter::TRACE),
        "debug" => Some(LevelFilter::DEBUG),
        "info" => Some(LevelFilter::INFO),
        "warn" => Some(LevelFilter::WARN),
        "error" => Some(LevelFilter::ERROR),
        "off" => Some(LevelFilter::OFF),
        _ => None,
    }
}

/// Builds an [`EnvFilter`] from a default level and per-target overrides.
///
/// Directives are built with the typed API rather than string parsing, so a
/// target containing `=` or `,` cannot inject extra directives.
pub fn build_filter(default: &str, overrides: &[(&str, &str)]) -> Result<EnvFilter, FilterError> {
    let default_level = parse_level(default).ok_or_else(|| FilterError::UnknownLevel {
        target: String::new(),
        level: default.to_owned(),
    })?;
    let mut filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .parse_lossy("");
    for (target, level) in overrides {
        if target.is_empty() {
            return Err(FilterError::EmptyTarget);
        }
        let level_filter = parse_level(level).ok_or_else(|| FilterError::UnknownLevel {
            target: (*target).to_owned(),
            level: (*level).to_owned(),
        })?;
        let directive = format!("{target}={level_filter}")
            .parse()
            .map_err(|_| FilterError::UnknownLevel {
                target: (*target).to_owned(),
                level: (*level).to_owned(),
            })?;
        filter = filter.add_directive(directive);
    }
    Ok(filter)
}
