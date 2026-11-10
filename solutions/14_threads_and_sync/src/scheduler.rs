//! Periodic jobs on dedicated threads, with a prompt, bounded stop.

use std::any::Any;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
        write!(f, "job `{}` failed: {}", self.job, self.message)
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
        match self {
            Self::InvalidInterval => f.write_str("interval must be non-zero"),
            Self::DuplicateName(name) => write!(f, "a job named `{name}` already exists"),
            Self::AlreadyStarted => f.write_str("scheduler already started"),
            Self::StopTimeout => f.write_str("timed out waiting for jobs to stop"),
        }
    }
}

impl Error for ScheduleError {}

struct Job {
    name: String,
    interval: Duration,
    task: Box<dyn FnMut() -> Result<(), BoxError> + Send>,
}

/// State shared by the scheduler and every job thread.
#[derive(Default)]
struct Shared {
    stopped: Mutex<bool>,
    wake: Condvar,
    errors: Mutex<Vec<JobError>>,
}

impl Shared {
    fn request_stop(&self) {
        // Set the flag under the same mutex the jobs wait on, so no wake-up is lost.
        *self.stopped.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.wake.notify_all();
    }

    fn record(&self, job: &str, message: String) {
        self.errors
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(JobError {
                job: job.to_owned(),
                message,
            });
    }
}

struct Running {
    handles: Vec<JoinHandle<()>>,
    /// Each job sends one `()` on this channel as its last act.
    finished: Receiver<()>,
    /// Jobs whose `()` has not been received yet.
    remaining: usize,
}

/// Runs named jobs periodically, one thread per job.
///
/// Each job runs immediately after `start`, then again every `interval`. A job
/// that returns `Err` or panics is recorded and keeps running.
#[derive(Default)]
pub struct Scheduler {
    jobs: Vec<Job>,
    started: bool,
    running: Option<Running>,
    shared: Arc<Shared>,
}

impl Scheduler {
    /// Creates an empty scheduler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a job.
    ///
    /// # Errors
    ///
    /// - [`ScheduleError::AlreadyStarted`] after `start`;
    /// - [`ScheduleError::InvalidInterval`] if `interval` is zero;
    /// - [`ScheduleError::DuplicateName`] if `name` is taken.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        interval: Duration,
        task: impl FnMut() -> Result<(), BoxError> + Send + 'static,
    ) -> Result<(), ScheduleError> {
        if self.started {
            return Err(ScheduleError::AlreadyStarted);
        }
        if interval.is_zero() {
            return Err(ScheduleError::InvalidInterval);
        }
        let name = name.into();
        if self.jobs.iter().any(|j| j.name == name) {
            return Err(ScheduleError::DuplicateName(name));
        }
        self.jobs.push(Job {
            name,
            interval,
            task: Box::new(task),
        });
        Ok(())
    }

    /// Starts one thread per registered job.
    ///
    /// # Errors
    ///
    /// Returns [`ScheduleError::AlreadyStarted`] if called before.
    pub fn start(&mut self) -> Result<(), ScheduleError> {
        if self.started {
            return Err(ScheduleError::AlreadyStarted);
        }
        self.started = true;
        let jobs = std::mem::take(&mut self.jobs);
        let (finished_tx, finished) = mpsc::channel();
        let remaining = jobs.len();
        let handles = jobs
            .into_iter()
            .map(|job| {
                let shared = Arc::clone(&self.shared);
                let finished = finished_tx.clone();
                thread::spawn(move || run_job(job, &shared, &finished))
            })
            .collect();
        self.running = Some(Running {
            handles,
            finished,
            remaining,
        });
        Ok(())
    }

    /// Asks every job to stop and waits up to `timeout` for them to finish.
    ///
    /// A job currently inside its task finishes that run first, then exits; a
    /// job waiting for its next tick exits immediately. On success every thread
    /// is joined. On [`ScheduleError::StopTimeout`] the jobs keep stopping in the
    /// background, and calling `stop` again waits for them.
    ///
    /// Calling `stop` before `start`, or after a successful stop, does nothing.
    pub fn stop(&mut self, timeout: Duration) -> Result<(), ScheduleError> {
        let Some(running) = self.running.as_mut() else {
            return Ok(());
        };
        self.shared.request_stop();
        let deadline = Instant::now() + timeout;
        while running.remaining > 0 {
            let left = deadline.saturating_duration_since(Instant::now());
            running
                .finished
                .recv_timeout(left)
                .map_err(|_| ScheduleError::StopTimeout)?;
            running.remaining -= 1;
        }
        if let Some(running) = self.running.take() {
            for handle in running.handles {
                let _ = handle.join();
            }
        }
        Ok(())
    }

    /// Every failure recorded so far, in the order it happened.
    pub fn errors(&self) -> Vec<JobError> {
        self.shared
            .errors
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        // Best effort: a job stuck past the timeout is detached, not leaked silently.
        let _ = self.stop(Duration::from_secs(5));
    }
}

fn run_job(mut job: Job, shared: &Shared, finished: &Sender<()>) {
    loop {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| (job.task)()));
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(err)) => shared.record(&job.name, err.to_string()),
            Err(payload) => shared.record(&job.name, format!("panic: {}", panic_text(&*payload))),
        }

        let stopped = shared.stopped.lock().unwrap_or_else(PoisonError::into_inner);
        let (stopped, _) = shared
            .wake
            .wait_timeout_while(stopped, job.interval, |s| !*s)
            .unwrap_or_else(PoisonError::into_inner);
        if *stopped {
            break;
        }
    }
    // Release the task's captures before signalling, so a successful `stop`
    // means the job's state is gone.
    drop(job);
    let _ = finished.send(());
}

fn panic_text(payload: &(dyn Any + Send)) -> &str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "non-string panic payload"
    }
}
