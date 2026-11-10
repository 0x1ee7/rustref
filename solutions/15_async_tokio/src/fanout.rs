//! Part 1 — fan-out with per-call timeouts (`aggregate`) and all-or-nothing fan-out
//! (`fetch_all_strict`).

use std::error::Error;
use std::fmt::{self, Display};
use std::panic;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::timeout;

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
/// # Errors
/// Only configuration errors. Per-source failures are data in the returned `Vec`.
pub async fn aggregate<T, E, F, Fut>(
    sources: Vec<(String, F)>,
    per_call_timeout: Duration,
) -> Result<Vec<SourceResult<T>>, ConfigError>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Display + Send + 'static,
{
    if per_call_timeout.is_zero() {
        return Err(ConfigError::ZeroTimeout);
    }

    let count = sources.len();
    // Dropping the set on early exit aborts every task still running.
    let mut set = JoinSet::new();
    for (index, (name, factory)) in sources.into_iter().enumerate() {
        set.spawn(async move {
            let outcome = match timeout(per_call_timeout, factory()).await {
                Err(_elapsed) => Outcome::TimedOut,
                Ok(Ok(value)) => Outcome::Ok(value),
                Ok(Err(error)) => Outcome::Failed(error.to_string()),
            };
            (index, SourceResult { name, outcome })
        });
    }

    let mut slots: Vec<Option<SourceResult<T>>> = (0..count).map(|_| None).collect();
    while let Some(joined) = set.join_next().await {
        let (index, result) = joined.unwrap_or_else(|e| panic::resume_unwind(e.into_panic()));
        slots[index] = Some(result);
    }
    Ok(slots
        .into_iter()
        .map(|slot| slot.expect("every source reports exactly once"))
        .collect())
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source `{}` failed: {}", self.name, self.error)
    }
}

impl<E: Error + 'static> Error for StrictError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

/// All-or-nothing fan-out.
///
/// The first failure aborts every sibling and waits for them to finish before returning, so no
/// work outlives the call.
pub async fn fetch_all_strict<T, E, F, Fut>(
    sources: Vec<(String, F)>,
) -> Result<Vec<(String, T)>, StrictError<E>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    let count = sources.len();
    let mut set = JoinSet::new();
    for (index, (name, factory)) in sources.into_iter().enumerate() {
        set.spawn(async move { (index, name, factory().await) });
    }

    let mut slots: Vec<Option<(String, T)>> = (0..count).map(|_| None).collect();
    while let Some(joined) = set.join_next().await {
        let (index, name, result) = joined.unwrap_or_else(|e| panic::resume_unwind(e.into_panic()));
        match result {
            Ok(value) => slots[index] = Some((name, value)),
            Err(error) => {
                set.shutdown().await;
                return Err(StrictError { name, error });
            }
        }
    }
    Ok(slots
        .into_iter()
        .map(|slot| slot.expect("every source reports exactly once"))
        .collect())
}
