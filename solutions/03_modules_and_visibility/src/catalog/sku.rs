use std::fmt;
use std::str::FromStr;

/// Errors from [`Sku::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SkuError {
    /// Fewer than 3 characters after normalisation. Holds the length.
    #[error("SKU too short: {0} characters (minimum 3)")]
    TooShort(usize),
    /// More than 12 characters after normalisation. Holds the length.
    #[error("SKU too long: {0} characters (maximum 12)")]
    TooLong(usize),
    /// A character outside `[A-Z0-9-]` after uppercasing.
    #[error("invalid character {0:?} in SKU")]
    InvalidChar(char),
    /// Leading or trailing dash.
    #[error("SKU must not start or end with '-'")]
    EdgeDash,
    /// Two consecutive dashes.
    #[error("SKU must not contain \"--\"")]
    DoubleDash,
}

/// A validated, normalised stock-keeping unit.
///
/// The inner string is private: the only way to obtain a `Sku` is through
/// validation, so every `Sku` is well-formed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sku {
    // Private on purpose: callers must go through `Sku::new`.
    value: String,
}

impl Sku {
    /// Validates and normalises `raw` (ASCII-uppercased) into a `Sku`.
    ///
    /// # Errors
    ///
    /// Returns a [`SkuError`] describing the first rule that fails.
    pub fn new(raw: &str) -> Result<Self, SkuError> {
        let value = raw.to_ascii_uppercase();
        let len = value.chars().count();
        if len < 3 {
            return Err(SkuError::TooShort(len));
        }
        if len > 12 {
            return Err(SkuError::TooLong(len));
        }
        if let Some(bad) = value
            .chars()
            .find(|c| !(c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(SkuError::InvalidChar(bad));
        }
        if value.starts_with('-') || value.ends_with('-') {
            return Err(SkuError::EdgeDash);
        }
        if value.contains("--") {
            return Err(SkuError::DoubleDash);
        }
        Ok(Self { value })
    }

    /// Returns the normalised text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Sku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl FromStr for Sku {
    type Err = SkuError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for Sku {
    fn as_ref(&self) -> &str {
        &self.value
    }
}
