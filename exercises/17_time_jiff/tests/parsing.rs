//! Part 2: heterogeneous timestamp parsing and epoch conversion.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::parsing::{parse_timestamp, to_epoch_ms};
use jiff::Timestamp;

fn ts(s: &str) -> Timestamp {
    s.parse().expect("valid RFC 3339 fixture")
}

fn is_invalid(result: Result<Timestamp, TimeError>) -> bool {
    matches!(result, Err(TimeError::InvalidInput(_)))
}

#[test]
fn part2_rfc3339_z_suffix_is_utc() {
    let parsed = parse_timestamp("2023-11-14T22:13:20Z").expect("valid RFC 3339");
    assert_eq!(parsed, ts("2023-11-14T22:13:20Z"));
}

#[test]
fn part2_rfc3339_offset_normalises_to_the_same_instant() {
    let parsed = parse_timestamp("2026-03-29T12:00:00+02:00").expect("valid RFC 3339");
    assert_eq!(parsed, ts("2026-03-29T10:00:00Z"), "+02:00 wall time is 10:00 UTC");
}

#[test]
fn part2_naive_iso_and_date_only_are_invalid() {
    assert!(is_invalid(parse_timestamp("2026-03-29T12:00:00")), "naive ISO must be rejected");
    assert!(is_invalid(parse_timestamp("2026-03-29")), "date-only must be rejected");
}

#[test]
fn part2_blank_and_garbage_text_are_invalid() {
    assert!(is_invalid(parse_timestamp("")), "empty text must be rejected");
    assert!(is_invalid(parse_timestamp("   ")), "whitespace-only must be rejected");
    assert!(is_invalid(parse_timestamp("not a timestamp")), "garbage must be rejected");
}

#[test]
fn part2_surrounding_whitespace_is_ignored() {
    assert_eq!(
        parse_timestamp("  2023-11-14T22:13:20Z \n").expect("whitespace ignored"),
        ts("2023-11-14T22:13:20Z"),
    );
    assert_eq!(
        parse_timestamp("  1700000000  ").expect("whitespace ignored"),
        ts("2023-11-14T22:13:20Z"),
    );
}

#[test]
fn part2_integer_seconds_and_numeric_text_agree() {
    let expected = ts("2023-11-14T22:13:20Z");
    assert_eq!(parse_timestamp(1_700_000_000_i64).expect("seconds"), expected);
    assert_eq!(parse_timestamp("1700000000").expect("numeric text"), expected);
}

#[test]
fn part2_integer_milliseconds_detected_by_magnitude() {
    let parsed = parse_timestamp(1_700_000_000_123_i64).expect("milliseconds");
    assert_eq!(parsed, ts("2023-11-14T22:13:20.123Z"), "13-digit values are milliseconds");
}

#[test]
fn part2_magnitude_threshold_is_exactly_one_hundred_billion() {
    let seconds = parse_timestamp(99_999_999_999_i64).expect("below threshold is seconds");
    assert_eq!(seconds, Timestamp::from_second(99_999_999_999).unwrap());
    let millis = parse_timestamp(100_000_000_000_i64).expect("at threshold is milliseconds");
    assert_eq!(millis, Timestamp::from_second(100_000_000).unwrap());
}

#[test]
fn part2_float_epoch_keeps_fractional_seconds() {
    let parsed = parse_timestamp(1_700_000_000.5_f64).expect("float epoch");
    assert_eq!(parsed, ts("2023-11-14T22:13:20.5Z"), "fraction must survive");
}

#[test]
fn part2_non_finite_floats_are_invalid() {
    assert!(is_invalid(parse_timestamp(f64::NAN)), "NaN must be rejected");
    assert!(is_invalid(parse_timestamp(f64::INFINITY)), "+inf must be rejected");
    assert!(is_invalid(parse_timestamp(f64::NEG_INFINITY)), "-inf must be rejected");
}

#[test]
fn part2_negative_epoch_is_before_1970() {
    let parsed = parse_timestamp(-1_i64).expect("negative epoch is valid");
    assert_eq!(parsed, ts("1969-12-31T23:59:59Z"));
}

#[test]
fn part2_rfc2822_with_numeric_offset_parses() {
    let parsed = parse_timestamp("Sun, 29 Mar 2026 12:00:00 +0200").expect("valid RFC 2822");
    assert_eq!(parsed, ts("2026-03-29T10:00:00Z"));
}

#[test]
fn part2_rfc2822_minus_zero_offset_is_invalid() {
    // RFC 2822 §3.3: "-0000" means the local offset is unknown, so the
    // instant is not determined. jiff silently treats it as UTC; we must not.
    let result = parse_timestamp("Sun, 29 Mar 2026 12:00:00 -0000");
    assert!(is_invalid(result), "-0000 must be rejected, not read as UTC");
}

#[test]
fn part2_to_epoch_ms_floors_toward_the_past() {
    let positive = Timestamp::from_nanosecond(1_500_000).unwrap(); // +1.5 ms
    let negative = Timestamp::from_nanosecond(-1_500_000).unwrap(); // -1.5 ms
    assert_eq!(to_epoch_ms(positive), 1, "+1.5 ms floors to 1");
    assert_eq!(to_epoch_ms(negative), -2, "-1.5 ms floors to -2, not -1");
}

#[test]
fn part2_to_epoch_ms_round_trips_whole_milliseconds() {
    let ts = Timestamp::from_millisecond(1_700_000_000_123).unwrap();
    assert_eq!(to_epoch_ms(ts), 1_700_000_000_123);
}
