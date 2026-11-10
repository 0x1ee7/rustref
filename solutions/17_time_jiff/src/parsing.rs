//! Part 2: heterogeneous timestamp parsing and epoch conversion.

use jiff::{SignedDuration, Timestamp};

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
    fn from(raw: &'a str) -> Self {
        Self::Text(raw)
    }
}

impl From<i64> for RawTimestamp<'_> {
    fn from(raw: i64) -> Self {
        Self::Integer(raw)
    }
}

impl From<f64> for RawTimestamp<'_> {
    fn from(raw: f64) -> Self {
        Self::Float(raw)
    }
}

/// Epoch magnitudes at or above this value are milliseconds, otherwise seconds.
/// `100_000_000_000` seconds is year 5138, so no plausible seconds value reaches it.
const MILLIS_THRESHOLD: u64 = 100_000_000_000;

/// Parse any supported input into a [`Timestamp`].
///
/// Text is trimmed. It may be a numeric epoch, RFC 3339 / ISO 8601 with an
/// explicit offset, or RFC 2822 with a numeric offset. Numbers are epoch
/// seconds or milliseconds by magnitude.
///
/// # Errors
///
/// [`TimeError::InvalidInput`] for blank or unparsable text, naive times
/// without an offset, RFC 2822 `-0000`, and non-finite floats.
/// [`TimeError::Jiff`] when a number is outside jiff's range.
pub fn parse_timestamp<'a>(raw: impl Into<RawTimestamp<'a>>) -> Result<Timestamp, TimeError> {
    match raw.into() {
        RawTimestamp::Text(text) => parse_text(text),
        RawTimestamp::Integer(n) => parse_epoch_int(n),
        RawTimestamp::Float(x) => parse_epoch_float(x),
    }
}

/// Whole milliseconds since the Unix epoch, floored toward the past.
pub fn to_epoch_ms(ts: Timestamp) -> i64 {
    // `div_euclid` floors for a positive divisor, so -1.5 ms becomes -2.
    let ms = ts.as_nanosecond().div_euclid(1_000_000);
    i64::try_from(ms).expect("jiff timestamps fit in i64 milliseconds")
}

fn invalid(message: impl Into<String>) -> TimeError {
    TimeError::InvalidInput(message.into())
}

fn parse_text(text: &str) -> Result<Timestamp, TimeError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(invalid("empty timestamp"));
    }
    if let Ok(n) = text.parse::<i64>() {
        return parse_epoch_int(n);
    }
    // RFC 2822 §3.3: `-0000` says the offset is unknown, so the instant is not
    // determined. jiff reads it as UTC, which would silently invent a time.
    if text.rsplit(' ').next() == Some("-0000") {
        return Err(invalid(format!("{text:?} has an unknown offset (-0000)")));
    }
    if let Ok(ts) = text.parse::<Timestamp>() {
        return Ok(ts);
    }
    jiff::fmt::rfc2822::parse(text)
        .map(|zoned| zoned.timestamp())
        .map_err(|_| invalid(format!("cannot parse {text:?} as a timestamp")))
}

fn parse_epoch_int(n: i64) -> Result<Timestamp, TimeError> {
    let ts = if n.unsigned_abs() >= MILLIS_THRESHOLD {
        Timestamp::from_millisecond(n)?
    } else {
        Timestamp::from_second(n)?
    };
    Ok(ts)
}

fn parse_epoch_float(x: f64) -> Result<Timestamp, TimeError> {
    if !x.is_finite() {
        return Err(invalid(format!("non-finite epoch value {x}")));
    }
    let seconds = if x.abs() >= MILLIS_THRESHOLD as f64 { x / 1_000.0 } else { x };
    let whole = seconds.trunc();
    let nanos = ((seconds - whole) * 1e9).round() as i64;
    let base = Timestamp::from_second(whole as i64)?;
    Ok(base.checked_add(SignedDuration::from_nanos(nanos))?)
}
