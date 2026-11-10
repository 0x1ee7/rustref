//! Part 1: 12-factor configuration.
//!
//! Configuration is read through an injected lookup (`Fn(&str) -> Option<String>`),
//! never from the process environment directly, so tests can supply any map.

use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::time::Duration;

/// Wraps a secret value. `Debug` and `Display` print `[REDACTED]`; the value is
/// reachable only through [`Secret::expose`], so every access is explicit.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps `value`.
    pub fn new(_value: T) -> Self {
        todo!("Part 1: Secret::new")
    }

    /// Returns the wrapped value. The only way to read a secret.
    pub fn expose(&self) -> &T {
        todo!("Part 1: Secret::expose")
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("Part 1: Debug must print [REDACTED]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("Part 1: Display must print [REDACTED]")
    }
}

/// Error from a value parser (`parse_bool`, `parse_duration`, `LogLevel::from_str`).
/// The message describes the problem and never echoes a secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub String);

/// Parses `true/false`, `1/0`, `yes/no`, `on/off` (case-insensitive, surrounding whitespace ignored).
pub fn parse_bool(_text: &str) -> Result<bool, ParseError> {
    todo!("Part 1: parse_bool")
}

/// Parses a duration: `500ms`, `30s`, `5m`, `1h`, `1.5s`, or a bare number of seconds (`2`).
/// Rejects empty input, negatives, unknown units and prose such as `5 minutes`.
pub fn parse_duration(_text: &str) -> Result<Duration, ParseError> {
    todo!("Part 1: parse_duration")
}

/// Splits on commas, trims each item and drops empty items. Never fails.
pub fn parse_list(_text: &str) -> Vec<String> {
    todo!("Part 1: parse_list")
}

/// Log verbosity. Parsed case-insensitively from `trace|debug|info|warn|error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Lowercase name, as accepted by `FromStr`.
    pub fn as_str(&self) -> &'static str {
        todo!("Part 1: LogLevel::as_str")
    }
}

impl FromStr for LogLevel {
    type Err = ParseError;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!("Part 1: LogLevel::from_str")
    }
}

/// One bad or missing variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Environment variable name, e.g. `PORT`.
    pub var: &'static str,
    /// Human-readable problem. Must not contain secret values.
    pub problem: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("Part 1: Issue renders as `VAR: problem`")
    }
}

/// Every problem found while loading configuration, not just the first.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ConfigError {
    issues: Vec<Issue>,
}

impl ConfigError {
    /// All issues, in the order the variables are documented on [`Config`].
    pub fn issues(&self) -> &[Issue] {
        todo!("Part 1: ConfigError::issues")
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("Part 1: `invalid configuration: ` followed by issues joined with `; `")
    }
}

/// Typed service configuration. Secrets are wrapped so `Debug` cannot leak them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `LISTEN_ADDR`, default `0.0.0.0:8080`.
    pub listen_addr: SocketAddr,
    /// `APP_NAME`, default `rustref-service`.
    pub app_name: String,
    /// `API_TOKEN`, required, at least 16 characters.
    pub api_token: Secret<String>,
    /// `DATABASE_URL`, required, scheme must be `postgres`, `postgresql` or `sqlite`.
    pub database_url: Secret<String>,
    /// `REQUEST_TIMEOUT`, default `30s`, must be greater than zero.
    pub request_timeout: Duration,
    /// `MAX_CONNECTIONS`, default `256`, must be non-zero.
    pub max_connections: NonZeroU32,
    /// `ALLOWED_HOSTS`, comma list, default empty.
    pub allowed_hosts: Vec<String>,
    /// `DEBUG`, default `false`.
    pub debug: bool,
    /// `LOG_LEVEL`, default `info`.
    pub log_level: LogLevel,
}

impl Config {
    /// Builds a configuration from `lookup`. Empty or whitespace-only values count as unset.
    ///
    /// Collects every problem: a single call reports all bad variables, not the first one.
    pub fn from_env(_lookup: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        todo!("Part 1: Config::from_env")
    }
}
