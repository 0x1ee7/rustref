//! Part 1 — escaping and unescaping with `Cow<'_, str>`.
//!
//! Escape sequences for the INI value syntax: `\\`, `\n`, `\t`, `\r`, `\"`.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;

/// Escapes `input` so it can be written as a single-line INI value.
///
/// Returns `Cow::Borrowed` when no character needs escaping.
pub fn escape(_input: &str) -> Cow<'_, str> {
    todo!()
}

/// Appends the escaped form of `input` to `out`. Never clears `out`.
pub fn escape_into(_input: &str, _out: &mut String) {
    todo!()
}

/// Decodes escape sequences in `input`.
///
/// Returns `Cow::Borrowed` when `input` contains no backslash.
pub fn unescape(_input: &str) -> Result<Cow<'_, str>, UnescapeError> {
    todo!()
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
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for UnescapeError {}
