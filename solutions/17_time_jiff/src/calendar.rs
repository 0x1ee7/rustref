//! Part 4: month arithmetic, billing schedules, and business-day calendars.

use std::collections::BTreeSet;

use jiff::{Span, civil::{self, Weekday}};

use crate::TimeError;

const ALL_WEEKDAYS: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
];

/// Days in `month` of `year` (proleptic Gregorian, leap-year aware).
///
/// # Errors
///
/// [`TimeError::InvalidInput`] when `month` is not in `1..=12`.
pub fn days_in_month(year: i16, month: i8) -> Result<i8, TimeError> {
    if !(1..=12).contains(&month) {
        return Err(TimeError::InvalidInput(format!("month {month} is not in 1..=12")));
    }
    Ok(civil::Date::new(year, month, 1)?.days_in_month())
}

/// Add `months` calendar months, clamping the day to the target month's length.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_months(date: civil::Date, months: i32) -> Result<civil::Date, TimeError> {
    Ok(date.checked_add(Span::new().months(i64::from(months)))?)
}

/// The first `count` billing dates, each computed from `anchor`.
///
/// Stepping from the previous date would drift (31 Jan, 28 Feb, 28 Mar).
/// Computing each date from the anchor keeps the 31st where the month allows it.
///
/// # Errors
///
/// [`TimeError::Jiff`] when a date leaves jiff's supported range.
pub fn billing_dates(anchor: civil::Date, count: usize) -> Result<Vec<civil::Date>, TimeError> {
    (0..count)
        .map(|i| {
            let months = i32::try_from(i).map_err(|e| TimeError::InvalidInput(e.to_string()))?;
            add_months(anchor, months)
        })
        .collect()
}

/// Business-day arithmetic over a weekend set and a holiday set.
///
/// Dates are plain [`civil::Date`]s, so there are no zones and no DST.
#[derive(Debug, Clone)]
pub struct BusinessCalendar {
    holidays: BTreeSet<civil::Date>,
    weekend: Vec<Weekday>,
}

impl BusinessCalendar {
    /// Calendar with Saturday and Sunday as the weekend.
    pub fn new(holidays: impl IntoIterator<Item = civil::Date>) -> Self {
        Self {
            holidays: holidays.into_iter().collect(),
            weekend: vec![Weekday::Saturday, Weekday::Sunday],
        }
    }

    /// Replace the weekend set.
    #[must_use]
    pub fn with_weekend(self, weekend: impl IntoIterator<Item = Weekday>) -> Self {
        Self { weekend: weekend.into_iter().collect(), ..self }
    }

    /// `true` when `date` is not a weekend day and not a holiday.
    pub fn is_business_day(&self, date: civil::Date) -> bool {
        !self.weekend.contains(&date.weekday()) && !self.holidays.contains(&date)
    }

    /// Step `n` business days from `date`. Forward for `n > 0`, backward for
    /// `n < 0`. `n == 0` returns `date` unchanged, even when it is a weekend.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidInput`] when every weekday is weekend and `n != 0`,
    /// since no business day exists to step to. [`TimeError::Jiff`] when the
    /// result is out of range.
    pub fn add_business_days(&self, date: civil::Date, n: i64) -> Result<civil::Date, TimeError> {
        if n == 0 {
            return Ok(date);
        }
        if ALL_WEEKDAYS.iter().all(|day| self.weekend.contains(day)) {
            return Err(TimeError::InvalidInput(
                "every weekday is weekend, so no business day exists".to_owned(),
            ));
        }
        let step = Span::new().days(n.signum());
        let mut remaining = n.unsigned_abs();
        let mut current = date;
        while remaining > 0 {
            current = current.checked_add(step)?;
            if self.is_business_day(current) {
                remaining -= 1;
            }
        }
        Ok(current)
    }

    /// Business days in `(start, end]`; negated when `end < start`.
    pub fn business_days_between(&self, start: civil::Date, end: civil::Date) -> i64 {
        let (from, to, sign) = if end >= start {
            (start, end, 1_i64)
        } else {
            (end, start, -1_i64)
        };
        let mut count: i64 = 0;
        let mut day = from;
        while day < to {
            day = day.tomorrow().expect("day < to, so the next day is in range");
            if self.is_business_day(day) {
                count += 1;
            }
        }
        sign * count
    }
}
