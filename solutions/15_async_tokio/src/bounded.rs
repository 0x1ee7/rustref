//! Part 3 — bounded concurrency with a `Semaphore`, and blocking file IO off the runtime.

use std::io;
use std::panic;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use thiserror::Error;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::ConfigError;

/// A fetch that failed, with the index of the item it was given.
#[derive(Debug)]
pub struct FetchError<E> {
    /// Position of the failing item in the input.
    pub index: usize,
    /// The error the fetch returned.
    pub error: E,
}

/// Why [`read_files`] failed.
#[derive(Debug, Error)]
pub enum ReadFilesError {
    /// The concurrency limit was invalid.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Reading the file at `index` failed.
    #[error("reading file #{index} failed")]
    Io {
        /// Position of the failing path in the input.
        index: usize,
        /// The underlying IO error (e.g. `NotFound`).
        #[source]
        error: io::Error,
    },
}

/// Shared counters: how many fetches are running now, and the highest number ever seen.
#[derive(Default)]
struct Gauge {
    current: AtomicUsize,
    peak: AtomicUsize,
}

/// Marks one running fetch. Dropping it (including on cancellation) decrements the count.
struct InFlight(Arc<Gauge>);

impl InFlight {
    fn enter(gauge: Arc<Gauge>) -> Self {
        let now = gauge.current.fetch_add(1, Ordering::SeqCst) + 1;
        gauge.peak.fetch_max(now, Ordering::SeqCst);
        Self(gauge)
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.current.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Runs fetches with at most `limit` in flight at any moment.
///
/// The limit is shared by every `run` on the same fetcher.
pub struct BoundedFetcher {
    permits: Arc<Semaphore>,
    gauge: Arc<Gauge>,
}

impl BoundedFetcher {
    /// Creates a fetcher. `limit == 0` is `ConfigError::ZeroLimit`.
    pub fn new(limit: usize) -> Result<Self, ConfigError> {
        if limit == 0 {
            return Err(ConfigError::ZeroLimit);
        }
        Ok(Self {
            permits: Arc::new(Semaphore::new(limit)),
            gauge: Arc::new(Gauge::default()),
        })
    }

    /// Highest number of fetches that were running at the same time.
    pub fn peak_in_flight(&self) -> usize {
        self.gauge.peak.load(Ordering::SeqCst)
    }

    /// Fetches every item and returns results in input order.
    ///
    /// The first failure cancels the remaining fetches, waits for them to stop, and returns the
    /// failing item's index with its error.
    pub async fn run<T, R, E, F, Fut>(&self, items: Vec<T>, fetch: F) -> Result<Vec<R>, FetchError<E>>
    where
        T: Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, E>> + Send + 'static,
    {
        let count = items.len();
        let fetch = Arc::new(fetch);
        let mut set = JoinSet::new();
        for (index, item) in items.into_iter().enumerate() {
            let permits = Arc::clone(&self.permits);
            let gauge = Arc::clone(&self.gauge);
            let fetch = Arc::clone(&fetch);
            set.spawn(async move {
                let _permit = permits
                    .acquire_owned()
                    .await
                    .expect("semaphore is never closed");
                let _in_flight = InFlight::enter(gauge);
                (index, (*fetch)(item).await)
            });
        }

        let mut slots: Vec<Option<R>> = (0..count).map(|_| None).collect();
        while let Some(joined) = set.join_next().await {
            let (index, result) = joined.unwrap_or_else(|e| panic::resume_unwind(e.into_panic()));
            match result {
                Ok(value) => slots[index] = Some(value),
                Err(error) => {
                    set.shutdown().await;
                    return Err(FetchError { index, error });
                }
            }
        }
        Ok(slots
            .into_iter()
            .map(|slot| slot.expect("every fetch reports exactly once"))
            .collect())
    }
}

/// Reads every file with at most `limit` reads in flight. Each read runs on the blocking pool.
///
/// Returns `(path, contents)` pairs in input order.
pub async fn read_files(
    paths: Vec<PathBuf>,
    limit: usize,
) -> Result<Vec<(PathBuf, Vec<u8>)>, ReadFilesError> {
    let fetcher = BoundedFetcher::new(limit)?;
    let files = fetcher
        .run(paths, |path: PathBuf| async move {
            let read_from = path.clone();
            let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read_from))
                .await
                .unwrap_or_else(|e| panic::resume_unwind(e.into_panic()))?;
            Ok::<_, io::Error>((path, bytes))
        })
        .await
        .map_err(|FetchError { index, error }| ReadFilesError::Io { index, error })?;
    Ok(files)
}
