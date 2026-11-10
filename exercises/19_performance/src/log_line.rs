//! Zero-allocation parser for `<timestamp> <LEVEL> <target> <message>` lines.

use std::fmt;
use std::marker::PhantomData;

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
    /// The upper-case token used in log lines, e.g. `Level::Warn` → `"WARN"`.
    pub fn as_str(self) -> &'static str {
        todo!()
    }
}

/// A required field that was absent from the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Timestamp,
    Level,
    Target,
}

/// Why a line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The first missing required field.
    MissingField(Field),
    /// The level token that is not one of the five known levels (case-sensitive).
    UnknownLevel(String),
}

impl fmt::Display for ParseError {
    /// Names the missing field, or quotes the offending level token.
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
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

/// Parses one line without allocating on success.
///
/// Surrounding ASCII whitespace (including `\r\n`) is ignored. Tokens are
/// separated by any whitespace. The message is everything after the target,
/// with internal spacing kept and trailing whitespace trimmed; it may be empty.
pub fn parse_line(_line: &str) -> Result<LogLine<'_>, ParseError> {
    todo!()
}

/// Iterator over the lines of a buffer. Engineer-owned state.
pub struct LogLines<'a> {
    _input: PhantomData<&'a str>,
}

/// Lazily parses every line of `text`. Blank lines are skipped; malformed lines
/// are yielded as `Err` in their position.
pub fn parse_lines(_text: &str) -> LogLines<'_> {
    todo!()
}

impl<'a> Iterator for LogLines<'a> {
    type Item = Result<LogLine<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
