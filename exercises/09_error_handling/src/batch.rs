//! Part 7: batch aggregation, transient/permanent classification, retry.

use std::error::Error;
use std::io;
use std::marker::PhantomData;
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
        todo!()
    }
}

/// A named unit of work for [`run_batch`]. The closure may borrow from its environment.
pub struct Task<'a, T, E> {
    // private: the name and the boxed closure
    _marker: PhantomData<(&'a (), fn() -> Result<T, E>)>,
}

impl<'a, T, E> Task<'a, T, E> {
    /// Wraps `_run` under `_name`.
    pub fn new(_name: impl Into<String>, _run: impl FnOnce() -> Result<T, E> + 'a) -> Self {
        todo!()
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
pub fn run_batch<T, E>(_tasks: Vec<Task<'_, T, E>>) -> Result<Vec<(String, T)>, BatchError<E>>
where
    E: Error + 'static,
{
    todo!()
}

/// Failures split by [`Classify`]. Both lists keep task order.
#[derive(Debug)]
pub struct Triage<E: Error + 'static> {
    /// Failures worth retrying.
    pub transient: Vec<TaskFailed<E>>,
    /// Failures that will not go away on retry.
    pub permanent: Vec<TaskFailed<E>>,
}

/// Splits `_failures` into transient and permanent groups.
pub fn triage<E>(_failures: Vec<TaskFailed<E>>) -> Triage<E>
where
    E: Error + Classify + 'static,
{
    todo!()
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

/// Calls `_op` up to `_attempts` times.
///
/// After a transient failure on attempt `n` (not the last), calls
/// `_sleep(base_delay * 2^(n-1))`. A permanent failure returns immediately. The
/// final transient failure returns [`RetryError::Exhausted`] with no sleep.
pub fn retry<T, E>(
    _attempts: NonZeroU32,
    _base_delay: Duration,
    _op: impl FnMut() -> Result<T, E>,
    _sleep: impl FnMut(Duration),
) -> Result<T, RetryError<E>>
where
    E: Error + Classify + 'static,
{
    todo!()
}
