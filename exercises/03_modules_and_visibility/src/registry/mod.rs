//! A named plugin registry. The registry is explicit context; a process-wide
//! instance lives in [`global`].

pub mod global;

use std::sync::Arc;

/// Errors from [`Plugin::run`] implementations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginError {
    /// The plugin ran and reported a failure.
    #[error("plugin failed: {0}")]
    Failed(String),
}

/// Errors from [`Registry`] management methods.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// The name does not match `[a-z][a-z0-9_-]*` with length 1..=32.
    #[error("invalid plugin name {0:?}")]
    InvalidName(String),
    /// A plugin with this name is already registered.
    #[error("plugin {0:?} is already registered")]
    Duplicate(String),
    /// No plugin with this name is registered.
    #[error("no plugin named {0:?}")]
    NotFound(String),
}

/// A unit of work the registry can run. Plugins are shared across threads.
pub trait Plugin: Send + Sync {
    /// The plugin's registered name.
    fn name(&self) -> &str;

    /// Runs the plugin on `input`.
    ///
    /// # Errors
    ///
    /// [`PluginError`] when the work fails.
    fn run(&self, input: &str) -> Result<String, PluginError>;
}

/// Adapts a closure into a [`Plugin`].
#[allow(dead_code)]
pub struct FnPlugin<F> {
    name: String,
    f: F,
}

impl<F> FnPlugin<F>
where
    F: Fn(&str) -> Result<String, PluginError> + Send + Sync,
{
    /// Wraps `f` under `name`. Name validation happens at registration.
    pub fn new(name: impl Into<String>, f: F) -> Self {
        let _ = (name.into(), f);
        todo!()
    }
}

impl<F> Plugin for FnPlugin<F>
where
    F: Fn(&str) -> Result<String, PluginError> + Send + Sync,
{
    fn name(&self) -> &str {
        todo!()
    }

    fn run(&self, input: &str) -> Result<String, PluginError> {
        let _ = input;
        todo!()
    }
}

/// A thread-safe, name-keyed collection of plugins. Methods take `&self`:
/// internal mutability keeps the registry shareable behind an `Arc` or a `static`.
#[derive(Default)]
pub struct Registry {}

impl Registry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// Registers `plugin` under its own [`Plugin::name`].
    ///
    /// # Errors
    ///
    /// [`RegistryError::InvalidName`] or [`RegistryError::Duplicate`].
    pub fn register<P: Plugin + 'static>(&self, plugin: P) -> Result<(), RegistryError> {
        let _ = plugin;
        todo!()
    }

    /// Removes the plugin named `name`.
    ///
    /// # Errors
    ///
    /// [`RegistryError::NotFound`] when absent.
    pub fn unregister(&self, name: &str) -> Result<(), RegistryError> {
        let _ = name;
        todo!()
    }

    /// Returns a shared handle to the plugin named `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        let _ = name;
        todo!()
    }

    /// Registered names, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        todo!()
    }

    /// Number of registered plugins.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no plugins are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Removes every plugin.
    pub fn clear(&self) {
        todo!()
    }

    /// Runs every plugin in name order and returns `(name, result)` pairs.
    /// Plugins run without the registry lock held, so a plugin may call back into the registry.
    #[must_use]
    pub fn run_all(&self, input: &str) -> Vec<(String, Result<String, PluginError>)> {
        let _ = input;
        todo!()
    }
}
