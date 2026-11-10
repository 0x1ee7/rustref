//! Zero-allocation parser for `<timestamp> <LEVEL> <target> <message>` lines.

use std::fmt;
use std::str::Lines;

/// Log severity, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    /// The upper-case token used in log lines.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    fn from_token(token: &str) -> Option<Level> {
        Some(match token {
            "TRACE" => Level::Trace,
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" => Level::Warn,
            "ERROR" => Level::Error,
            _ => return None,
        })
    }
}

/// A required field that was absent from the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Timestamp,
    Level,
    Target,
}

impl Field {
    fn name(self) -> &'static str {
        match self {
            Field::Timestamp => "timestamp",
            Field::Level => "level",
            Field::Target => "target",
        }
    }
}

/// Why a line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first missing required field.
    MissingField(Field),
    /// The level token that is not one of the five known levels.
    UnknownLevel(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingField(field) => write!(f, "missing {} field", field.name()),
            ParseError::UnknownLevel(token) => write!(f, "unknown log level {token:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// One parsed line. All text fields borrow from the input buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLine<'a> {
    pub timestamp: &'a str,
    pub level: Level,
    pub target: &'a str,
    pub message: &'a str,
}

/// Splits off the first whitespace-delimited token. The remainder has its
/// leading whitespace removed.
fn next_token(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    if input.is_empty() {
        return None;
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Some((&input[..end], input[end..].trim_start()))
}

/// Parses one line without allocating on success.
pub fn parse_line(line: &str) -> Result<LogLine<'_>, ParseError> {
    let line = line.trim();
    let (timestamp, rest) =
        next_token(line).ok_or(ParseError::MissingField(Field::Timestamp))?;
    let (level_token, rest) = next_token(rest).ok_or(ParseError::MissingField(Field::Level))?;
    let (target, message) = next_token(rest).ok_or(ParseError::MissingField(Field::Target))?;
    let level = Level::from_token(level_token)
        .ok_or_else(|| ParseError::UnknownLevel(level_token.to_owned()))?;
    Ok(LogLine {
        timestamp,
        level,
        target,
        message,
    })
}

/// Iterator over the lines of a buffer; blank lines are skipped.
pub struct LogLines<'a> {
    remaining: Lines<'a>,
}

/// Lazily parses every line of `text`.
pub fn parse_lines(text: &str) -> LogLines<'_> {
    LogLines {
        remaining: text.lines(),
    }
}

impl<'a> Iterator for LogLines<'a> {
    type Item = Result<LogLine<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self
            .remaining
            .by_ref()
            .map(str::trim)
            .find(|line| !line.is_empty())?;
        Some(parse_line(line))
    }
}
