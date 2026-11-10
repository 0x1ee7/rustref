//! Part 6: booking slots across time zones.

use jiff::{SignedDuration, Timestamp, Zoned, tz::TimeZone};

use crate::TimeError;

/// A half-open booking interval `[start, end)`. Each bound keeps its own zone
/// for display; comparisons use absolute time.
///
/// Fields are private. The invariant `start < end` is checked by [`TimeSlot::new`].
#[derive(Debug, Clone)]
pub struct TimeSlot {
    start: Zoned,
    end: Zoned,
}

impl TimeSlot {
    /// Build a slot. `end` must be strictly after `start`.
    ///
    /// # Errors
    ///
    /// [`TimeError::InvalidInput`] when `end <= start`.
    pub fn new(start: Zoned, end: Zoned) -> Result<Self, TimeError> {
        if end.timestamp() <= start.timestamp() {
            return Err(TimeError::InvalidInput(format!(
                "slot end {end} must be after start {start}"
            )));
        }
        Ok(Self { start, end })
    }

    /// Inclusive start bound.
    pub fn start(&self) -> &Zoned {
        &self.start
    }

    /// Exclusive end bound.
    pub fn end(&self) -> &Zoned {
        &self.end
    }

    /// `true` when the two slots share at least one instant. Touching slots
    /// (`self.end == other.start`) do not overlap.
    pub fn overlaps(&self, other: &TimeSlot) -> bool {
        self.start.timestamp() < other.end.timestamp()
            && other.start.timestamp() < self.end.timestamp()
    }
}

/// Sort by start and combine overlapping or touching slots. Results are UTC.
pub fn merge_slots(slots: impl IntoIterator<Item = TimeSlot>) -> Vec<TimeSlot> {
    let mut bounds: Vec<(Timestamp, Timestamp)> = slots
        .into_iter()
        .map(|slot| (slot.start.timestamp(), slot.end.timestamp()))
        .collect();
    bounds.sort_unstable_by_key(|&(start, _)| start);

    let mut merged: Vec<(Timestamp, Timestamp)> = Vec::with_capacity(bounds.len());
    for (start, end) in bounds {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    merged
        .into_iter()
        .map(|(start, end)| TimeSlot {
            start: start.to_zoned(TimeZone::UTC),
            end: end.to_zoned(TimeZone::UTC),
        })
        .collect()
}

/// Total covered time after merging; overlapped time counts once.
pub fn total_booked(slots: impl IntoIterator<Item = TimeSlot>) -> SignedDuration {
    merge_slots(slots).iter().fold(SignedDuration::ZERO, |total, slot| {
        total + slot.end.timestamp().duration_since(slot.start.timestamp())
    })
}
