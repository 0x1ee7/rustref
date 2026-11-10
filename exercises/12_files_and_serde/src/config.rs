//! Sync job configuration parsed from JSON with serde, then validated with
//! `#[serde(try_from)]` so an invalid config can never be constructed.

use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Error, Result};

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
        todo!()
    }
}

/// A validated sync job description.
///
/// Deserialization goes through a private raw shape and `TryFrom`, so validation
/// runs on every path into this type.
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
#[allow(dead_code)] // fields are read by the engineer's `TryFrom` impl
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

fn default_retention_days() -> u32 {
    todo!()
}

impl TryFrom<RawSyncConfig> for SyncConfig {
    type Error = Error;

    fn try_from(_raw: RawSyncConfig) -> Result<Self> {
        todo!()
    }
}

impl SyncConfig {
    /// Parses and validates a JSON document.
    ///
    /// # Errors
    /// [`Error::Json`] for syntax, unknown-key, or validation failures.
    pub fn from_json(_text: &str) -> Result<Self> {
        todo!()
    }
}
