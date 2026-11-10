//! Part 1 — the iterator protocol and container traits.

use std::iter::FusedIterator;
use std::ops::Range;

/// One-shot countdown yielding `start, start-1, ..., 1`.
///
/// Double-ended: `rev()` counts up `1, 2, ..., start`. Exact-size and fused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Countdown {
    /// Remaining values as a half-open range; `u64` avoids overflow at `u32::MAX`.
    remaining: Range<u64>,
}

impl Countdown {
    /// Countdown from `start` down to 1. `Countdown::new(0)` is empty.
    pub fn new(start: u32) -> Self {
        Self {
            remaining: 1..u64::from(start) + 1,
        }
    }
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.remaining.next_back().map(|v| v as u32)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.remaining.size_hint()
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.remaining.next().map(|v| v as u32)
    }
}

impl ExactSizeIterator for Countdown {}

impl FusedIterator for Countdown {}

/// Re-iterable snapshot of records.
///
/// `new` takes the records by value (any `IntoIterator`, so generators work) and
/// stores them, so later mutation of the source cannot leak in. Every borrowing
/// iteration is independent; the owned form consumes the dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset<T> {
    records: Vec<T>,
}

impl<T> Dataset<T> {
    /// Snapshot the records produced by `records`.
    pub fn new(records: impl IntoIterator<Item = T>) -> Self {
        Self {
            records: records.into_iter().collect(),
        }
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// `true` when there are no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Borrowing iterator over the records.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.records.iter()
    }

    /// Mutable borrowing iterator over the records.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.records.iter_mut()
    }
}

impl<T> FromIterator<T> for Dataset<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter)
    }
}

impl<T> Extend<T> for Dataset<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.records.extend(iter);
    }
}

impl<T> IntoIterator for Dataset<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Dataset<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Dataset<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
