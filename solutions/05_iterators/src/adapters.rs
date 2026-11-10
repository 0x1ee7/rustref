//! Part 6 — adapters, extension traits and slice iterators.

use std::collections::VecDeque;
use std::iter::FusedIterator;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

use crate::log::LogRecord;

/// Extension methods available on every iterator.
pub trait IterExt: Iterator + Sized {
    /// Group items into `Vec`s of up to `size`. The last chunk may be shorter.
    /// Panics if `size == 0`.
    fn chunked(self, size: usize) -> Chunked<Self> {
        assert!(size >= 1, "batch size must be at least 1");
        Chunked { inner: self, size }
    }

    /// Mean of each full window of `window` consecutive values. Yields nothing until
    /// the first full window. Panics if `window == 0`.
    fn moving_average(self, window: usize) -> MovingAverage<Self>
    where
        Self: Iterator<Item = f64>,
    {
        assert!(window >= 1, "window must be at least 1");
        MovingAverage {
            inner: self,
            window,
            buf: VecDeque::with_capacity(window),
            sum: 0.0,
        }
    }
}

impl<I: Iterator> IterExt for I {}

/// Adapter returned by [`IterExt::chunked`].
#[derive(Debug, Clone)]
pub struct Chunked<I> {
    inner: I,
    size: usize,
}

impl<I: Iterator> Iterator for Chunked<I> {
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let batch: Vec<I::Item> = self.inner.by_ref().take(self.size).collect();
        if batch.is_empty() {
            None
        } else {
            Some(batch)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.inner.size_hint();
        (lo.div_ceil(self.size), hi.map(|h| h.div_ceil(self.size)))
    }
}

impl<I: FusedIterator> FusedIterator for Chunked<I> {}

/// Adapter returned by [`IterExt::moving_average`].
#[derive(Debug, Clone)]
pub struct MovingAverage<I> {
    inner: I,
    window: usize,
    buf: VecDeque<f64>,
    sum: f64,
}

impl<I: Iterator<Item = f64>> Iterator for MovingAverage<I> {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        loop {
            let x = self.inner.next()?;
            self.buf.push_back(x);
            self.sum += x;
            if self.buf.len() > self.window {
                self.sum -= self.buf.pop_front().unwrap_or_default();
            }
            if self.buf.len() == self.window {
                return Some(self.sum / self.window as f64);
            }
        }
    }
}

/// Values before the first value `>= threshold`. Must not pull past that value.
pub fn leading_below<I>(values: I, threshold: f64) -> Vec<f64>
where
    I: IntoIterator<Item = f64>,
{
    values.into_iter().take_while(|&v| v < threshold).collect()
}

/// Running maximum: the peak seen so far at each position. Empty in, empty out.
pub fn running_peak<I>(values: I) -> impl Iterator<Item = f64>
where
    I: IntoIterator<Item = f64>,
{
    values.into_iter().scan(None, |peak: &mut Option<f64>, x| {
        let next = peak.map_or(x, |p| p.max(x));
        *peak = Some(next);
        Some(next)
    })
}

/// Differences between adjacent timestamps (`later - earlier`). Fewer than two → nothing.
pub fn gaps(timestamps: &[Timestamp]) -> impl Iterator<Item = SignedDuration> + '_ {
    timestamps
        .windows(2)
        .map(|w| w[1].duration_since(w[0]))
}

/// Split into runs of *consecutive* records sharing a server. Non-adjacent records
/// of the same server form separate groups — by design.
pub fn group_by_server(records: &[LogRecord]) -> impl Iterator<Item = &[LogRecord]> {
    records.chunk_by(|a, b| a.server == b.server)
}

/// Infinite exponential backoff: `base`, `2*base`, `4*base`, ... clamped to `cap`.
/// The first delay is `base` clamped to `cap`. Never overflows.
pub fn retry_delays(base: Duration, cap: Duration) -> impl Iterator<Item = Duration> {
    std::iter::successors(Some(base.min(cap)), move |&d| {
        Some(d.checked_mul(2).map_or(cap, |doubled| doubled.min(cap)))
    })
}
