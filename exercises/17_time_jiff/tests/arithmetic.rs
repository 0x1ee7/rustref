//! Part 3: wall-clock (calendar) vs elapsed (absolute) arithmetic.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::arithmetic::{add_calendar_days, add_elapsed, elapsed_between};
use jiff::{SignedDuration, Timestamp, Zoned, civil};

fn berlin_wall(y: i16, m: i8, d: i8, h: i8, min: i8) -> Zoned {
    civil::date(y, m, d)
        .at(h, min, 0, 0)
        .in_tz("Europe/Berlin")
        .expect("known zone and unambiguous fixture")
}

#[test]
fn part3_calendar_day_keeps_wall_clock_across_spring_forward() {
    let start = berlin_wall(2026, 3, 28, 9, 0); // CET, +01:00
    let next = add_calendar_days(&start, 1).expect("in range");
    assert_eq!(next.datetime(), civil::date(2026, 3, 29).at(9, 0, 0, 0), "wall clock kept");
    assert_eq!(next.offset().seconds(), 7200, "offset must switch to CEST");
    assert_eq!(
        elapsed_between(&start, &next),
        SignedDuration::from_hours(23),
        "the calendar day is 23 elapsed hours"
    );
}

#[test]
fn part3_elapsed_24h_across_spring_forward_lands_on_10_00() {
    let start = berlin_wall(2026, 3, 28, 9, 0);
    let later = add_elapsed(&start, SignedDuration::from_hours(24)).expect("in range");
    assert_eq!(later.datetime(), civil::date(2026, 3, 29).at(10, 0, 0, 0), "24 exact hours later");
    assert_eq!(later.time_zone().iana_name(), Some("Europe/Berlin"), "zone must be kept");
}

#[test]
fn part3_gap_on_calendar_add_shifts_forward() {
    let start = berlin_wall(2026, 3, 28, 2, 30); // CET; 02:30 on the 29th is in the gap
    let next = add_calendar_days(&start, 1).expect("in range");
    assert_eq!(next.timestamp(), "2026-03-29T01:30:00Z".parse::<Timestamp>().unwrap(),
        "02:30 in the gap shifts to 03:30 CEST");
}

#[test]
fn part3_overlap_on_calendar_add_resolves_to_earlier_occurrence() {
    let start = civil::date(2026, 10, 24)
        .at(2, 30, 0, 0)
        .to_zoned(jiff::tz::TimeZone::get("Europe/Berlin").unwrap())
        .unwrap();
    let next = add_calendar_days(&start, 1).expect("in range");
    assert_eq!(next.offset().seconds(), 7200, "fold-0 semantics: the earlier (CEST) occurrence");
    assert_eq!(next.datetime(), civil::date(2026, 10, 25).at(2, 30, 0, 0));
}

#[test]
fn part3_elapsed_add_is_signed_and_keeps_zone() {
    let start = berlin_wall(2026, 1, 15, 9, 0);
    let earlier = add_elapsed(&start, -SignedDuration::from_hours(1)).expect("in range");
    assert_eq!(earlier.time_zone().iana_name(), Some("Europe/Berlin"));
    assert_eq!(elapsed_between(&earlier, &start), SignedDuration::from_hours(1));
    assert_eq!(elapsed_between(&start, &earlier), SignedDuration::from_hours(-1));
}

#[test]
fn part3_elapsed_between_compares_instants_across_zones() {
    let berlin = berlin_wall(2026, 1, 15, 9, 0);
    let utc = berlin.with_time_zone(jiff::tz::TimeZone::UTC);
    assert_eq!(
        elapsed_between(&berlin, &utc),
        SignedDuration::ZERO,
        "the same instant in two zones is zero elapsed time"
    );
}

#[test]
fn part3_add_elapsed_overflow_is_a_jiff_error() {
    let start = berlin_wall(2026, 1, 15, 9, 0);
    let err = add_elapsed(&start, SignedDuration::MAX).unwrap_err();
    assert!(matches!(err, TimeError::Jiff(_)), "range overflow must be Jiff, got {err:?}");
}
