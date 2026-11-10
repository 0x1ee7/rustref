//! Part 5 · Option/Result combinators and `?` for config parsing.

use std::collections::HashMap;
use std::num::IntErrorKind;

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

/// Keys accepted by `parse_config`.
const KNOWN_KEYS: [&str; 4] = ["host", "port", "debug", "workers"];

/// Trims whitespace, then parses a port in `1..=65535`.
pub fn parse_port(raw: &str) -> Result<u16, ConfigError> {
    match raw.trim().parse::<i64>() {
        Ok(number) => match u16::try_from(number) {
            Ok(port) if port >= 1 => Ok(port),
            _ => Err(ConfigError::PortOutOfRange),
        },
        Err(error) => match error.kind() {
            IntErrorKind::PosOverflow | IntErrorKind::NegOverflow => {
                Err(ConfigError::PortOutOfRange)
            }
            _ => Err(ConfigError::InvalidPort(raw.to_owned())),
        },
    }
}

/// Comma-separated ports. The first error wins.
pub fn parse_ports(raw: &str) -> Result<Vec<u16>, ConfigError> {
    raw.split(',').map(parse_port).collect()
}

/// Case-insensitive boolean (`true/yes/1/on`, `false/no/0/off`). Does not trim.
pub fn parse_bool(raw: &str) -> Result<bool, ConfigError> {
    match raw.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidBool(raw.to_owned())),
    }
}

/// `None` stays `None`; `Some(raw)` is parsed with `parse_port`.
pub fn parse_optional_port(raw: Option<&str>) -> Result<Option<u16>, ConfigError> {
    raw.map(parse_port).transpose()
}

/// Fail-fast parser for `key = value` text. Never panics. See README Part 5.
pub fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let mut values: HashMap<&str, &str> = HashMap::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or(ConfigError::MalformedLine { line: line_no })?;
        let key = key.trim();
        if !KNOWN_KEYS.contains(&key) {
            return Err(ConfigError::UnknownKey(key.to_owned()));
        }
        if values.insert(key, value.trim()).is_some() {
            return Err(ConfigError::DuplicateKey {
                line: line_no,
                key: key.to_owned(),
            });
        }
    }

    let host = *values.get("host").ok_or(ConfigError::MissingKey("host"))?;
    if host.is_empty() {
        return Err(ConfigError::EmptyHost);
    }
    let port = parse_port(values.get("port").ok_or(ConfigError::MissingKey("port"))?)?;
    let debug = values
        .get("debug")
        .map(|raw| parse_bool(raw))
        .transpose()?
        .unwrap_or(false);
    let workers = match values.get("workers") {
        None => 4,
        Some(raw) => raw
            .parse::<u8>()
            .ok()
            .filter(|workers| (1..=64).contains(workers))
            .ok_or(ConfigError::WorkersOutOfRange)?,
    };

    Ok(Config {
        host: host.to_owned(),
        port,
        debug,
        workers,
    })
}
