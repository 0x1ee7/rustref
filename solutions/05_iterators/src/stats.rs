//! Part 5 — collecting and extending, with an RAII finaliser.
//!
//! Statistics are folded one value at a time with Welford's incremental mean, so
//! nothing is stored. The finaliser runs from `Drop`, so it fires on early return and
//! on unwind as well as on normal scope exit.

/// Snapshot of a stream's statistics. The empty state is `Stats::default()`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stats {
    /// Number of values folded in.
    pub count: u64,
    /// Arithmetic mean; `None` when empty.
    pub mean: Option<f64>,
    /// Smallest value; `None` when empty.
    pub minimum: Option<f64>,
    /// Largest value; `None` when empty.
    pub maximum: Option<f64>,
}

/// Incremental accumulator.
///
/// `on_close`, when given, is invoked exactly once with the final [`Stats`] when the
/// accumulator is dropped.
pub struct RunningStats {
    count: u64,
    mean: f64,
    /// Sum of squared deviations from the running mean (Welford's `M2`).
    m2: f64,
    min: f64,
    max: f64,
    on_close: Option<Box<dyn FnOnce(Stats)>>,
}

impl RunningStats {
    /// Empty accumulator with no finaliser.
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            on_close: None,
        }
    }

    /// Empty accumulator that reports its final stats to `on_close` when dropped.
    pub fn with_on_close(on_close: impl FnOnce(Stats) + 'static) -> Self {
        Self {
            on_close: Some(Box::new(on_close)),
            ..Self::new()
        }
    }

    /// Fold one value into the statistics.
    pub fn push(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        self.m2 += delta * (x - self.mean);
        self.min = self.min.min(x);
        self.max = self.max.max(x);
    }

    /// Current statistics, without consuming anything.
    pub fn snapshot(&self) -> Stats {
        let some = |v: f64| (self.count > 0).then_some(v);
        Stats {
            count: self.count,
            mean: some(self.mean),
            minimum: some(self.min),
            maximum: some(self.max),
        }
    }

    /// Clear back to the empty state. The finaliser is kept.
    pub fn reset(&mut self) {
        let on_close = self.on_close.take();
        *self = Self {
            on_close,
            ..Self::new()
        };
    }
}

impl Default for RunningStats {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<f64> for RunningStats {
    fn extend<I: IntoIterator<Item = f64>>(&mut self, iter: I) {
        for x in iter {
            self.push(x);
        }
    }
}

impl Drop for RunningStats {
    fn drop(&mut self) {
        if let Some(on_close) = self.on_close.take() {
            on_close(self.snapshot());
        }
    }
}

impl FromIterator<f64> for Stats {
    /// Statistics of everything the iterator yields.
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        let mut running = RunningStats::new();
        running.extend(iter);
        running.snapshot()
    }
}
