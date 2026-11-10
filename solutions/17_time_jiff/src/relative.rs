//! Part 7: humanized relative time.

use jiff::{Zoned, tz::TimeZone};

use crate::clock::Clock;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const THIRTY_DAYS: u64 = 30 * DAY;

/// Describe `when` relative to `now` using absolute time.
///
/// Compares instants, so DST and zone labels do not change the result. Within
/// 45 seconds it says `"just now"`. Within 30 days it counts minutes, hours,
/// or days, using `"ago"` for the past and `"in"` for the future. Beyond that
/// it prints the date in `when`'s own zone, for example `"on 2026-10-11"`.
pub fn humanize(when: &Zoned, now: &Zoned) -> String {
    let elapsed = now.timestamp().duration_since(when.timestamp());
    let seconds = elapsed.as_secs();
    let past = seconds >= 0;
    let magnitude = seconds.unsigned_abs();

    if magnitude < 45 {
        "just now".to_owned()
    } else if magnitude < HOUR {
        phrase((magnitude / MINUTE).max(1), "minute", past)
    } else if magnitude < DAY {
        phrase(magnitude / HOUR, "hour", past)
    } else if magnitude < THIRTY_DAYS {
        phrase(magnitude / DAY, "day", past)
    } else {
        format!("on {}", when.date())
    }
}

/// [`humanize`] against `clock.now()`.
pub fn humanize_with_clock(when: &Zoned, clock: &impl Clock) -> String {
    humanize(when, &clock.now().to_zoned(TimeZone::UTC))
}

fn phrase(count: u64, unit: &str, past: bool) -> String {
    let noun = if count == 1 { unit.to_owned() } else { format!("{unit}s") };
    if past {
        format!("{count} {noun} ago")
    } else {
        format!("in {count} {noun}")
    }
}
