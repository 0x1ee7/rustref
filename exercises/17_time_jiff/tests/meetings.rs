//! Part 5: recurring meetings with an explicit DST policy.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::meetings::{Gap, Overlap, Recurrence};
use jiff::{Timestamp, civil::{self, Weekday}};

fn ts(s: &str) -> Timestamp {
    s.parse().expect("valid RFC 3339 fixture")
}

fn monday_0900(start: civil::Date) -> Recurrence {
    Recurrence::new(start, Weekday::Monday, civil::time(9, 0, 0, 0), "Europe/Berlin")
        .expect("known zone")
}

#[test]
fn part5_monday_0900_keeps_wall_clock_across_spring_dst() {
    let occ = monday_0900(civil::date(2026, 3, 16)).occurrences(4).unwrap();
    assert_eq!(occ.len(), 4);
    for z in &occ {
        assert_eq!(z.time(), civil::time(9, 0, 0, 0), "every occurrence is at 09:00 local time");
    }
    let offsets: Vec<i32> = occ.iter().map(|z| z.offset().seconds()).collect();
    assert_eq!(offsets, [3600, 3600, 7200, 7200], "CET before 29 March, CEST after");
}

#[test]
fn part5_first_occurrence_is_on_or_after_start() {
    let on_wed = monday_0900(civil::date(2026, 3, 25)).occurrences(1).unwrap();
    assert_eq!(on_wed[0].date(), civil::date(2026, 3, 30), "Wed start: first Monday is 30 March");
    let on_mon = monday_0900(civil::date(2026, 3, 30)).occurrences(1).unwrap();
    assert_eq!(on_mon[0].date(), civil::date(2026, 3, 30), "a Monday start counts itself");
}

#[test]
fn part5_zero_count_is_empty_and_instants_are_utc_correct() {
    assert!(monday_0900(civil::date(2026, 3, 16)).occurrences(0).unwrap().is_empty());
    let occ = monday_0900(civil::date(2026, 3, 30)).occurrences(1).unwrap();
    assert_eq!(occ[0].timestamp(), ts("2026-03-30T07:00:00Z"), "09:00 CEST is 07:00 UTC");
}

#[test]
fn part5_sunday_0230_gap_shifts_forward_by_default() {
    let rec = Recurrence::new(
        civil::date(2026, 3, 22),
        Weekday::Sunday,
        civil::time(2, 30, 0, 0),
        "Europe/Berlin",
    )
    .unwrap();
    let occ = rec.occurrences(2).unwrap();
    assert_eq!(occ[0].timestamp(), ts("2026-03-22T01:30:00Z"), "03-22 02:30 is CET");
    assert_eq!(occ[1].timestamp(), ts("2026-03-29T01:30:00Z"), "03-29 02:30 shifts to 03:30 CEST");
}

#[test]
fn part5_gap_skip_drops_occurrence_without_counting_it() {
    let rec = Recurrence::new(
        civil::date(2026, 3, 22),
        Weekday::Sunday,
        civil::time(2, 30, 0, 0),
        "Europe/Berlin",
    )
    .unwrap()
    .gap(Gap::Skip);
    let dates: Vec<civil::Date> = rec.occurrences(2).unwrap().iter().map(|z| z.date()).collect();
    assert_eq!(
        dates,
        [civil::date(2026, 3, 22), civil::date(2026, 4, 5)],
        "the 29 March occurrence is dropped and the count is refilled"
    );
}

#[test]
fn part5_overlap_default_picks_earlier_and_later_picks_cet() {
    let sunday_0230 = |overlap: Option<Overlap>| {
        let rec = Recurrence::new(
            civil::date(2026, 10, 18),
            Weekday::Sunday,
            civil::time(2, 30, 0, 0),
            "Europe/Berlin",
        )
        .unwrap();
        match overlap {
            Some(o) => rec.overlap(o),
            None => rec,
        }
        .occurrences(2)
        .unwrap()
    };
    let default = sunday_0230(None);
    assert_eq!(default[1].timestamp(), ts("2026-10-25T00:30:00Z"), "default Earlier: 02:30 CEST");
    let later = sunday_0230(Some(Overlap::Later));
    assert_eq!(later[1].timestamp(), ts("2026-10-25T01:30:00Z"), "Later: 02:30 CET");
}

#[test]
fn part5_overlap_skip_drops_second_occurrence() {
    let rec = Recurrence::new(
        civil::date(2026, 10, 18),
        Weekday::Sunday,
        civil::time(2, 30, 0, 0),
        "Europe/Berlin",
    )
    .unwrap()
    .overlap(Overlap::Skip);
    let dates: Vec<civil::Date> = rec.occurrences(2).unwrap().iter().map(|z| z.date()).collect();
    assert_eq!(dates, [civil::date(2026, 10, 18), civil::date(2026, 11, 1)]);
}

#[test]
fn part5_unknown_zone_is_reported() {
    let err = Recurrence::new(
        civil::date(2026, 3, 16),
        Weekday::Monday,
        civil::time(9, 0, 0, 0),
        "Nowhere/City",
    )
    .unwrap_err();
    assert!(matches!(err, TimeError::UnknownZone(_)), "got {err:?}");
}
