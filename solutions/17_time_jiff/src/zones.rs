//! Part 1: instants, zones, and explicit DST disambiguation.

use jiff::{Timestamp, Zoned, civil, tz::{AmbiguousOffset, Disambiguation, TimeZone}};

use crate::TimeError;

/// The same instant, expressed in UTC.
///
/// Infallible: a `Zoned` always carries an absolute instant.
pub fn to_utc(zoned: &Zoned) -> Zoned {
    zoned.with_time_zone(TimeZone::UTC)
}

/// Look up an IANA zone, mapping jiff's lookup failure to our error type.
pub(crate) fn zone(name: &str) -> Result<TimeZone, TimeError> {
    TimeZone::get(name).map_err(|_| TimeError::UnknownZone(name.to_owned()))
}

/// Resolve a wall time in `zone_name` using an explicit DST policy.
///
/// - [`Disambiguation::Compatible`] shifts a gap forward and takes the
///   earlier occurrence of an overlap (Python's `fold=0`).
/// - [`Disambiguation::Earlier`] and [`Disambiguation::Later`] only apply to
///   overlaps; in a gap they are `Nonexistent`, because jiff's own
///   `earlier()` would silently use the after-transition offset.
/// - [`Disambiguation::Reject`] reports both gaps and overlaps.
///
/// # Errors
///
/// - [`TimeError::UnknownZone`] for an unknown IANA name.
/// - [`TimeError::Nonexistent`] for a gap under a policy that will not shift.
/// - [`TimeError::Ambiguous`] for an overlap under [`Disambiguation::Reject`].
pub fn localize(
    wall: civil::DateTime,
    zone_name: &str,
    policy: Disambiguation,
) -> Result<Zoned, TimeError> {
    let tz = zone(zone_name)?;
    let amb = tz.to_ambiguous_zoned(wall);
    let in_gap = matches!(amb.offset(), AmbiguousOffset::Gap { .. });
    let zoned = match policy {
        Disambiguation::Compatible => amb.compatible()?,
        Disambiguation::Earlier if in_gap => return Err(TimeError::Nonexistent(wall)),
        Disambiguation::Later if in_gap => return Err(TimeError::Nonexistent(wall)),
        Disambiguation::Reject if in_gap => return Err(TimeError::Nonexistent(wall)),
        Disambiguation::Reject if amb.is_ambiguous() => return Err(TimeError::Ambiguous(wall)),
        Disambiguation::Earlier => amb.earlier()?,
        Disambiguation::Later => amb.later()?,
        Disambiguation::Reject => amb.unambiguous()?,
        other => amb.disambiguate(other)?,
    };
    Ok(zoned)
}

/// The instant `ts` rendered as wall time in `zone_name`.
///
/// # Errors
///
/// [`TimeError::UnknownZone`] for an unknown IANA name.
pub fn in_zone(ts: Timestamp, zone_name: &str) -> Result<Zoned, TimeError> {
    Ok(ts.to_zoned(zone(zone_name)?))
}

/// `ts` as `"YYYY-MM-DD HH:MM ABBR"` in `zone_name`, such as `"2026-03-29 14:00 CEST"`.
///
/// # Errors
///
/// [`TimeError::UnknownZone`] for an unknown IANA name.
pub fn format_in_zone(ts: Timestamp, zone_name: &str) -> Result<String, TimeError> {
    Ok(in_zone(ts, zone_name)?.strftime("%Y-%m-%d %H:%M %Z").to_string())
}
