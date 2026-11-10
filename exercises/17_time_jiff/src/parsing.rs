//! Part 2: heterogeneous timestamp parsing and epoch conversion.

use jiff::Timestamp;

use crate::TimeError;

/// Raw input from an external system: text, an integer epoch, or a float epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawTimestamp<'a> {
    /// RFC 2822, RFC 3339 / ISO 8601, or a numeric string.
    Text(&'a str),
    /// Epoch value; seconds or milliseconds by magnitude.
    Integer(i64),
    /// Epoch value with fractional seconds (or milliseconds by magnitude).
    Float(f64),
}

impl<'a> From<&'a str> for RawTimestamp<'a> {
    fn from(_raw: &'a str) -> Self {
        todo!()
    }
}

impl From<i64> for RawTimestamp<'_> {
    fn from(_raw: i64) -> Self {
        todo!()
    }
}

impl From<f64> for RawTimestamp<'_> {
    fn from(_raw: f64) -> Self {
        todo!()
    }
}

/// Parse any supported input into a [`Timestamp`] (always UTC).
///
/// See `README.md` Part 2 for the full rules.
///
/// # Errors
///
/// [`TimeError::InvalidInput`] for empty text, naive or date-only ISO input,
/// `-0000` RFC 2822 offsets, non-finite floats, and anything unparsable.
pub fn parse_timestamp<'a>(_raw: impl Into<RawTimestamp<'a>>) -> Result<Timestamp, TimeError> {
    todo!()
}

/// Whole milliseconds since the Unix epoch, floored toward the past.
pub fn to_epoch_ms(_ts: Timestamp) -> i64 {
    todo!()
}
