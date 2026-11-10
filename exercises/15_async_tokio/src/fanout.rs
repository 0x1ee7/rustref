//! Part 1 — fan-out with per-call timeouts (`aggregate`) and all-or-nothing fan-out
//! (`fetch_all_strict`).

use std::error::Error;
use std::fmt::{self, Display};
use std::future::Future;
use std::time::Duration;

use crate::error::ConfigError;

/// What happened to one source inside [`aggregate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The source returned `Ok(value)`.
    Ok(T),
    /// The source returned `Err(e)`; the payload is `e.to_string()`.
    Failed(String),
    /// Our per-call timeout fired first. A timeout error *returned by the source* is `Failed`.
    TimedOut,
}

/// One entry of the result of [`aggregate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceResult<T> {
    /// The name the source was registered under.
    pub name: String,
    /// What happened to it.
    pub outcome: Outcome<T>,
}

/// Calls every source concurrently, each under its own timeout, and returns partial results.
///
/// Contract:
/// - Every factory is called exactly once; sources run concurrently as spawned tasks.
/// - Each source future is wrapped in `tokio::time::timeout(per_call_timeout, ..)`.
/// - Results come back in the order of `sources`.
/// - A zero `per_call_timeout` is `ConfigError::ZeroTimeout`; no factory is called then.
/// - If the caller's future is dropped (e.g. by an outer `timeout`), every in-flight source is
///   aborted. Cancellation is never turned into a result.
///
/// # Errors
/// Only configuration errors. Per-source failures are data in the returned `Vec`.
pub async fn aggregate<T, E, F, Fut>(
    _sources: Vec<(String, F)>,
    _per_call_timeout: Duration,
) -> Result<Vec<SourceResult<T>>, ConfigError>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Display + Send + 'static,
{
    todo!()
}

/// The first failure of a [`fetch_all_strict`] batch.
#[derive(Debug)]
pub struct StrictError<E> {
    /// Name of the source that failed first.
    pub name: String,
    /// The error that source returned.
    pub error: E,
}

impl<E: Display> Display for StrictError<E> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<E: Error + 'static> Error for StrictError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        todo!()
    }
}

/// All-or-nothing fan-out.
///
/// Contract:
/// - Success returns `(name, value)` pairs in source order; an empty input returns `Ok(vec![])`.
/// - The first source to fail aborts every sibling. The error is returned only after every
///   sibling has been cancelled and has finished dropping its state.
/// - Siblings that were cancelled contribute nothing to the error.
pub async fn fetch_all_strict<T, E, F, Fut>(
    _sources: Vec<(String, F)>,
) -> Result<Vec<(String, T)>, StrictError<E>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    todo!()
}
