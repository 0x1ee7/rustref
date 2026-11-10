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
    /// Every level, in severity order.
    pub const ALL: [Level; 5] = [
        Level::Debug,
        Level::Info,
        Level::Warning,
        Level::Error,
        Level::Critical,
    ];

    /// `true` for `Error` and `Critical`.
    pub fn is_error(self) -> bool {
        matches!(self, Level::Error | Level::Critical)
    }
}

/// Display uses the wire spelling: `WARNING`, `ERROR`, ...
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARNING",
            Level::Error => "ERROR",
            Level::Critical => "CRITICAL",
        };
        f.write_str(s)
    }
}

/// Parses the exact upper-case wire spelling. Anything else is `UnknownLevel`.
impl FromStr for Level {
    type Err = UnknownLevel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEBUG" => Ok(Level::Debug),
            "INFO" => Ok(Level::Info),
            "WARNING" => Ok(Level::Warning),
            "ERROR" => Ok(Level::Error),
            "CRITICAL" => Ok(Level::Critical),
            other => Err(UnknownLevel(other.to_owned())),
        }
    }
}

/// The text was not one of the five levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLevel(pub String);

impl fmt::Display for UnknownLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown level `{}`", self.0)
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
    /// The timestamp token is not a zoned RFC 3339 timestamp.
    BadTimestamp(String),
}

impl fmt::Display for ParseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseReason::FieldCount(n) => write!(f, "expected 4 fields, found {n}"),
            ParseReason::UnknownLevel(s) => write!(f, "unknown level `{s}`"),
            ParseReason::BadTimestamp(s) => write!(f, "bad timestamp `{s}`"),
        }
    }
}

/// A malformed log line.
///
/// `line` is the 1-based position in the input stream, `None` when the line was
/// parsed in isolation. Display: `"line N: reason"`, or just the reason when unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogParseError {
    /// 1-based line number, when known.
    pub line: Option<usize>,
    /// What was wrong with the line.
    pub reason: ParseReason,
}

impl fmt::Display for LogParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(n) => write!(f, "line {n}: {}", self.reason),
            None => write!(f, "{}", self.reason),
        }
    }
}

impl std::error::Error for LogParseError {}

/// One parsed log record.
///
/// `region` is `None` until [`enrich`] fills it in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// When the event happened (zoned).
    pub ts: Timestamp,
    /// Severity.
    pub level: Level,
    /// Emitting server name.
    pub server: String,
    /// Free-form message; internal spacing preserved.
    pub message: String,
    /// Region, filled in by [`enrich`].
    pub region: Option<String>,
}

/// Split off the next whitespace-delimited token. Returns `(token, rest)`.
fn next_token(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    match s.find(char::is_whitespace) {
        Some(i) => Some((&s[..i], &s[i..])),
        None => Some((s, "")),
    }
}

/// Parse one line. Surrounding whitespace is ignored; the message keeps its
/// internal spacing exactly. Errors carry `line: None`.
pub fn parse_line(line: &str) -> Result<LogRecord, LogParseError> {
    let fail = |reason| LogParseError { line: None, reason };

    let mut count = 0;
    let mut rest = line;
    let mut tokens = [""; 3];
    for slot in &mut tokens {
        match next_token(rest) {
            Some((tok, after)) => {
                *slot = tok;
                rest = after;
                count += 1;
            }
            None => return Err(fail(ParseReason::FieldCount(count))),
        }
    }
    let message = rest.trim();
    if message.is_empty() {
        return Err(fail(ParseReason::FieldCount(count)));
    }

    let [ts_tok, level_tok, server] = tokens;
    let ts: Timestamp = ts_tok
        .parse()
        .map_err(|_| fail(ParseReason::BadTimestamp(ts_tok.to_owned())))?;
    let level: Level = level_tok
        .parse()
        .map_err(|e: UnknownLevel| fail(ParseReason::UnknownLevel(e.0)))?;

    Ok(LogRecord {
        ts,
        level,
        server: server.to_owned(),
        message: message.to_owned(),
        region: None,
    })
}

/// Lazily parse an iterator of lines. Blank lines are skipped; malformed lines
/// yield `Err` with `line` set to the 1-based position *including* blank lines.
pub fn parse_records<I>(lines: I) -> ParseRecords<I::IntoIter>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    ParseRecords {
        lines: lines.into_iter(),
        lineno: 0,
    }
}

/// Iterator returned by [`parse_records`].
pub struct ParseRecords<I> {
    lines: I,
    lineno: usize,
}

impl<I> Iterator for ParseRecords<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = Result<LogRecord, LogParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?;
            self.lineno += 1;
            let text = line.as_ref();
            if text.trim().is_empty() {
                continue;
            }
            let lineno = self.lineno;
            return Some(parse_line(text).map_err(|mut e| {
                e.line = Some(lineno);
                e
            }));
        }
    }
}

/// Failure while reading records from a `BufRead` source.
#[derive(Debug)]
pub enum ReadError {
    /// A line was read but could not be parsed.
    Parse(LogParseError),
    /// The underlying reader failed.
    Io(io::Error),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Parse(e) => write!(f, "{e}"),
            ReadError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Parse(e) => Some(e),
            ReadError::Io(e) => Some(e),
        }
    }
}

impl From<LogParseError> for ReadError {
    fn from(e: LogParseError) -> Self {
        ReadError::Parse(e)
    }
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        ReadError::Io(e)
    }
}

/// Read records from a buffered reader, one line at a time, without reading ahead
/// of what the consumer has asked for.
pub fn read_records<R: BufRead>(reader: R) -> ReadRecords<R> {
    ReadRecords {
        lines: reader.lines(),
        lineno: 0,
    }
}

/// Iterator returned by [`read_records`].
pub struct ReadRecords<R> {
    lines: io::Lines<R>,
    lineno: usize,
}

impl<R: BufRead> Iterator for ReadRecords<R> {
    type Item = Result<LogRecord, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = match self.lines.next()? {
                Ok(line) => line,
                Err(e) => return Some(Err(e.into())),
            };
            self.lineno += 1;
            if line.trim().is_empty() {
                continue;
            }
            let lineno = self.lineno;
            return Some(parse_line(&line).map_err(|mut e| {
                e.line = Some(lineno);
                ReadError::from(e)
            }));
        }
    }
}

/// Keep records at or above `minimum`.
///
/// An unknown `minimum` is reported eagerly, at call time, not on the first `next()`.
pub fn filter_level<I>(
    records: I,
    minimum: &str,
) -> Result<impl Iterator<Item = LogRecord>, UnknownLevel>
where
    I: Iterator<Item = LogRecord>,
{
    let min: Level = minimum.parse()?;
    Ok(records.filter(move |r| r.level >= min))
}

/// Set `region` from `regions[server]`, defaulting to `"unknown"`. Lazy.
pub fn enrich<'a, I>(
    records: I,
    regions: &'a HashMap<String, String>,
) -> impl Iterator<Item = LogRecord> + 'a
where
    I: Iterator<Item = LogRecord> + 'a,
{
    records.map(move |mut record| {
        let region = regions
            .get(&record.server)
            .map_or("unknown", String::as_str);
        record.region = Some(region.to_owned());
        record
    })
}

/// First record whose level is `Error` or `Critical`. Stops consuming at the match.
pub fn first_error<I>(records: I) -> Option<LogRecord>
where
    I: IntoIterator<Item = LogRecord>,
{
    records.into_iter().find(|r| r.level.is_error())
}

/// One `key=value` pair from a structured message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    /// The key, never empty.
    pub key: String,
    /// The value, unquoted and unescaped. Empty for `a=`.
    pub value: String,
}

/// Why `parse_attrs` rejected its input. `at` is a byte offset of the offending token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrError {
    /// A token started with `=`.
    EmptyKey { at: usize },
    /// A token had no `=` before the next whitespace or end of input.
    MissingEquals { at: usize },
    /// A quoted value was never closed.
    UnterminatedQuote { at: usize },
}

impl fmt::Display for AttrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttrError::EmptyKey { at } => write!(f, "empty key at byte {at}"),
            AttrError::MissingEquals { at } => write!(f, "expected key=value at byte {at}"),
            AttrError::UnterminatedQuote { at } => {
                write!(f, "unterminated quote starting at byte {at}")
            }
        }
    }
}

impl std::error::Error for AttrError {}

/// Parse whitespace-separated `key=value` pairs. Values may be double-quoted to
/// contain spaces; inside quotes `\"` and `\\` are escapes. `a=` is an empty value.
pub fn parse_attrs(input: &str) -> Result<Vec<Attr>, AttrError> {
    let mut attrs = Vec::new();
    let mut chars = input.char_indices().peekable();

    loop {
        while chars.next_if(|&(_, c)| c.is_whitespace()).is_some() {}
        let Some(&(start, _)) = chars.peek() else {
            return Ok(attrs);
        };

        let mut key = String::new();
        while let Some((_, c)) = chars.next_if(|&(_, c)| c != '=' && !c.is_whitespace()) {
            key.push(c);
        }
        match chars.next() {
            Some((_, '=')) => {}
            _ => return Err(AttrError::MissingEquals { at: start }),
        }
        if key.is_empty() {
            return Err(AttrError::EmptyKey { at: start });
        }

        let mut value = String::new();
        if chars.next_if(|&(_, c)| c == '"').is_some() {
            loop {
                match chars.next() {
                    None => return Err(AttrError::UnterminatedQuote { at: start }),
                    Some((_, '"')) => break,
                    Some((_, '\\')) => match chars.next() {
                        Some((_, c @ ('"' | '\\'))) => value.push(c),
                        Some((_, c)) => {
                            value.push('\\');
                            value.push(c);
                        }
                        None => return Err(AttrError::UnterminatedQuote { at: start }),
                    },
                    Some((_, c)) => value.push(c),
                }
            }
        }
        while let Some((_, c)) = chars.next_if(|&(_, c)| !c.is_whitespace()) {
            value.push(c);
        }

        attrs.push(Attr { key, value });
    }
}
