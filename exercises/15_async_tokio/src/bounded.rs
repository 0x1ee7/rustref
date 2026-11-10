//! Part 3 — bounded concurrency with a `Semaphore`, and blocking file IO off the runtime.

use std::future::Future;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

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

/// Runs fetches with at most `limit` in flight at any moment.
///
/// The limit is shared by every `run` on the same fetcher. `peak_in_flight` is the highest
/// concurrency ever observed on this fetcher.
pub struct BoundedFetcher {
    /// Engineer: semaphore, in-flight counter, peak counter.
    _private: (),
}

impl BoundedFetcher {
    /// Creates a fetcher. `limit == 0` is `ConfigError::ZeroLimit`.
    pub fn new(_limit: usize) -> Result<Self, ConfigError> {
        todo!()
    }

    /// Highest number of fetches that were running at the same time.
    pub fn peak_in_flight(&self) -> usize {
        todo!()
    }

    /// Fetches every item and returns results in input order.
    ///
    /// The first failure cancels the remaining fetches, waits for them to stop, and returns the
    /// failing item's index with its error. An empty input returns `Ok(vec![])`.
    pub async fn run<T, R, E, F, Fut>(
        &self,
        _items: Vec<T>,
        _fetch: F,
    ) -> Result<Vec<R>, FetchError<E>>
    where
        T: Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, E>> + Send + 'static,
    {
        todo!()
    }
}

/// Reads every file with at most `limit` reads in flight. Each read runs on the blocking pool.
///
/// Returns `(path, contents)` pairs in input order.
pub async fn read_files(
    _paths: Vec<PathBuf>,
    _limit: usize,
) -> Result<Vec<(PathBuf, Vec<u8>)>, ReadFilesError> {
    todo!()
}
