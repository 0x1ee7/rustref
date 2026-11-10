//! Part 1 — escaping and unescaping with `Cow<'_, str>`.
//!
//! Escape sequences for the INI value syntax: `\\`, `\n`, `\t`, `\r`, `\"`.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;

fn needs_escape(c: char) -> bool {
    matches!(c, '\\' | '\n' | '\t' | '\r' | '"')
}

/// Escapes `input` so it can be written as a single-line INI value.
///
/// Returns `Cow::Borrowed` when no character needs escaping, so the common
/// case allocates nothing.
pub fn escape(input: &str) -> Cow<'_, str> {
    if !input.chars().any(needs_escape) {
        return Cow::Borrowed(input);
    }
    let mut out = String::with_capacity(input.len() + 8);
    escape_into(input, &mut out);
    Cow::Owned(out)
}

/// Appends the escaped form of `input` to `out`. Never clears `out`.
pub fn escape_into(input: &str, out: &mut String) {
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
}

/// Decodes escape sequences in `input`.
///
/// Returns `Cow::Borrowed` when `input` contains no backslash.
pub fn unescape(input: &str) -> Result<Cow<'_, str>, UnescapeError> {
    let Some(first) = input.find('\\') else {
        return Ok(Cow::Borrowed(input));
    };
    let mut out = String::with_capacity(input.len());
    out.push_str(&input[..first]);
    let mut rest = input[first..].char_indices();
    while let Some((i, c)) = rest.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let offset = first + i;
        match rest.next() {
            None => return Err(UnescapeError::TrailingBackslash { offset }),
            Some((_, 'n')) => out.push('\n'),
            Some((_, 't')) => out.push('\t'),
            Some((_, 'r')) => out.push('\r'),
            Some((_, '\\')) => out.push('\\'),
            Some((_, '"')) => out.push('"'),
            Some((_, found)) => return Err(UnescapeError::UnknownEscape { offset, found }),
        }
    }
    Ok(Cow::Owned(out))
}

/// Why [`unescape`] rejected its input. Offsets are byte indices into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescapeError {
    /// A backslash followed by a character that is not a known escape.
    UnknownEscape { offset: usize, found: char },
    /// The input ends with a lone backslash.
    TrailingBackslash { offset: usize },
}

impl fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnescapeError::UnknownEscape { offset, found } => {
                write!(f, "unknown escape `\\{found}` at byte {offset}")
            }
            UnescapeError::TrailingBackslash { offset } => {
                write!(f, "trailing backslash at byte {offset}")
            }
        }
    }
}

impl Error for UnescapeError {}
