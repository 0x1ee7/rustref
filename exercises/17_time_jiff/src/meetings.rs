//! Part 5: recurring meetings with an explicit DST policy.

use jiff::{Zoned, civil::{self, Weekday}, tz::TimeZone};

use crate::TimeError;

/// Policy for a wall time that does not exist (spring-forward gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// Drop the occurrence. It does not count toward `count`.
    Skip,
    /// Use the instant the clock jumps to (`02:30` becomes `03:30 CEST`).
    ShiftForward,
}

/// Policy for a wall time that occurs twice (fall-back overlap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    /// Use the first occurrence (pre-transition offset).
    Earlier,
    /// Use the second occurrence (post-transition offset).
    Later,
    /// Drop the occurrence. It does not count toward `count`.
    Skip,
}

/// A weekly recurrence at a fixed wall time in one IANA zone.
///
/// Fields are private; add whatever state the implementation needs.
#[derive(Debug, Clone)]
pub struct Recurrence {
    _start: civil::Date,
    _weekday: Weekday,
    _at: civil::Time,
    _zone: TimeZone,
    _gap: Gap,
    _overlap: Overlap,
}

impl Recurrence {
    /// Weekly at `at` in `zone`, first occurrence on or after `start`.
    /// Defaults: [`Gap::ShiftForward`] and [`Overlap::Earlier`].
    ///
    /// # Errors
    ///
    /// [`TimeError::UnknownZone`] for an unknown IANA name.
    pub fn new(
        _start: civil::Date,
        _weekday: Weekday,
        _at: civil::Time,
        _zone: &str,
    ) -> Result<Self, TimeError> {
        todo!()
    }

    /// Set the policy for wall times in a gap.
    #[must_use]
    pub fn gap(self, _gap: Gap) -> Self {
        todo!()
    }

    /// Set the policy for wall times that occur twice.
    #[must_use]
    pub fn overlap(self, _overlap: Overlap) -> Self {
        todo!()
    }

    /// The next `count` occurrences, in ascending order. Dropped occurrences
    /// do not count toward `count`.
    ///
    /// # Errors
    ///
    /// [`TimeError::Jiff`] when a computed date is out of range.
    pub fn occurrences(&self, _count: usize) -> Result<Vec<Zoned>, TimeError> {
        todo!()
    }
}
