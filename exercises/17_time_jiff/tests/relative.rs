//! Part 7: humanized relative time and the injected clock.

use ex17_time_jiff::clock::{Clock, FixedClock, SystemClock};
use ex17_time_jiff::relative::{humanize, humanize_with_clock};
use jiff::{SignedDuration, Timestamp, Zoned, civil, tz::TimeZone};

fn berlin(y: i16, m: i8, d: i8, h: i8, min: i8) -> Zoned {
    civil::date(y, m, d)
        .at(h, min, 0, 0)
        .in_tz("Europe/Berlin")
        .expect("known zone and unambiguous fixture")
}

fn now() -> Zoned {
    berlin(2026, 11, 10, 12, 0)
}

fn secs(n: i64) -> SignedDuration {
    SignedDuration::from_secs(n)
}

#[test]
fn part7_just_now_within_45_seconds_either_side() {
    let n = now();
    let before = n.checked_sub(secs(44)).unwrap();
    let after = n.checked_add(secs(44)).unwrap();
    assert_eq!(humanize(&before, &n), "just now");
    assert_eq!(humanize(&after, &n), "just now", "future is also 'just now' within 45 s");
}

#[test]
fn part7_minutes_threshold_and_singular_form() {
    let n = now();
    assert_eq!(humanize(&n.checked_sub(secs(45)).unwrap(), &n), "1 minute ago", "45 s rounds up to one minute");
    assert_eq!(humanize(&n.checked_sub(secs(3599)).unwrap(), &n), "59 minutes ago", "59m59s floors to 59");
    assert_eq!(humanize(&n.checked_add(secs(600)).unwrap(), &n), "in 10 minutes", "future uses 'in'");
}

#[test]
fn part7_hours_including_one_hour_singular() {
    let n = now();
    assert_eq!(humanize(&n.checked_add(secs(3600)).unwrap(), &n), "in 1 hour");
    assert_eq!(humanize(&n.checked_sub(secs(3 * 3600)).unwrap(), &n), "3 hours ago");
}

#[test]
fn part7_days_under_thirty() {
    let n = now();
    assert_eq!(humanize(&n.checked_sub(secs(24 * 3600)).unwrap(), &n), "1 day ago");
    assert_eq!(humanize(&n.checked_add(secs(48 * 3600)).unwrap(), &n), "in 2 days");
}

#[test]
fn part7_thirty_days_or_more_prints_date_in_the_when_zone() {
    // 2026-10-11 00:30 Berlin (CEST) is 2026-10-10 22:30 UTC: more than 30 days
    // before `now`. The printed date must be the Berlin date, not the UTC one.
    let when = berlin(2026, 10, 11, 0, 30);
    assert_eq!(humanize(&when, &now()), "on 2026-10-11");
}

#[test]
fn part7_elapsed_is_absolute_not_wall_clock_across_dst() {
    // 28 March 12:00 CET to 29 March 12:00 CEST is 23 elapsed hours.
    let when = berlin(2026, 3, 28, 12, 0);
    let now_dst = berlin(2026, 3, 29, 12, 0);
    assert_eq!(humanize(&when, &now_dst), "23 hours ago");
}

#[test]
fn part7_comparison_uses_instants_not_zone_labels() {
    let n = now();
    let same_instant_utc = n.with_time_zone(TimeZone::UTC);
    assert_eq!(humanize(&same_instant_utc, &n), "just now");
}

#[test]
fn part7_humanize_with_fixed_clock_uses_injected_now() {
    let n = now();
    let clock = FixedClock::new(n.timestamp());
    let when = n.checked_sub(SignedDuration::from_hours(2)).unwrap();
    assert_eq!(humanize_with_clock(&when, &clock), "2 hours ago");
}

#[test]
fn part7_system_clock_reads_current_wall_time() {
    let before = Timestamp::now();
    let read = SystemClock.now();
    let after = Timestamp::now();
    assert!(before <= read && read <= after, "SystemClock must read the real wall clock");
}
