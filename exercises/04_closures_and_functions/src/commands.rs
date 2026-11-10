//! Part 7: a command registry of boxed handlers keyed by static names.

use std::collections::BTreeMap;
use thiserror::Error;

/// Error returned by a command handler.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The arguments were the wrong shape. The message describes the usage.
    #[error("usage: {0}")]
    Usage(String),
    /// The command ran but failed.
    #[error("command failed: {0}")]
    Failed(String),
}

/// Registration failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Names must be non-empty and contain no whitespace.
    #[error("invalid command name {0:?}: must be non-empty without whitespace")]
    InvalidName(&'static str),
    /// Each name can be registered once.
    #[error("command {0:?} is already registered")]
    Duplicate(&'static str),
}

/// Failure from [`CommandRegistry::dispatch`].
#[derive(Debug, Error)]
pub enum DispatchError {
    /// No command is registered under this name.
    #[error("unknown command {0:?}")]
    UnknownCommand(String),
    /// The handler ran and returned an error.
    #[error(transparent)]
    Command(#[from] CommandError),
}

/// A boxed command handler.
type Handler = Box<dyn Fn(&[String]) -> Result<String, CommandError> + Send + Sync>;

/// A registered handler together with its help text.
#[allow(dead_code)] // stub: remove once the fields are used
struct Command {
    help: &'static str,
    handler: Handler,
}

/// Maps command names to handlers. Each registry is independent.
///
/// Names are `&'static str`, so keys cost nothing to store. `BTreeMap` keeps
/// `names()` sorted.
#[derive(Default)]
pub struct CommandRegistry {
    #[allow(dead_code)] // stub: remove once the field is used
    commands: BTreeMap<&'static str, Command>,
}

impl CommandRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        todo!()
    }

    /// Registers `handler` under `name`, with one line of `help` text.
    ///
    /// Rejects invalid and duplicate names.
    pub fn register<F>(
        &mut self,
        _name: &'static str,
        _help: &'static str,
        _handler: F,
    ) -> Result<(), RegistryError>
    where
        F: Fn(&[String]) -> Result<String, CommandError> + Send + Sync + 'static,
    {
        todo!()
    }

    /// Runs the handler registered under `name` with `args`.
    pub fn dispatch(&self, _name: &str, _args: &[String]) -> Result<String, DispatchError> {
        todo!()
    }

    /// All registered names, sorted.
    pub fn names(&self) -> Vec<&'static str> {
        todo!()
    }

    /// Help text for `name`, or `None` if the command is not registered.
    pub fn help(&self, _name: &str) -> Option<&'static str> {
        todo!()
    }
}
