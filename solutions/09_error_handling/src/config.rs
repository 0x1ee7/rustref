//! Parts 1 and 2: config loading, the error hierarchy, settings lookup and
//! application-edge errors.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;

use anyhow::Context;
use serde_json::Value;

/// Parsed config root. The type guarantees the root is a JSON object.
pub type Config = serde_json::Map<String, Value>;

/// Type-erased error for boundaries where the concrete type is not part of the contract.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Why file contents could not be turned into a [`Config`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The bytes are not UTF-8.
    #[error("config is not valid UTF-8")]
    NotUtf8(#[from] Utf8Error),
    /// The text is not well-formed JSON. The message carries line and column.
    #[error("malformed JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Valid JSON, but the top-level value is not an object.
    #[error("top-level JSON value must be an object")]
    NotAnObject,
}

/// Errors from [`load_config`]. Every variant that wraps a cause carries the path.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The file does not exist.
    #[error("config file not found: {}", .path.display())]
    NotFound {
        /// The path that was opened.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
    /// The file exists but cannot be read (permissions, a directory, I/O failure).
    #[error("cannot read config file {}", .path.display())]
    Unreadable {
        /// The path that was opened.
        path: PathBuf,
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },
    /// The file was read but its contents are not a valid config.
    #[error("invalid config file {}", .path.display())]
    Malformed {
        /// The path that was read.
        path: PathBuf,
        /// Why the contents were rejected.
        #[source]
        source: ParseError,
    },
    /// A settings lookup failed. Transparent: displays and chains as the inner error.
    #[error(transparent)]
    Setting(#[from] SettingError),
}

/// A required setting is absent. Written by hand: `Display` and `Error`, no derive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingMissing {
    key: String,
}

impl SettingMissing {
    /// Creates the error for `key`, the full dotted path that was looked up.
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }

    /// The dotted key that was looked up.
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for SettingMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing setting '{}'", self.key)
    }
}

impl Error for SettingMissing {}

/// A settings lookup failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SettingError {
    /// The key (or a segment of it) is absent, or a non-object sits in the path.
    #[error(transparent)]
    Missing(#[from] SettingMissing),
    /// The value exists but the caller's cast rejected it.
    #[error("setting '{key}' has invalid value {raw}")]
    Invalid {
        /// The dotted key.
        key: String,
        /// The raw JSON text of the value.
        raw: String,
        /// The cast's error.
        #[source]
        source: BoxError,
    },
}

/// Returned by a port cast when the value is not a usable TCP port.
#[derive(Debug, thiserror::Error)]
#[error("expected a port number in 1..=65535")]
pub struct InvalidPort;

/// Reads a config file as UTF-8 JSON whose root is an object.
///
/// Missing file → [`ConfigError::NotFound`]; any other I/O failure →
/// [`ConfigError::Unreadable`]; bad UTF-8, bad JSON or a non-object root →
/// [`ConfigError::Malformed`]. Every variant carries the path.
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ConfigError::NotFound {
                path: path.to_path_buf(),
                source,
            }
        } else {
            ConfigError::Unreadable {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;
    parse_config(&bytes).map_err(|source| ConfigError::Malformed {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_config(bytes: &[u8]) -> Result<Config, ParseError> {
    let text = std::str::from_utf8(bytes)?;
    match serde_json::from_str::<Value>(text)? {
        Value::Object(map) => Ok(map),
        _ => Err(ParseError::NotAnObject),
    }
}

/// Looks up `dotted_key` (for example `"db.port"`) through nested objects.
///
/// A missing segment, or a non-object where a segment is expected, yields
/// [`SettingError::Missing`].
pub fn get_setting<'a>(config: &'a Config, dotted_key: &str) -> Result<&'a Value, SettingError> {
    let mut segments = dotted_key.split('.');
    let mut current = segments.next().and_then(|first| config.get(first));
    for segment in segments {
        current = current
            .and_then(Value::as_object)
            .and_then(|object| object.get(segment));
    }
    current.ok_or_else(|| SettingMissing::new(dotted_key).into())
}

/// Like [`get_setting`], then applies `cast` to the found value.
///
/// A missing key never calls `cast`. A cast error becomes
/// [`SettingError::Invalid`], which keeps the key, the raw JSON text and the
/// cast's error as its source.
pub fn get_setting_as<'a, T, E>(
    config: &'a Config,
    dotted_key: &str,
    cast: impl FnOnce(&'a Value) -> Result<T, E>,
) -> Result<T, SettingError>
where
    E: Error + Send + Sync + 'static,
{
    let value = get_setting(config, dotted_key)?;
    cast(value).map_err(|source| SettingError::Invalid {
        key: dotted_key.to_owned(),
        raw: value.to_string(),
        source: Box::new(source),
    })
}

fn port_of(value: &Value) -> Result<u16, InvalidPort> {
    value
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .filter(|&port| port != 0)
        .ok_or(InvalidPort)
}

/// Application boundary: loads `server.port` and returns `anyhow` errors with context.
///
/// The concrete [`ConfigError`] or [`SettingError`] stays downcastable.
pub fn startup_port(path: impl AsRef<Path>) -> anyhow::Result<u16> {
    let config = load_config(path).context("loading config")?;
    let port = get_setting_as(&config, "server.port", port_of).context("reading server.port")?;
    Ok(port)
}
