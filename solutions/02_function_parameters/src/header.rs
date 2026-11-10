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

/// RFC 9110 `tchar`: ALPHA / DIGIT / one of `!#$%&'*+-.^_`|~`.
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' | '^' | '_' | '`' | '|' | '~'
        )
}

/// Trims ASCII whitespace from `name`, validates it as an RFC 9110 token and
/// lowercases it.
///
/// Returns `Cow::Borrowed` (a sub-slice of `name`) when the trimmed name is
/// already lowercase, and `Cow::Owned` only when lowercasing changed it.
pub fn normalize_header_name(name: &str) -> Result<Cow<'_, str>, HeaderError> {
    let trimmed = name.trim_ascii();
    if trimmed.is_empty() {
        return Err(HeaderError::EmptyName);
    }
    if let Some(bad) = trimmed.chars().find(|&c| !is_token_char(c)) {
        return Err(HeaderError::InvalidNameChar(bad));
    }
    if trimmed.bytes().any(|b| b.is_ascii_uppercase()) {
        Ok(Cow::Owned(trimmed.to_ascii_lowercase()))
    } else {
        Ok(Cow::Borrowed(trimmed))
    }
}

/// A validated header with a normalized (lowercase) name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    /// Creates a header. `name` accepts anything `AsRef<str>`; `value` accepts
    /// anything `Into<String>` so owned input is moved, not copied.
    pub fn new(name: impl AsRef<str>, value: impl Into<String>) -> Result<Self, HeaderError> {
        let name = normalize_header_name(name.as_ref())?.into_owned();
        let value = value.into();
        if value.bytes().any(|b| b.is_ascii_control() && b != b'\t') {
            return Err(HeaderError::InvalidValue);
        }
        Ok(Self { name, value })
    }

    /// The normalized (trimmed, lowercase) name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value, stored verbatim.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Moves both strings out without cloning.
    pub fn into_parts(self) -> (String, String) {
        (self.name, self.value)
    }
}
