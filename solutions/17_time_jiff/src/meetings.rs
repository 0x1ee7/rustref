//! Part 5: recurring meetings with an explicit DST policy.

use jiff::{Zoned, civil::{self, Weekday}, tz::{AmbiguousOffset, TimeZone}};

use crate::TimeError;
use crate::zones::zone;

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

/// A weekly meeting at a fixed wall time in a zone.
///
/// Fields are private. The DST policies default to [`Gap::ShiftForward`] and
/// [`Overlap::Earlier`].
#[derive(Debug, Clone)]
pub struct Recurrence {
    start: civil::Date,
    weekday: Weekday,
    at: civil::Time,
    zone: TimeZone,
    gap: Gap,
    overlap: Overlap,
}

impl Recurrence {
    /// Weekly at `at` in `zone`, first occurrence on or after `start`.
    /// Defaults: [`Gap::ShiftForward`] and [`Overlap::Earlier`].
    ///
    /// # Errors
    ///
    /// [`TimeError::UnknownZone`] for an unknown IANA name.
    pub fn new(
        start: civil::Date,
        weekday: Weekday,
        at: civil::Time,
        zone_name: &str,
    ) -> Result<Self, TimeError> {
        Ok(Self {
            start,
            weekday,
            at,
            zone: zone(zone_name)?,
            gap: Gap::ShiftForward,
            overlap: Overlap::Earlier,
        })
    }

    /// Set the policy for wall times that do not exist.
    #[must_use]
    pub fn gap(self, gap: Gap) -> Self {
        Self { gap, ..self }
    }

    /// Set the policy for wall times that occur twice.
    #[must_use]
    pub fn overlap(self, overlap: Overlap) -> Self {
        Self { overlap, ..self }
    }

    /// The next `count` occurrences as zoned datetimes, in order.
    ///
    /// Skipped gap or overlap occurrences are not counted, so the search
    /// continues until `count` real occurrences exist.
    ///
    /// # Errors
    ///
    /// [`TimeError::Jiff`] when a computed date is out of range.
    pub fn occurrences(&self, count: usize) -> Result<Vec<Zoned>, TimeError> {
        let mut out = Vec::with_capacity(count);
        let mut date = self.start;
        while out.len() < count {
            if date.weekday() == self.weekday {
                let wall = civil::DateTime::from_parts(date, self.at);
                let amb = self.zone.to_ambiguous_zoned(wall);
                match amb.offset() {
                    AmbiguousOffset::Gap { .. } => match self.gap {
                        Gap::Skip => {}
                        Gap::ShiftForward => out.push(amb.compatible()?),
                    },
                    AmbiguousOffset::Fold { .. } => match self.overlap {
                        Overlap::Earlier => out.push(amb.earlier()?),
                        Overlap::Later => out.push(amb.later()?),
                        Overlap::Skip => {}
                    },
                    _ => out.push(amb.unambiguous()?),
                }
            }
            date = date.tomorrow()?;
        }
        Ok(out)
    }
}
