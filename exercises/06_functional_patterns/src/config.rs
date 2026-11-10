//! Part 5 · Option/Result combinators and `?` for config parsing.

use thiserror::Error;

/// Parsed edge-worker configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    /// Defaults to `false`.
    pub debug: bool,
    /// Defaults to 4. Valid range `1..=64`.
    pub workers: u8,
}

/// Config parse failures. Messages are part of the contract (see README Part 5).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("line {line}: expected 'key = value'")]
    MalformedLine { line: usize },
    #[error("line {line}: duplicate key '{key}'")]
    DuplicateKey { line: usize, key: String },
    #[error("unknown key '{0}'")]
    UnknownKey(String),
    #[error("missing required key '{0}'")]
    MissingKey(&'static str),
    #[error("host must not be empty")]
    EmptyHost,
    #[error("port must be an integer, got '{0}'")]
    InvalidPort(String),
    #[error("port must be between 1 and 65535")]
    PortOutOfRange,
    #[error("expected boolean, got '{0}'")]
    InvalidBool(String),
    #[error("workers must be between 1 and 64")]
    WorkersOutOfRange,
}

/// Trims whitespace, then parses a port in `1..=65535`.
pub fn parse_port(_raw: &str) -> Result<u16, ConfigError> {
    todo!()
}

/// Comma-separated ports. The first error wins.
pub fn parse_ports(_raw: &str) -> Result<Vec<u16>, ConfigError> {
    todo!()
}

/// Case-insensitive boolean (`true/yes/1/on`, `false/no/0/off`). Does not trim.
pub fn parse_bool(_raw: &str) -> Result<bool, ConfigError> {
    todo!()
}

/// `None` stays `None`; `Some(raw)` is parsed with `parse_port`.
pub fn parse_optional_port(_raw: Option<&str>) -> Result<Option<u16>, ConfigError> {
    todo!()
}

/// Fail-fast parser for `key = value` text. Never panics. See README Part 5.
pub fn parse_config(_text: &str) -> Result<Config, ConfigError> {
    todo!()
}
