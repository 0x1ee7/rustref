//! Part 4: month arithmetic, billing schedules, and business-day calendars.

use ex17_time_jiff::TimeError;
use ex17_time_jiff::calendar::{BusinessCalendar, add_months, billing_dates, days_in_month};
use jiff::civil::{self, Weekday};

fn fri() -> civil::Date { civil::date(2026, 3, 27) }
fn sat() -> civil::Date { civil::date(2026, 3, 28) }
fn sun() -> civil::Date { civil::date(2026, 3, 29) }
fn mon() -> civil::Date { civil::date(2026, 3, 30) }
fn tue() -> civil::Date { civil::date(2026, 3, 31) }

#[test]
fn part4_days_in_month_is_leap_year_aware() {
    assert_eq!(days_in_month(2024, 2).unwrap(), 29, "2024 is a leap year");
    assert_eq!(days_in_month(2026, 2).unwrap(), 28, "2026 is not");
    assert_eq!(days_in_month(2000, 2).unwrap(), 29, "2000 is divisible by 400");
    assert_eq!(days_in_month(1900, 2).unwrap(), 28, "1900 is not (divisible by 100)");
    assert_eq!(days_in_month(2026, 4).unwrap(), 30, "April has 30 days");
}

#[test]
fn part4_days_in_month_rejects_month_out_of_range() {
    for month in [0_i8, 13] {
        let err = days_in_month(2026, month).unwrap_err();
        assert!(matches!(err, TimeError::InvalidInput(_)), "month {month} must be InvalidInput, got {err:?}");
    }
}

#[test]
fn part4_add_months_clamps_to_end_of_shorter_month() {
    assert_eq!(add_months(civil::date(2026, 1, 31), 1).unwrap(), civil::date(2026, 2, 28));
    assert_eq!(add_months(civil::date(2028, 1, 31), 1).unwrap(), civil::date(2028, 2, 29), "leap day");
}

#[test]
fn part4_add_months_negative_crosses_year_and_clamps() {
    assert_eq!(add_months(civil::date(2026, 1, 15), -2).unwrap(), civil::date(2025, 11, 15));
    assert_eq!(add_months(civil::date(2026, 3, 31), -1).unwrap(), civil::date(2026, 2, 28));
    assert_eq!(add_months(civil::date(2026, 5, 9), 0).unwrap(), civil::date(2026, 5, 9), "zero is identity");
}

#[test]
fn part4_billing_dates_are_computed_from_anchor_without_drift() {
    let got = billing_dates(civil::date(2026, 1, 31), 4).unwrap();
    assert_eq!(
        got,
        vec![
            civil::date(2026, 1, 31),
            civil::date(2026, 2, 28),
            civil::date(2026, 3, 31),
            civil::date(2026, 4, 30),
        ],
        "March must return to the 31st, not stay at the 28th"
    );
}

#[test]
fn part4_billing_dates_zero_count_is_empty() {
    assert!(billing_dates(civil::date(2026, 1, 31), 0).unwrap().is_empty());
}

#[test]
fn part4_default_weekend_is_saturday_and_sunday() {
    let cal = BusinessCalendar::new([]);
    assert!(cal.is_business_day(fri()), "Friday is a business day");
    assert!(!cal.is_business_day(sat()), "Saturday is weekend");
    assert!(!cal.is_business_day(sun()), "Sunday is weekend");
    assert!(cal.is_business_day(mon()), "Monday is a business day");
}

#[test]
fn part4_holiday_is_not_a_business_day() {
    let cal = BusinessCalendar::new([mon(), mon()]); // duplicates are harmless
    assert!(!cal.is_business_day(mon()), "a holiday on a weekday is not a business day");
}

#[test]
fn part4_add_business_days_skips_weekend_and_holiday_forward() {
    let cal = BusinessCalendar::new([mon()]);
    assert_eq!(cal.add_business_days(fri(), 1).unwrap(), tue(), "Mon is a holiday, so Fri+1 is Tue");
    assert_eq!(cal.add_business_days(sat(), 1).unwrap(), tue(), "a weekend start steps to the next business day");
}

#[test]
fn part4_add_business_days_backward_and_zero_step() {
    let cal = BusinessCalendar::new([]);
    assert_eq!(cal.add_business_days(sat(), -1).unwrap(), fri(), "backward from a weekend lands on Friday");
    assert_eq!(cal.add_business_days(sat(), 0).unwrap(), sat(), "zero steps returns the input, even on a weekend");
}

#[test]
fn part4_business_days_between_is_half_open_and_signed() {
    let cal = BusinessCalendar::new([]);
    assert_eq!(cal.business_days_between(fri(), mon()), 1, "(Fri, Mon] contains only Mon");
    assert_eq!(cal.business_days_between(mon(), fri()), -1, "reversed range is negated");
    assert_eq!(cal.business_days_between(fri(), fri()), 0, "empty interval");
}

#[test]
fn part4_business_days_between_excludes_holidays() {
    let cal = BusinessCalendar::new([mon()]);
    assert_eq!(cal.business_days_between(fri(), tue()), 1, "Mon is a holiday; only Tue counts");
}

#[test]
fn part4_custom_weekend_friday_and_saturday() {
    let cal = BusinessCalendar::new([]).with_weekend([Weekday::Friday, Weekday::Saturday]);
    assert!(!cal.is_business_day(fri()), "Friday is now weekend");
    assert!(cal.is_business_day(sun()), "Sunday is now a business day");
}

#[test]
fn part4_all_days_weekend_is_an_error_not_an_infinite_loop() {
    let all = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];
    let cal = BusinessCalendar::new([]).with_weekend(all);
    let err = cal.add_business_days(fri(), 1).unwrap_err();
    assert!(matches!(err, TimeError::InvalidInput(_)), "no business days must be InvalidInput, got {err:?}");
    assert_eq!(cal.add_business_days(fri(), 0).unwrap(), fri(), "zero steps still succeeds");
}
