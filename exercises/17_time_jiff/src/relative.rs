//! Part 7: humanized relative time against an injected "now".

use jiff::Zoned;

use crate::clock::Clock;

/// Describe `when` relative to `now` using absolute time.
///
/// See `README.md` Part 7 for the thresholds. Output examples:
/// `"just now"`, `"1 minute ago"`, `"in 59 minutes"`, `"3 hours ago"`,
/// `"in 2 days"`, `"on 2026-10-11"`.
pub fn humanize(_when: &Zoned, _now: &Zoned) -> String {
    todo!()
}

/// [`humanize`] against `clock.now()`, expressed in `when`'s zone.
pub fn humanize_with_clock(_when: &Zoned, _clock: &impl Clock) -> String {
    todo!()
}
