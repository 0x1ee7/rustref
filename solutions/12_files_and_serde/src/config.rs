//! Sync job configuration parsed from JSON with serde, then validated with
//! `#[serde(try_from)]` so an invalid config can never be constructed.

use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Error, Result};

/// Largest accepted retention, roughly one century.
const MAX_RETENTION_DAYS: u32 = 36_500;

fn default_retention_days() -> u32 {
    30
}

/// Object form of an ignore rule. `deny_unknown_fields` catches typos.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetailedIgnore {
    /// Glob pattern, same syntax as [`crate::glob`].
    pub pattern: String,
    /// Free-form note for humans.
    #[serde(default)]
    pub reason: Option<String>,
}

/// An ignore rule written either as a bare string or as an object.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum IgnoreRule {
    /// `"*.tmp"`
    Pattern(String),
    /// `{"pattern": "*.tmp", "reason": "scratch files"}`
    Detailed(DetailedIgnore),
}

impl IgnoreRule {
    /// The glob pattern, regardless of the input form.
    #[must_use]
    pub fn pattern(&self) -> &str {
        match self {
            Self::Pattern(pattern) => pattern,
            Self::Detailed(detailed) => &detailed.pattern,
        }
    }
}

/// A validated sync job description.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawSyncConfig")]
pub struct SyncConfig {
    /// Directory to back up from.
    pub source: PathBuf,
    /// Directory to keep the mirror in.
    pub destination: PathBuf,
    /// Glob patterns to skip during scans.
    pub ignore: Vec<String>,
    /// Days to keep removed files (0..=36500).
    pub retention_days: u32,
}

/// Wire shape: kebab-case keys, unknown keys rejected, defaults applied.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct RawSyncConfig {
    source: PathBuf,
    destination: PathBuf,
    #[serde(default)]
    ignore: Vec<IgnoreRule>,
    #[serde(default = "default_retention_days")]
    retention_days: u32,
}

impl TryFrom<RawSyncConfig> for SyncConfig {
    type Error = Error;

    fn try_from(raw: RawSyncConfig) -> Result<Self> {
        if raw.source.as_os_str().is_empty() {
            return Err(Error::Config("`source` must not be empty".to_owned()));
        }
        if raw.destination.as_os_str().is_empty() {
            return Err(Error::Config("`destination` must not be empty".to_owned()));
        }
        if raw.retention_days > MAX_RETENTION_DAYS {
            return Err(Error::Config(format!(
                "`retention-days` must be in 0..={MAX_RETENTION_DAYS}, got {}",
                raw.retention_days
            )));
        }
        if raw.ignore.iter().any(|rule| rule.pattern().is_empty()) {
            return Err(Error::Config("ignore patterns must not be empty".to_owned()));
        }
        Ok(Self {
            source: raw.source,
            destination: raw.destination,
            ignore: raw
                .ignore
                .into_iter()
                .map(|rule| match rule {
                    IgnoreRule::Pattern(pattern) => pattern,
                    IgnoreRule::Detailed(detailed) => detailed.pattern,
                })
                .collect(),
            retention_days: raw.retention_days,
        })
    }
}

impl SyncConfig {
    /// Parses and validates a JSON document.
    ///
    /// # Errors
    /// [`Error::Json`] for syntax errors; [`Error::Config`] for schema or validation failures.
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|err| match err.classify() {
            serde_json::error::Category::Data => Error::Config(err.to_string()),
            _ => Error::Json(err),
        })
    }
}
