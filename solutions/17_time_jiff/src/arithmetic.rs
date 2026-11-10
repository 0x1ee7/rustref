//! Part 3: wall-clock (calendar) vs elapsed (absolute) arithmetic.

use jiff::{SignedDuration, Span, Zoned, civil};

use crate::TimeError;

/// Move `zoned` by `days` calendar days: wall-clock time and zone are kept,
/// and the offset is recomputed with fold-0 (`Compatible`) semantics.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_calendar_days(zoned: &Zoned, days: i64) -> Result<Zoned, TimeError> {
    let date = zoned.date().checked_add(Span::new().days(days))?;
    let wall = civil::DateTime::from_parts(date, zoned.time());
    Ok(zoned.time_zone().to_ambiguous_zoned(wall).compatible()?)
}

/// Move `zoned` by an exact amount of elapsed time. The zone is kept and the
/// wall clock moves with the offset.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_elapsed(zoned: &Zoned, delta: SignedDuration) -> Result<Zoned, TimeError> {
    Ok(zoned.checked_add(delta)?)
}

/// Signed absolute difference `end - start`.
pub fn elapsed_between(start: &Zoned, end: &Zoned) -> SignedDuration {
    end.duration_since(start)
}
