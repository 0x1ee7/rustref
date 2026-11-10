//! Part 3: wall-clock (calendar) vs elapsed (absolute) arithmetic.

use jiff::{SignedDuration, Zoned};

use crate::TimeError;

/// Move `zoned` by `days` calendar days: wall-clock time and zone are kept,
/// and the offset is recomputed with fold-0 (`Compatible`) semantics.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_calendar_days(_zoned: &Zoned, _days: i64) -> Result<Zoned, TimeError> {
    todo!()
}

/// Move `zoned` by exact elapsed time. The result stays in `zoned`'s zone.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_elapsed(_zoned: &Zoned, _delta: SignedDuration) -> Result<Zoned, TimeError> {
    todo!()
}

/// Signed absolute difference `end - start`.
pub fn elapsed_between(_start: &Zoned, _end: &Zoned) -> SignedDuration {
    todo!()
}
