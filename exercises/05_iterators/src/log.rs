//! Part 2 — a lazy log pipeline: parsing, filtering, enrichment, structured attributes.
//!
//! Line format (fields separated by whitespace; the message is everything after the server):
//!
//! ```text
//! 2026-03-01T12:05:00+00:00 WARNING web-1 disk at 91% on /var
//! ```

use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead};
use std::str::FromStr;

use jiff::Timestamp;

/// Severity, ordered least to most severe (`Ord` follows declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl Level {
    /// Every level, ordered by severity.
    pub const ALL: [Level; 5] = [
        Level::Debug,
        Level::Info,
        Level::Warning,
        Level::Error,
        Level::Critical,
    ];

    /// `true` for `Error` and `Critical`.
    pub fn is_error(self) -> bool {
        todo!()
    }
}

/// Display uses the wire spelling: `WARNING`, `ERROR`, ...
impl fmt::Display for Level {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Parses the exact upper-case wire spelling. Anything else is `UnknownLevel`.
impl FromStr for Level {
    type Err = UnknownLevel;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

/// The text was not one of the five levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLevel(pub String);

impl fmt::Display for UnknownLevel {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for UnknownLevel {}

/// Why a line could not become a [`LogRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseReason {
    /// Fewer than four fields; carries how many were found.
    FieldCount(usize),
    /// The level token is not a known level.
    UnknownLevel(String),
    /// The timestamp is not RFC 3339 *with an offset* (naive timestamps are rejected).
    BadTimestamp(String),
}

impl fmt::Display for ParseReason {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// A malformed log line.
///
/// `line` is the 1-based position in the input stream, `None` when the line was
/// parsed in isolation. Display: `"line N: reason"`, or just the reason when unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogParseError {
    pub line: Option<usize>,
    pub reason: ParseReason,
}

impl fmt::Display for LogParseError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for LogParseError {}

/// One parsed log record.
///
/// `region` is `None` until [`enrich`] fills it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub ts: Timestamp,
    pub level: Level,
    pub server: String,
    pub message: String,
    pub region: Option<String>,
}

/// Parse one line. Surrounding whitespace is ignored; the message keeps its
/// internal spacing exactly. Errors carry `line: None`.
pub fn parse_line(_line: &str) -> Result<LogRecord, LogParseError> {
    todo!()
}

/// Lazily parse an iterator of lines. Blank lines are skipped; malformed lines
/// yield `Err` with `line` set to the 1-based position *including* blank lines.
pub fn parse_records<I>(lines: I) -> ParseRecords<I::IntoIter>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    ParseRecords {
        _lines: lines.into_iter(),
        _lineno: 0,
    }
}

/// Iterator returned by [`parse_records`].
pub struct ParseRecords<I> {
    _lines: I,
    _lineno: usize,
}

impl<I> Iterator for ParseRecords<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = Result<LogRecord, LogParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Failure while reading records from a `BufRead` source.
#[derive(Debug)]
pub enum ReadError {
    /// The source produced a line that is not a valid record.
    Parse(LogParseError),
    /// The underlying reader failed.
    Io(io::Error),
}

impl fmt::Display for ReadError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        todo!()
    }
}

impl From<LogParseError> for ReadError {
    fn from(err: LogParseError) -> Self {
        ReadError::Parse(err)
    }
}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> Self {
        ReadError::Io(err)
    }
}

/// Read records from a buffered reader, one line at a time, without reading ahead
/// of what the consumer has asked for.
pub fn read_records<R: BufRead>(_reader: R) -> ReadRecords<R> {
    todo!()
}

/// Iterator returned by [`read_records`].
pub struct ReadRecords<R> {
    _lines: io::Lines<R>,
    _lineno: usize,
}

impl<R: BufRead> Iterator for ReadRecords<R> {
    type Item = Result<LogRecord, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Keep records at or above `minimum`.
///
/// An unknown `minimum` is reported eagerly, at call time, not on the first `next()`.
pub fn filter_level<I>(
    _records: I,
    _minimum: &str,
) -> Result<impl Iterator<Item = LogRecord>, UnknownLevel>
where
    I: Iterator<Item = LogRecord>,
{
    todo!();
    #[allow(unreachable_code)]
    Ok(std::iter::empty())
}

/// Set `region` from `regions[server]`, defaulting to `"unknown"`. Lazy.
pub fn enrich<'a, I>(
    _records: I,
    _regions: &'a HashMap<String, String>,
) -> impl Iterator<Item = LogRecord> + 'a
where
    I: Iterator<Item = LogRecord> + 'a,
{
    todo!();
    #[allow(unreachable_code)]
    std::iter::empty()
}

/// First record whose level is `Error` or `Critical`. Stops consuming at the match.
pub fn first_error<I>(_records: I) -> Option<LogRecord>
where
    I: IntoIterator<Item = LogRecord>,
{
    todo!()
}

/// One `key=value` pair from a structured message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub key: String,
    pub value: String,
}

/// Why `parse_attrs` rejected its input. `at` is a byte offset of the offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrError {
    /// A token starts with `=` (no key).
    EmptyKey { at: usize },
    /// A key is not followed by `=`.
    MissingEquals { at: usize },
    /// A quoted value never closes.
    UnterminatedQuote { at: usize },
}

impl fmt::Display for AttrError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for AttrError {}

/// Parse whitespace-separated `key=value` pairs. Values may be double-quoted to
/// contain spaces; inside quotes `\"` and `\\` are escapes. `a=` is an empty value.
pub fn parse_attrs(_input: &str) -> Result<Vec<Attr>, AttrError> {
    todo!()
}
