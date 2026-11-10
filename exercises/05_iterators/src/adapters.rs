//! Part 6 — adapters, extension traits and slice iterators.

use std::iter::FusedIterator;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

use crate::log::LogRecord;

/// Extension methods available on every iterator.
pub trait IterExt: Iterator + Sized {
    /// Group items into `Vec`s of `size`; the last group may be shorter. Empty input
    /// yields nothing. Panics if `size == 0`.
    fn chunked(self, _size: usize) -> Chunked<Self> {
        todo!()
    }

    /// Mean of each full window of `window` consecutive values. Partial windows are
    /// never emitted. Panics if `window == 0`.
    fn moving_average(self, _window: usize) -> MovingAverage<Self>
    where
        Self: Iterator<Item = f64>,
    {
        todo!()
    }
}

impl<I: Iterator> IterExt for I {}

/// Adapter returned by [`IterExt::chunked`].
pub struct Chunked<I> {
    _inner: I,
    _size: usize,
}

impl<I: Iterator> Iterator for Chunked<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl<I: FusedIterator> FusedIterator for Chunked<I> {}

/// Adapter returned by [`IterExt::moving_average`].
pub struct MovingAverage<I> {
    _inner: I,
    _window: usize,
}

impl<I: Iterator<Item = f64>> Iterator for MovingAverage<I> {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        todo!()
    }
}

/// Values before the first value `>= threshold`. Must not pull past that value.
pub fn leading_below<I>(_values: I, _threshold: f64) -> Vec<f64>
where
    I: IntoIterator<Item = f64>,
{
    todo!()
}

/// Running maximum: the peak seen so far at each position. Empty in, empty out.
pub fn running_peak<I>(_values: I) -> impl Iterator<Item = f64>
where
    I: IntoIterator<Item = f64>,
{
    todo!();
    #[allow(unreachable_code)]
    std::iter::empty()
}

/// Differences between adjacent timestamps (`later - earlier`). Fewer than two → nothing.
pub fn gaps(_timestamps: &[Timestamp]) -> impl Iterator<Item = SignedDuration> + '_ {
    todo!();
    #[allow(unreachable_code)]
    std::iter::empty()
}

/// Split into runs of *consecutive* records sharing a server. Non-adjacent records
/// of the same server form separate groups — by design.
pub fn group_by_server<'a>(_records: &'a [LogRecord]) -> impl Iterator<Item = &'a [LogRecord]> {
    todo!();
    #[allow(unreachable_code)]
    std::iter::empty()
}

/// Infinite exponential backoff: `base`, `2*base`, `4*base`, ... clamped to `cap`.
/// The first delay is `base` clamped to `cap`. Never overflows.
pub fn retry_delays(_base: Duration, _cap: Duration) -> impl Iterator<Item = Duration> {
    todo!();
    #[allow(unreachable_code)]
    std::iter::empty()
}
