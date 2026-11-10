//! Part 6: booking slots across time zones.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::slots::{TimeSlot, merge_slots, total_booked};
use jiff::{SignedDuration, Timestamp, Zoned, civil, tz::TimeZone};

fn utc(s: &str) -> Zoned {
    s.parse::<Timestamp>().expect("valid RFC 3339").to_zoned(TimeZone::UTC)
}

fn slot(start: &str, end: &str) -> TimeSlot {
    TimeSlot::new(utc(start), utc(end)).expect("valid slot fixture")
}

#[test]
fn part6_new_rejects_inverted_and_empty_slots() {
    let inverted = TimeSlot::new(utc("2026-01-15T10:00:00Z"), utc("2026-01-15T09:00:00Z"));
    assert!(matches!(inverted, Err(TimeError::InvalidInput(_))), "end before start must be rejected");
    let empty = TimeSlot::new(utc("2026-01-15T10:00:00Z"), utc("2026-01-15T10:00:00Z"));
    assert!(matches!(empty, Err(TimeError::InvalidInput(_))), "zero-length slot must be rejected");
}

#[test]
fn part6_overlap_on_shared_instant_but_not_when_touching() {
    let a = slot("2026-01-15T09:00:00Z", "2026-01-15T10:00:00Z");
    let b = slot("2026-01-15T09:30:00Z", "2026-01-15T11:00:00Z");
    let c = slot("2026-01-15T10:00:00Z", "2026-01-15T11:00:00Z");
    assert!(a.overlaps(&b), "partial overlap");
    assert!(!a.overlaps(&c), "touching slots (a.end == c.start) do not overlap");
    assert!(c.overlaps(&c.clone()), "a slot overlaps itself");
}

#[test]
fn part6_overlap_compares_instants_across_zones() {
    let berlin = TimeSlot::new(
        civil::date(2026, 1, 15).at(10, 0, 0, 0).in_tz("Europe/Berlin").unwrap(), // 09:00Z
        civil::date(2026, 1, 15).at(11, 0, 0, 0).in_tz("Europe/Berlin").unwrap(), // 10:00Z
    )
    .unwrap();
    let new_york = TimeSlot::new(
        civil::date(2026, 1, 15).at(4, 30, 0, 0).in_tz("America/New_York").unwrap(), // 09:30Z
        civil::date(2026, 1, 15).at(5, 30, 0, 0).in_tz("America/New_York").unwrap(), // 10:30Z
    )
    .unwrap();
    assert!(berlin.overlaps(&new_york), "09:30Z–10:00Z is shared");
}

#[test]
fn part6_merge_sorts_combines_overlaps_and_absorbs_containment() {
    let merged = merge_slots([
        slot("2026-01-15T12:00:00Z", "2026-01-15T13:00:00Z"),
        slot("2026-01-15T09:30:00Z", "2026-01-15T10:30:00Z"),
        slot("2026-01-15T09:00:00Z", "2026-01-15T10:00:00Z"),
        slot("2026-01-15T10:00:00Z", "2026-01-15T11:00:00Z"), // touches the previous
        slot("2026-01-15T09:15:00Z", "2026-01-15T09:20:00Z"), // contained
    ]);
    assert_eq!(merged.len(), 2, "09:00–11:00 and 12:00–13:00");
    assert_eq!(merged[0].start().timestamp(), "2026-01-15T09:00:00Z".parse::<Timestamp>().unwrap());
    assert_eq!(merged[0].end().timestamp(), "2026-01-15T11:00:00Z".parse::<Timestamp>().unwrap());
    assert_eq!(merged[1].start().timestamp(), "2026-01-15T12:00:00Z".parse::<Timestamp>().unwrap());
}

#[test]
fn part6_merge_results_are_normalised_to_utc() {
    let berlin = TimeSlot::new(
        civil::date(2026, 1, 15).at(10, 0, 0, 0).in_tz("Europe/Berlin").unwrap(),
        civil::date(2026, 1, 15).at(11, 0, 0, 0).in_tz("Europe/Berlin").unwrap(),
    )
    .unwrap();
    let merged = merge_slots([berlin]);
    assert_eq!(merged[0].start().offset().seconds(), 0, "merged bounds are UTC");
    assert_eq!(merged[0].start().hour(), 9, "10:00 CET is 09:00 UTC");
}

#[test]
fn part6_merge_of_nothing_is_empty() {
    assert!(merge_slots(std::iter::empty::<TimeSlot>()).is_empty());
}

#[test]
fn part6_total_booked_counts_overlap_once() {
    let total = total_booked([
        slot("2026-01-15T09:00:00Z", "2026-01-15T11:00:00Z"),
        slot("2026-01-15T10:00:00Z", "2026-01-15T12:00:00Z"),
    ]);
    assert_eq!(total, SignedDuration::from_hours(3), "9–12 is three hours, not four");
}

#[test]
fn part6_total_booked_uses_absolute_time_across_dst() {
    let night = TimeSlot::new(
        civil::date(2026, 3, 28).at(9, 0, 0, 0).in_tz("Europe/Berlin").unwrap(),
        civil::date(2026, 3, 29).at(9, 0, 0, 0).in_tz("Europe/Berlin").unwrap(),
    )
    .unwrap();
    assert_eq!(total_booked([night]), SignedDuration::from_hours(23), "a wall-clock day is 23 hours here");
}
