//! Part 3: a checked step with a log trail, `?`-style propagation and unwinding.

use std::error::Error;

use crate::config::BoxError;

/// A step failed. Displays as `step '<name>' failed`; the cause is the source.
#[derive(Debug, thiserror::Error)]
#[error("step '{step_name}' failed")]
#[non_exhaustive]
pub struct StepFailed {
    /// Name of the step that failed.
    pub step_name: String,
    #[source]
    source: BoxError,
}

/// Writes `end <name>` when dropped, so the trail closes even on panic.
struct StepLog<'a> {
    name: &'a str,
    log: &'a mut Vec<String>,
}

impl<'a> StepLog<'a> {
    fn start(name: &'a str, log: &'a mut Vec<String>) -> Self {
        log.push(format!("start {name}"));
        Self { name, log }
    }
}

impl Drop for StepLog<'_> {
    fn drop(&mut self) {
        self.log.push(format!("end {}", self.name));
    }
}

/// Runs `step`, recording its trail in `log`.
///
/// Logs `start <name>` before the call and `end <name>` after it, including
/// when the step panics. Then `ok <name>` on success, or `failed <name>` and
/// [`StepFailed`] on error. A panic propagates unchanged and is not logged as
/// `failed`.
pub fn run_checked<T, E>(
    name: &str,
    step: impl FnOnce() -> Result<T, E>,
    log: &mut Vec<String>,
) -> Result<T, StepFailed>
where
    E: Error + Send + Sync + 'static,
{
    let outcome = {
        let _guard = StepLog::start(name, &mut *log);
        step()
    };
    match outcome {
        Ok(value) => {
            log.push(format!("ok {name}"));
            Ok(value)
        }
        Err(err) => {
            log.push(format!("failed {name}"));
            Err(StepFailed {
                step_name: name.to_owned(),
                source: Box::new(err),
            })
        }
    }
}
