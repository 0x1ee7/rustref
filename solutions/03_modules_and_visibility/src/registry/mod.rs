//! A named plugin registry. The registry is explicit context; a process-wide
//! instance lives in [`global`].

pub mod global;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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
        Self {
            name: name.into(),
            f,
        }
    }
}

impl<F> Plugin for FnPlugin<F>
where
    F: Fn(&str) -> Result<String, PluginError> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, input: &str) -> Result<String, PluginError> {
        (self.f)(input)
    }
}

type Plugins = BTreeMap<String, Arc<dyn Plugin>>;

/// A thread-safe, name-keyed collection of plugins. Methods take `&self`:
/// internal mutability keeps the registry shareable behind an `Arc` or a `static`.
#[derive(Default)]
pub struct Registry {
    plugins: Mutex<Plugins>,
}

/// Checks `[a-z][a-z0-9_-]*` with length 1..=32.
fn validate_name(name: &str) -> Result<(), RegistryError> {
    let valid = (1..=32).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if valid {
        Ok(())
    } else {
        Err(RegistryError::InvalidName(name.to_owned()))
    }
}

impl Registry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Plugins> {
        // Plugins run outside the lock, so poisoning cannot leave the map torn.
        self.plugins.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers `plugin` under its own [`Plugin::name`].
    ///
    /// # Errors
    ///
    /// [`RegistryError::InvalidName`] or [`RegistryError::Duplicate`].
    pub fn register<P: Plugin + 'static>(&self, plugin: P) -> Result<(), RegistryError> {
        let name = plugin.name().to_owned();
        validate_name(&name)?;
        match self.lock().entry(name) {
            Entry::Occupied(taken) => Err(RegistryError::Duplicate(taken.key().clone())),
            Entry::Vacant(slot) => {
                slot.insert(Arc::new(plugin));
                Ok(())
            }
        }
    }

    /// Removes the plugin named `name`.
    ///
    /// # Errors
    ///
    /// [`RegistryError::NotFound`] when absent.
    pub fn unregister(&self, name: &str) -> Result<(), RegistryError> {
        self.lock()
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| RegistryError::NotFound(name.to_owned()))
    }

    /// Returns a shared handle to the plugin named `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.lock().get(name).cloned()
    }

    /// Registered names, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Number of registered plugins.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// `true` when no plugins are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// Removes every plugin.
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Runs every plugin in name order and returns `(name, result)` pairs.
    /// Plugins run without the registry lock held, so a plugin may call back into the registry.
    #[must_use]
    pub fn run_all(&self, input: &str) -> Vec<(String, Result<String, PluginError>)> {
        let snapshot: Vec<(String, Arc<dyn Plugin>)> = self
            .lock()
            .iter()
            .map(|(name, plugin)| (name.clone(), Arc::clone(plugin)))
            .collect();
        snapshot
            .into_iter()
            .map(|(name, plugin)| {
                let result = plugin.run(input);
                (name, result)
            })
            .collect()
    }
}
