//! Part 6: panics vs errors at a plugin boundary.

use std::error::Error;
use std::panic::UnwindSafe;

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

/// Runs `_plugin` behind `catch_unwind`.
///
/// Returns its value, [`PluginError::Failed`] for an `Err`, or
/// [`PluginError::Panicked`] when it unwinds. The closure must be
/// [`UnwindSafe`]; wrap it in `AssertUnwindSafe` to accept a `&mut` capture.
pub fn run_isolated<T, E>(
    _name: &str,
    _plugin: impl FnOnce() -> Result<T, E> + UnwindSafe,
) -> Result<T, PluginError<E>>
where
    E: Error + 'static,
{
    todo!()
}
