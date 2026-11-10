//! Part 7: batch aggregation, transient/permanent classification, retry.

use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::num::NonZeroU32;
use std::time::Duration;

/// Whether a failure is worth retrying. Implemented by each error type that takes part.
pub trait Classify {
    /// `true` when repeating the same operation may succeed later.
    fn is_transient(&self) -> bool;
}

/// Transient kinds: `TimedOut`, `Interrupted`, `WouldBlock` (EAGAIN),
/// `ResourceBusy` (EBUSY), `ConnectionAborted`, `ConnectionRefused`,
/// `ConnectionReset`, `BrokenPipe`. Every other kind is permanent.
impl Classify for io::Error {
    fn is_transient(&self) -> bool {
        use io::ErrorKind::{
            BrokenPipe, ConnectionAborted, ConnectionRefused, ConnectionReset, Interrupted,
            ResourceBusy, TimedOut, WouldBlock,
        };
        matches!(
            self.kind(),
            TimedOut
                | Interrupted
                | WouldBlock
                | ResourceBusy
                | ConnectionAborted
                | ConnectionRefused
                | ConnectionReset
                | BrokenPipe
        )
    }
}

/// A named unit of work for [`run_batch`]. The closure may borrow from its environment.
pub struct Task<'a, T, E> {
    name: String,
    run: Box<dyn FnOnce() -> Result<T, E> + 'a>,
}

impl<'a, T, E> Task<'a, T, E> {
    /// Wraps `run` under `name`.
    pub fn new(name: impl Into<String>, run: impl FnOnce() -> Result<T, E> + 'a) -> Self {
        Self {
            name: name.into(),
            run: Box::new(run),
        }
    }
}

/// One failed task. Displays as `task '<name>' failed`; the task's error is the source.
#[derive(Debug, thiserror::Error)]
#[error("task '{name}' failed")]
pub struct TaskFailed<E: Error + 'static> {
    /// Name of the failed task.
    pub name: String,
    /// The task's error.
    #[source]
    pub source: E,
}

/// Errors from [`run_batch`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BatchError<E: Error + 'static> {
    /// Two tasks share a name. Nothing ran.
    #[error("task name '{0}' is used more than once")]
    DuplicateTask(String),
    /// One or more tasks failed. `failures` lists them in task order.
    #[error("{} of {} tasks failed", .failures.len(), .total)]
    Failed {
        /// How many tasks ran.
        total: usize,
        /// Every failure, in task order.
        failures: Vec<TaskFailed<E>>,
    },
}

/// Runs every task in order and collects all failures.
///
/// Returns each task's result in task order when all succeed. A panic in a task
/// is not collected; it propagates and stops the batch.
pub fn run_batch<T, E>(tasks: Vec<Task<'_, T, E>>) -> Result<Vec<(String, T)>, BatchError<E>>
where
    E: Error + 'static,
{
    let mut names = HashSet::new();
    for task in &tasks {
        if !names.insert(task.name.as_str()) {
            return Err(BatchError::DuplicateTask(task.name.clone()));
        }
    }

    let total = tasks.len();
    let mut results = Vec::with_capacity(total);
    let mut failures = Vec::new();
    for task in tasks {
        match (task.run)() {
            Ok(value) => results.push((task.name, value)),
            Err(source) => failures.push(TaskFailed {
                name: task.name,
                source,
            }),
        }
    }

    if failures.is_empty() {
        Ok(results)
    } else {
        Err(BatchError::Failed { total, failures })
    }
}

/// Failures split by [`Classify`]. Both lists keep task order.
#[derive(Debug)]
pub struct Triage<E: Error + 'static> {
    /// Failures worth retrying.
    pub transient: Vec<TaskFailed<E>>,
    /// Failures that will not go away on retry.
    pub permanent: Vec<TaskFailed<E>>,
}

/// Splits `failures` into transient and permanent groups.
pub fn triage<E>(failures: Vec<TaskFailed<E>>) -> Triage<E>
where
    E: Error + Classify + 'static,
{
    let (transient, permanent) = failures
        .into_iter()
        .partition(|failure| failure.source.is_transient());
    Triage {
        transient,
        permanent,
    }
}

/// Errors from [`retry`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RetryError<E: Error + 'static> {
    /// A permanent error, returned on the attempt that produced it. Transparent.
    #[error(transparent)]
    Permanent(E),
    /// Every attempt failed transiently. `last` is the final attempt's error.
    #[error("gave up after {attempts} attempts")]
    Exhausted {
        /// Number of attempts made.
        attempts: u32,
        /// The final attempt's error.
        #[source]
        last: E,
    },
}

/// Calls `op` up to `attempts` times.
///
/// After a transient failure on attempt `n` (not the last), calls
/// `sleep(base_delay * 2^(n-1))`. A permanent failure returns immediately. The
/// final transient failure returns [`RetryError::Exhausted`] with no sleep.
pub fn retry<T, E>(
    attempts: NonZeroU32,
    base_delay: Duration,
    mut op: impl FnMut() -> Result<T, E>,
    mut sleep: impl FnMut(Duration),
) -> Result<T, RetryError<E>>
where
    E: Error + Classify + 'static,
{
    let max = attempts.get();
    for attempt in 1..=max {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if !err.is_transient() => return Err(RetryError::Permanent(err)),
            Err(last) if attempt == max => {
                return Err(RetryError::Exhausted {
                    attempts: max,
                    last,
                });
            }
            Err(_) => sleep(backoff(base_delay, attempt)),
        }
    }
    unreachable!("NonZeroU32 guarantees at least one attempt")
}

/// `base * 2^(attempt-1)`, saturating instead of overflowing.
fn backoff(base: Duration, attempt: u32) -> Duration {
    base.saturating_mul(2u32.saturating_pow(attempt - 1))
}
