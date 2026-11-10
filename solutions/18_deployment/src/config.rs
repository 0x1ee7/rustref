//! Part 1: 12-factor configuration.
//!
//! Configuration is read through an injected lookup (`Fn(&str) -> Option<String>`),
//! never from the process environment directly, so tests can supply any map.

use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::time::Duration;

const MIN_TOKEN_LEN: usize = 16;
const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_APP_NAME: &str = "rustref-service";
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_CONNECTIONS: u32 = 256;

/// Wraps a secret value. `Debug` and `Display` print `[REDACTED]`; the value is
/// reachable only through [`Secret::expose`], so every access is explicit.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps `value`.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Returns the wrapped value. The only way to read a secret.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Error from a value parser (`parse_bool`, `parse_duration`, `LogLevel::from_str`).
/// The message describes the problem and never echoes a secret.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub String);

/// Parses `true/false`, `1/0`, `yes/no`, `on/off` (case-insensitive, surrounding whitespace ignored).
pub fn parse_bool(text: &str) -> Result<bool, ParseError> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ParseError(format!(
            "expected true/false, yes/no, on/off or 1/0, got {text:?}"
        ))),
    }
}

/// Parses a duration: `500ms`, `30s`, `5m`, `1h`, `1.5s`, or a bare number of seconds (`2`).
/// Rejects empty input, negatives, unknown units and prose such as `5 minutes`.
pub fn parse_duration(text: &str) -> Result<Duration, ParseError> {
    let trimmed = text.trim();
    let split = trimmed
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(split);
    if number.is_empty() {
        return Err(ParseError(format!(
            "expected a duration such as 30s or 5m, got {text:?}"
        )));
    }
    let value: f64 = number
        .parse()
        .map_err(|_| ParseError(format!("invalid number in duration {text:?}")))?;
    let seconds = match unit {
        "" | "s" => value,
        "ms" => value / 1000.0,
        "m" => value * 60.0,
        "h" => value * 3600.0,
        other => {
            return Err(ParseError(format!(
                "unknown duration unit {other:?}; use ms, s, m or h"
            )));
        }
    };
    Duration::try_from_secs_f64(seconds)
        .map_err(|_| ParseError(format!("duration {text:?} is out of range")))
}

/// Splits on commas, trims each item and drops empty items. Never fails.
pub fn parse_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
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
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl FromStr for LogLevel {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(ParseError(format!(
                "expected trace, debug, info, warn or error, got {s:?}"
            ))),
        }
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.var, self.problem)
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
        &self.issues
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid configuration: ")?;
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{issue}")?;
        }
        Ok(())
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
    pub fn from_env(lookup: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
        let mut issues = Vec::new();

        let api_token = parse_required(&lookup, "API_TOKEN", &mut issues, |value| {
            if value.chars().count() < MIN_TOKEN_LEN {
                Err(format!("must be at least {MIN_TOKEN_LEN} characters"))
            } else {
                Ok(Secret::new(value.to_owned()))
            }
        });
        let database_url = parse_required(&lookup, "DATABASE_URL", &mut issues, |value| {
            match value.split_once("://") {
                None => Err("must be a URL such as postgres://host/db".to_owned()),
                Some(("postgres" | "postgresql" | "sqlite", _)) => Ok(Secret::new(value.to_owned())),
                Some((scheme, _)) => Err(format!(
                    "unsupported scheme {scheme:?}; use postgres, postgresql or sqlite"
                )),
            }
        });

        let listen_addr = parse_optional(&lookup, "LISTEN_ADDR", &mut issues, |value| {
            value
                .parse::<SocketAddr>()
                .map_err(|e| format!("not a socket address: {e}"))
        })
        .unwrap_or_else(|| {
            DEFAULT_LISTEN_ADDR
                .parse()
                .expect("default listen address is valid")
        });
        let app_name = parse_optional(&lookup, "APP_NAME", &mut issues, |value| {
            Ok(value.to_owned())
        })
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_owned());
        let request_timeout = parse_optional(&lookup, "REQUEST_TIMEOUT", &mut issues, |value| {
            let duration = parse_duration(value).map_err(|e| e.0)?;
            if duration.is_zero() {
                Err("must be greater than zero".to_owned())
            } else {
                Ok(duration)
            }
        })
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT);
        let max_connections = parse_optional(&lookup, "MAX_CONNECTIONS", &mut issues, |value| {
            let n: u32 = value
                .parse()
                .map_err(|e| format!("not a whole number: {e}"))?;
            NonZeroU32::new(n).ok_or_else(|| "must be greater than zero".to_owned())
        })
        .unwrap_or_else(|| {
            NonZeroU32::new(DEFAULT_MAX_CONNECTIONS).expect("default connection limit is non-zero")
        });
        let allowed_hosts = parse_optional(&lookup, "ALLOWED_HOSTS", &mut issues, |value| {
            Ok(parse_list(value))
        })
        .unwrap_or_default();
        let debug = parse_optional(&lookup, "DEBUG", &mut issues, |value| {
            parse_bool(value).map_err(|e| e.0)
        })
        .unwrap_or(false);
        let log_level = parse_optional(&lookup, "LOG_LEVEL", &mut issues, |value| {
            value.parse::<LogLevel>().map_err(|e| e.0)
        })
        .unwrap_or(LogLevel::Info);

        if !issues.is_empty() {
            return Err(ConfigError { issues });
        }
        Ok(Config {
            listen_addr,
            app_name,
            api_token: api_token.expect("no issues recorded, so API_TOKEN parsed"),
            database_url: database_url.expect("no issues recorded, so DATABASE_URL parsed"),
            request_timeout,
            max_connections,
            allowed_hosts,
            debug,
            log_level,
        })
    }
}

/// Returns the trimmed value of `var`, or `None` when unset or blank.
fn raw(lookup: &impl Fn(&str) -> Option<String>, var: &str) -> Option<String> {
    lookup(var)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Parses a present value. Records an issue and returns `None` when it is invalid.
fn apply<T>(
    var: &'static str,
    value: &str,
    issues: &mut Vec<Issue>,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Option<T> {
    match parse(value) {
        Ok(parsed) => Some(parsed),
        Err(problem) => {
            issues.push(Issue { var, problem });
            None
        }
    }
}

/// Optional variable: `None` when unset (caller supplies the default) or invalid (issue recorded).
fn parse_optional<T>(
    lookup: &impl Fn(&str) -> Option<String>,
    var: &'static str,
    issues: &mut Vec<Issue>,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Option<T> {
    let value = raw(lookup, var)?;
    apply(var, &value, issues, parse)
}

/// Required variable: records a "not set" issue when missing, otherwise parses it.
fn parse_required<T>(
    lookup: &impl Fn(&str) -> Option<String>,
    var: &'static str,
    issues: &mut Vec<Issue>,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Option<T> {
    match raw(lookup, var) {
        Some(value) => apply(var, &value, issues, parse),
        None => {
            issues.push(Issue {
                var,
                problem: "required but not set".to_owned(),
            });
            None
        }
    }
}
