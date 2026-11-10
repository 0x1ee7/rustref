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

/// Runs `_step`, recording its trail in `_log`.
///
/// Logs `start <name>` before the call and `end <name>` after it, including
/// when the step panics. Then `ok <name>` on success, or `failed <name>` and
/// [`StepFailed`] on error. A panic propagates unchanged and is not logged as
/// `failed`.
pub fn run_checked<T, E>(
    _name: &str,
    _step: impl FnOnce() -> Result<T, E>,
    _log: &mut Vec<String>,
) -> Result<T, StepFailed>
where
    E: Error + Send + Sync + 'static,
{
    todo!()
}
