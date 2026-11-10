//! Part 1: instants, zones, and explicit DST disambiguation.

use jiff::{Timestamp, Zoned, civil, tz::Disambiguation};

use crate::TimeError;

/// The same instant, expressed in UTC.
///
/// Infallible: a `Zoned` always carries an absolute instant.
pub fn to_utc(_zoned: &Zoned) -> Zoned {
    todo!()
}

/// Attach `zone` to a civil wall time and resolve DST ambiguity with `policy`.
///
/// See `README.md` Part 1 for the per-policy rules for gaps and overlaps.
///
/// # Errors
///
/// - [`TimeError::UnknownZone`] for an unknown IANA name.
/// - [`TimeError::Nonexistent`] for a gap the policy does not resolve.
/// - [`TimeError::Ambiguous`] for an overlap under [`Disambiguation::Reject`].
pub fn localize(
    _wall: civil::DateTime,
    _zone: &str,
    _policy: Disambiguation,
) -> Result<Zoned, TimeError> {
    todo!()
}

/// The instant `ts` as a wall time in `zone`.
///
/// # Errors
///
/// [`TimeError::UnknownZone`] for an unknown IANA name.
pub fn in_zone(_ts: Timestamp, _zone: &str) -> Result<Zoned, TimeError> {
    todo!()
}

/// Format `ts` as `"%Y-%m-%d %H:%M %Z"` in `zone`, e.g. `"2026-03-29 14:00 CEST"`.
///
/// # Errors
///
/// [`TimeError::UnknownZone`] for an unknown IANA name.
pub fn format_in_zone(_ts: Timestamp, _zone: &str) -> Result<String, TimeError> {
    todo!()
}
