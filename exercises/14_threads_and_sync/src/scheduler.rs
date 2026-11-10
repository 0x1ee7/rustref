//! Part 7: periodic jobs on threads, with a prompt and bounded stop.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

/// Error type a job may return.
pub type BoxError = Box<dyn Error + Send + Sync>;

/// A failure recorded by the scheduler: which job, and what went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobError {
    /// Name the job was added under.
    pub job: String,
    /// The returned error's text, or `panic: <payload>` for a panic.
    pub message: String,
}

impl Display for JobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let _ = f;
        todo!()
    }
}

impl Error for JobError {}

/// Misuse of the scheduler or a stop that ran out of time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// The interval was zero.
    InvalidInterval,
    /// A job with this name was already added.
    DuplicateName(String),
    /// `start` was already called; no more jobs can be added.
    AlreadyStarted,
    /// Some jobs were still running when the timeout expired.
    StopTimeout,
}

impl Display for ScheduleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let _ = f;
        todo!()
    }
}

impl Error for ScheduleError {}

/// Runs named jobs periodically, one thread per job.
///
/// Requirements:
/// - `add` rejects a zero interval (`InvalidInterval`), a repeated name
///   (`DuplicateName`) and any call after `start` (`AlreadyStarted`);
/// - a job runs immediately on `start`, then every `interval`;
/// - a job returning `Err` is recorded as `JobError { job, message }` and keeps
///   running; a panicking job is recorded with `message` starting `panic: `;
/// - `stop(timeout)` wakes jobs that are waiting, joins them, and returns
///   `StopTimeout` if one is still inside its task after `timeout`;
/// - `stop` before `start` is a no-op; `start` twice is `AlreadyStarted`;
/// - dropping a running scheduler stops it and joins its threads.
#[derive(Default)]
pub struct Scheduler {
    // Private state.
}

impl Scheduler {
    /// Creates an empty scheduler.
    pub fn new() -> Self {
        todo!()
    }

    /// Registers a job.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        interval: Duration,
        task: impl FnMut() -> Result<(), BoxError> + Send + 'static,
    ) -> Result<(), ScheduleError> {
        let _: String = name.into();
        let _ = (interval, task);
        todo!()
    }

    /// Starts one thread per registered job.
    pub fn start(&mut self) -> Result<(), ScheduleError> {
        todo!()
    }

    /// Asks every job to stop and waits up to `timeout` for them to finish.
    pub fn stop(&mut self, timeout: Duration) -> Result<(), ScheduleError> {
        let _ = timeout;
        todo!()
    }

    /// Every failure recorded so far, in the order it happened.
    pub fn errors(&self) -> Vec<JobError> {
        todo!()
    }
}
