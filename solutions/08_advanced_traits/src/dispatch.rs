//! Part 1 — trait objects vs generics: alert sinks.
//!
//! `Notifier` must stay dyn-compatible: no generic methods, no by-value
//! `self`, no `Sized` supertrait.

use std::sync::{Mutex, MutexGuard, PoisonError};

use thiserror::Error;

/// Why a notifier could not deliver a message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NotifyError {
    /// The channel is down or was never configured.
    #[error("channel '{channel}' is unavailable")]
    Unavailable { channel: String },
    /// The channel is reachable but refused this message.
    #[error("message rejected: {reason}")]
    Rejected { reason: String },
}

/// A destination for alert messages.
pub trait Notifier {
    /// Channel name used in error reports.
    fn name(&self) -> &str;

    /// Delivers one message. Takes `&self`: record state with interior mutability.
    fn notify(&self, message: &str) -> Result<(), NotifyError>;
}

/// Records delivered messages. A failing one rejects every delivery with
/// `NotifyError::Unavailable` and records nothing.
#[derive(Debug)]
pub struct RecordingNotifier {
    name: String,
    failing: bool,
    sent: Mutex<Vec<String>>,
}

impl RecordingNotifier {
    /// Creates a notifier that accepts every message.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), failing: false, sent: Mutex::new(Vec::new()) }
    }

    /// Creates a notifier whose every delivery fails with
    /// `NotifyError::Unavailable { channel: name }`.
    pub fn failing(name: impl Into<String>) -> Self {
        Self { failing: true, ..Self::new(name) }
    }

    /// Messages accepted so far, in delivery order.
    pub fn sent(&self) -> Vec<String> {
        self.log().clone()
    }

    /// The log guard. A poisoned lock still holds a valid `Vec`, so recover it.
    fn log(&self) -> MutexGuard<'_, Vec<String>> {
        self.sent.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Notifier for RecordingNotifier {
    fn name(&self) -> &str {
        &self.name
    }

    fn notify(&self, message: &str) -> Result<(), NotifyError> {
        if self.failing {
            return Err(NotifyError::Unavailable { channel: self.name.clone() });
        }
        self.log().push(message.to_owned());
        Ok(())
    }
}

/// Owns heterogeneous notifiers and broadcasts to all of them.
#[derive(Default)]
pub struct FanOut {
    targets: Vec<Box<dyn Notifier + Send + Sync>>,
}

impl FanOut {
    /// An empty fan-out.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a target and returns the fan-out for chaining.
    #[must_use]
    pub fn with(mut self, target: impl Notifier + Send + Sync + 'static) -> Self {
        self.targets.push(Box::new(target));
        self
    }

    /// Number of targets.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// `true` when there are no targets.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Attempts delivery to every target, including those after a failure.
    /// Returns `Ok(n)` where `n` is the number of targets when all succeed,
    /// otherwise every failure as a [`FanOutError`].
    pub fn broadcast(&self, message: &str) -> Result<usize, FanOutError> {
        let failures: Vec<(String, NotifyError)> = self
            .targets
            .iter()
            .filter_map(|target| {
                target.notify(message).err().map(|e| (target.name().to_owned(), e))
            })
            .collect();

        if failures.is_empty() {
            Ok(self.targets.len())
        } else {
            Err(FanOutError { attempted: self.targets.len(), failures })
        }
    }
}

/// Aggregate failure of a broadcast. Every target was attempted.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}/{attempted} notifiers failed", failures.len())]
pub struct FanOutError {
    /// Number of targets that were attempted.
    pub attempted: usize,
    /// `(channel name, error)` for each failed target, in registration order.
    pub failures: Vec<(String, NotifyError)>,
}

/// Sends through any notifier, including `dyn` references.
pub fn send_one<N: Notifier + ?Sized>(notifier: &N, message: &str) -> Result<(), NotifyError> {
    notifier.notify(message)
}

/// Forwarding impl: a boxed notifier, including `Box<dyn Notifier>`, is itself a notifier.
impl<N: Notifier + ?Sized> Notifier for Box<N> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn notify(&self, message: &str) -> Result<(), NotifyError> {
        (**self).notify(message)
    }
}
