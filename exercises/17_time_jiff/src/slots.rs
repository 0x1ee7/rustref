//! Part 6: booking slots across time zones.

use jiff::{SignedDuration, Zoned};

use crate::TimeError;

/// A half-open booking interval `[start, end)`. Each bound keeps its own zone
/// for display; comparisons use absolute time.
///
/// Fields are private; add whatever state the implementation needs.
#[derive(Debug, Clone)]
pub struct TimeSlot {
    _start: Zoned,
    _end: Zoned,
}

impl TimeSlot {
    /// Build a slot. `end` must be strictly after `start`.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidInput`] when `end <= start`.
    pub fn new(_start: Zoned, _end: Zoned) -> Result<Self, TimeError> {
        todo!()
    }

    /// Inclusive start bound.
    pub fn start(&self) -> &Zoned {
        todo!()
    }

    /// Exclusive end bound.
    pub fn end(&self) -> &Zoned {
        todo!()
    }

    /// `true` when the two slots share at least one instant. Touching slots
    /// (`self.end == other.start`) do not overlap.
    pub fn overlaps(&self, _other: &TimeSlot) -> bool {
        todo!()
    }
}

/// Sort by start and combine overlapping or touching slots. Results are UTC.
pub fn merge_slots(_slots: impl IntoIterator<Item = TimeSlot>) -> Vec<TimeSlot> {
    todo!()
}

/// Total covered time after merging; overlapped time counts once.
pub fn total_booked(_slots: impl IntoIterator<Item = TimeSlot>) -> SignedDuration {
    todo!()
}
