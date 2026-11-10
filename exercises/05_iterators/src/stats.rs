//! Part 5 — collecting and extending, with an RAII finaliser.
//!
//! Statistics are folded one value at a time with Welford's incremental mean, so
//! nothing is stored. The finaliser runs from `Drop`, so it fires on early return and
//! on unwind as well as on normal scope exit.

/// Snapshot of a stream's statistics. The empty state is `Stats::default()`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stats {
    pub count: u64,
    pub mean: Option<f64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

/// Incremental accumulator.
///
/// `on_close`, when given, is invoked exactly once with the final [`Stats`] when the
/// accumulator is dropped. Private state is yours to design.
pub struct RunningStats {
    _state: Stats,
    _on_close: Option<Box<dyn FnOnce(Stats)>>,
}

impl RunningStats {
    /// Empty accumulator with no finaliser.
    pub fn new() -> Self {
        todo!()
    }

    /// Empty accumulator that reports its final stats to `on_close` when dropped.
    pub fn with_on_close(_on_close: impl FnOnce(Stats) + 'static) -> Self {
        todo!()
    }

    /// Fold one value into the statistics.
    pub fn push(&mut self, _x: f64) {
        todo!()
    }

    /// Current statistics, without consuming anything.
    pub fn snapshot(&self) -> Stats {
        todo!()
    }

    /// Clear back to the empty state. The finaliser is kept.
    pub fn reset(&mut self) {
        todo!()
    }
}

impl Default for RunningStats {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<f64> for RunningStats {
    fn extend<I: IntoIterator<Item = f64>>(&mut self, _iter: I) {
        todo!()
    }
}

impl Drop for RunningStats {
    fn drop(&mut self) {
        // Engineer: fire the finaliser here, exactly once.
    }
}

impl FromIterator<f64> for Stats {
    /// Statistics of everything the iterator yields.
    fn from_iter<I: IntoIterator<Item = f64>>(_iter: I) -> Self {
        todo!()
    }
}
