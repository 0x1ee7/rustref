//! Part 6: panics vs errors at a plugin boundary.

use std::any::Any;
use std::error::Error;
use std::panic::{UnwindSafe, catch_unwind};

/// Why an isolated plugin did not produce a value.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PluginError<E: Error + 'static> {
    /// The plugin returned an error. The error is the source.
    #[error("plugin '{name}' failed")]
    Failed {
        /// Plugin name.
        name: String,
        /// The plugin's error.
        #[source]
        source: E,
    },
    /// The plugin panicked. `message` is the payload when it is a string.
    #[error("plugin '{name}' panicked: {message}")]
    Panicked {
        /// Plugin name.
        name: String,
        /// The panic message, or `non-string panic payload`.
        message: String,
    },
}

/// Runs `plugin` behind `catch_unwind`.
///
/// Returns its value, [`PluginError::Failed`] for an `Err`, or
/// [`PluginError::Panicked`] when it unwinds. The closure must be
/// [`UnwindSafe`]; wrap it in `AssertUnwindSafe` to accept a `&mut` capture.
pub fn run_isolated<T, E>(
    name: &str,
    plugin: impl FnOnce() -> Result<T, E> + UnwindSafe,
) -> Result<T, PluginError<E>>
where
    E: Error + 'static,
{
    match catch_unwind(plugin) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(source)) => Err(PluginError::Failed {
            name: name.to_owned(),
            source,
        }),
        Err(payload) => Err(PluginError::Panicked {
            name: name.to_owned(),
            message: panic_message(payload.as_ref()),
        }),
    }
}

/// Extracts the message from the two payload types `panic!` produces.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}
