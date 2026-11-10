//! Part 1 — ownership in signatures: borrowed vs owned header data.

use std::borrow::Cow;

/// Why a header name or value was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    /// The name was empty after trimming ASCII whitespace.
    #[error("header name is empty")]
    EmptyName,
    /// The name contained a character outside the RFC 9110 `token` set.
    #[error("invalid character {0:?} in header name")]
    InvalidNameChar(char),
    /// The value contained an ASCII control character other than HTAB.
    #[error("header value contains a control character")]
    InvalidValue,
}

/// Trims ASCII whitespace from `name`, validates it as an RFC 9110 token and
/// lowercases it.
///
/// Returns `Cow::Borrowed` (a sub-slice of `name`) when the trimmed name is
/// already lowercase, and `Cow::Owned` only when lowercasing changed it.
pub fn normalize_header_name(_name: &str) -> Result<Cow<'_, str>, HeaderError> {
    todo!()
}

/// A validated header with a normalized (lowercase) name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    // private fields: name and value storage (engineer's choice)
}

impl Header {
    /// Creates a header. `name` accepts anything `AsRef<str>`; `value` accepts
    /// anything `Into<String>` so owned input is moved, not copied.
    pub fn new(_name: impl AsRef<str>, _value: impl Into<String>) -> Result<Self, HeaderError> {
        todo!()
    }

    /// The normalized (trimmed, lowercase) name.
    pub fn name(&self) -> &str {
        todo!()
    }

    /// The value, stored verbatim.
    pub fn value(&self) -> &str {
        todo!()
    }

    /// Moves both strings out without cloning.
    pub fn into_parts(self) -> (String, String) {
        todo!()
    }
}
