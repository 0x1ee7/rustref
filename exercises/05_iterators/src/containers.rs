//! Part 1 — the iterator protocol and container traits.

use std::iter::FusedIterator;

/// One-shot countdown yielding `start, start-1, ..., 1`.
///
/// Double-ended: `rev()` counts up `1, 2, ..., start`. Exact-size and fused.
/// Private state is yours to design (a half-open `[lo, hi)` over `u64` avoids
/// overflow at `u32::MAX`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Countdown {}

impl Countdown {
    /// Countdown from `start` down to 1. `Countdown::new(0)` is empty.
    pub fn new(_start: u32) -> Self {
        todo!()
    }
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        todo!()
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
    _records: Vec<T>,
}

impl<T> Dataset<T> {
    /// Snapshot `records` into a new dataset.
    pub fn new(_records: impl IntoIterator<Item = T>) -> Self {
        todo!()
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when the dataset holds no records.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Borrowing iterator; each call starts a fresh, independent pass.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        todo!()
    }

    /// Mutable borrowing iterator.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        todo!()
    }
}

impl<T> FromIterator<T> for Dataset<T> {
    fn from_iter<I: IntoIterator<Item = T>>(_iter: I) -> Self {
        todo!()
    }
}

impl<T> Extend<T> for Dataset<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, _iter: I) {
        todo!()
    }
}

impl<T> IntoIterator for Dataset<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

impl<'a, T> IntoIterator for &'a Dataset<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

impl<'a, T> IntoIterator for &'a mut Dataset<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}
