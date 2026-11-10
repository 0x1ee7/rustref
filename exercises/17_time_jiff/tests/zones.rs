//! Part 1: instants, zones, and explicit DST disambiguation.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::zones::{format_in_zone, in_zone, localize, to_utc};
use jiff::{Timestamp, civil, tz::Disambiguation};

fn ts(s: &str) -> Timestamp {
    s.parse().expect("valid RFC 3339 fixture")
}

fn spring_gap() -> civil::DateTime {
    civil::date(2026, 3, 29).at(2, 30, 0, 0)
}

fn fall_overlap() -> civil::DateTime {
    civil::date(2026, 10, 25).at(2, 30, 0, 0)
}

#[test]
fn part1_to_utc_keeps_instant_and_zero_offset() {
    let berlin = localize(
        civil::date(2026, 1, 15).at(9, 0, 0, 0),
        "Europe/Berlin",
        Disambiguation::Reject,
    )
    .expect("unambiguous wall time");
    let utc = to_utc(&berlin);
    assert_eq!(utc.timestamp(), berlin.timestamp(), "to_utc must keep the instant");
    assert_eq!(utc.offset().seconds(), 0, "UTC offset must be zero");
}

#[test]
fn part1_localize_unambiguous_winter_wall_time_is_cet() {
    let z = localize(
        civil::date(2026, 1, 15).at(9, 0, 0, 0),
        "Europe/Berlin",
        Disambiguation::Reject,
    )
    .expect("unambiguous wall time");
    assert_eq!(z.timestamp(), ts("2026-01-15T08:00:00Z"), "09:00 CET is 08:00 UTC");
}

#[test]
fn part1_gap_compatible_shifts_forward_to_post_transition_offset() {
    let z = localize(spring_gap(), "Europe/Berlin", Disambiguation::Compatible)
        .expect("compatible resolves a gap");
    assert_eq!(z.timestamp(), ts("2026-03-29T01:30:00Z"), "02:30 in the gap becomes 03:30 CEST");
    assert_eq!(z.offset().seconds(), 7200, "result must carry the CEST offset");
}

#[test]
fn part1_gap_earlier_later_and_reject_report_nonexistent() {
    for policy in [Disambiguation::Earlier, Disambiguation::Later, Disambiguation::Reject] {
        let err = localize(spring_gap(), "Europe/Berlin", policy).unwrap_err();
        assert!(
            matches!(err, TimeError::Nonexistent(_)),
            "{policy:?} on a DST gap must be Nonexistent, got {err:?}"
        );
    }
}

#[test]
fn part1_overlap_earlier_later_and_compatible_pick_occurrences() {
    let earlier = localize(fall_overlap(), "Europe/Berlin", Disambiguation::Earlier).unwrap();
    let later = localize(fall_overlap(), "Europe/Berlin", Disambiguation::Later).unwrap();
    let compat = localize(fall_overlap(), "Europe/Berlin", Disambiguation::Compatible).unwrap();
    assert_eq!(earlier.timestamp(), ts("2026-10-25T00:30:00Z"), "Earlier is the CEST occurrence");
    assert_eq!(later.timestamp(), ts("2026-10-25T01:30:00Z"), "Later is the CET occurrence");
    assert_eq!(compat.timestamp(), earlier.timestamp(), "Compatible picks the earlier occurrence");
}

#[test]
fn part1_overlap_reject_reports_ambiguous() {
    let err = localize(fall_overlap(), "Europe/Berlin", Disambiguation::Reject).unwrap_err();
    assert!(
        matches!(err, TimeError::Ambiguous(_)),
        "Reject on a DST overlap must be Ambiguous, got {err:?}"
    );
}

#[test]
fn part1_unknown_zone_is_reported_by_name() {
    let err = localize(
        civil::date(2026, 1, 15).at(9, 0, 0, 0),
        "Mars/Olympus",
        Disambiguation::Reject,
    )
    .unwrap_err();
    assert!(
        matches!(&err, TimeError::UnknownZone(name) if name == "Mars/Olympus"),
        "expected UnknownZone(\"Mars/Olympus\"), got {err:?}"
    );
}

#[test]
fn part1_in_zone_renders_instant_as_berlin_wall_time() {
    let z = in_zone(ts("2026-03-29T12:00:00Z"), "Europe/Berlin").expect("known zone");
    assert_eq!(z.hour(), 14, "12:00 UTC after the spring transition is 14:00 CEST");
    assert_eq!(z.offset().seconds(), 7200, "offset must be CEST");
    assert_eq!(z.timestamp(), ts("2026-03-29T12:00:00Z"), "instant must be unchanged");
}

#[test]
fn part1_format_in_zone_summer_uses_cest() {
    let s = format_in_zone(ts("2026-03-29T12:00:00Z"), "Europe/Berlin").expect("known zone");
    assert_eq!(s, "2026-03-29 14:00 CEST");
}

#[test]
fn part1_format_in_zone_winter_uses_cet_and_unknown_zone_errors() {
    let s = format_in_zone(ts("2026-01-15T08:00:00Z"), "Europe/Berlin").expect("known zone");
    assert_eq!(s, "2026-01-15 09:00 CET");
    let err = format_in_zone(ts("2026-01-15T08:00:00Z"), "Mars/Olympus").unwrap_err();
    assert!(matches!(err, TimeError::UnknownZone(_)), "unknown zone must error, got {err:?}");
}
