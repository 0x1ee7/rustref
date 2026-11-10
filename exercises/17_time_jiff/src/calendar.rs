//! Part 4: month arithmetic, billing schedules, and business-day calendars.

use jiff::civil::{self, Weekday};

use crate::TimeError;

/// Days in `month` of `year` (proleptic Gregorian, leap-year aware).
///
/// # Errors
///
/// [`TimeError::InvalidInput`] when `month` is not in `1..=12`.
pub fn days_in_month(_year: i16, _month: i8) -> Result<i8, TimeError> {
    todo!()
}

/// Add calendar months, clamping the day to the target month's length.
///
/// `2026-01-31 + 1 month = 2026-02-28`. Negative `months` are allowed.
///
/// # Errors
///
/// [`TimeError::Jiff`] when the result leaves jiff's supported range.
pub fn add_months(_date: civil::Date, _months: i32) -> Result<civil::Date, TimeError> {
    todo!()
}

/// `count` billing dates, each computed from `anchor` (no drift).
///
/// # Errors
///
/// [`TimeError::Jiff`] when a date leaves jiff's supported range.
pub fn billing_dates(_anchor: civil::Date, _count: usize) -> Result<Vec<civil::Date>, TimeError> {
    todo!()
}

/// Business-day rules: a weekend set plus a holiday set.
///
/// Fields are private; add whatever state the implementation needs.
pub struct BusinessCalendar {}

impl BusinessCalendar {
    /// Calendar with Saturday and Sunday as the weekend.
    pub fn new(_holidays: impl IntoIterator<Item = civil::Date>) -> Self {
        todo!()
    }

    /// Replace the weekend set.
    #[must_use]
    pub fn with_weekend(self, _weekend: impl IntoIterator<Item = Weekday>) -> Self {
        todo!()
    }

    /// `true` when `date` is not a weekend day and not a holiday.
    pub fn is_business_day(&self, _date: civil::Date) -> bool {
        todo!()
    }

    /// Step `n` business days from `date` (backward if negative).
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidInput`] when `n != 0` and no weekday is a business
    /// day; [`TimeError::Jiff`] when the result is out of range.
    pub fn add_business_days(&self, _date: civil::Date, _n: i64) -> Result<civil::Date, TimeError> {
        todo!()
    }

    /// Business days in `(start, end]`; negated when `end < start`.
    pub fn business_days_between(&self, _start: civil::Date, _end: civil::Date) -> i64 {
        todo!()
    }
}
