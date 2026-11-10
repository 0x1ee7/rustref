//! Part 1 — trait objects vs generics: alert sinks.
//!
//! `Notifier` must stay dyn-compatible: no generic methods, no by-value
//! `self`, no `Sized` supertrait.

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
///
/// Engineer: add fields (channel name, failure flag, recorded messages behind
/// interior mutability so `notify(&self)` can record).
#[derive(Debug)]
pub struct RecordingNotifier;

impl RecordingNotifier {
    /// Creates a notifier that accepts every message.
    pub fn new(name: impl Into<String>) -> Self {
        let _ = name;
        todo!()
    }

    /// Creates a notifier whose every delivery fails with
    /// `NotifyError::Unavailable { channel: name }`.
    pub fn failing(name: impl Into<String>) -> Self {
        let _ = name;
        todo!()
    }

    /// Messages accepted so far, in delivery order.
    pub fn sent(&self) -> Vec<String> {
        let _ = self;
        todo!()
    }
}

impl Notifier for RecordingNotifier {
    fn name(&self) -> &str {
        let _ = self;
        todo!()
    }

    fn notify(&self, message: &str) -> Result<(), NotifyError> {
        let _ = (self, message);
        todo!()
    }
}

/// Owns heterogeneous notifiers and broadcasts to all of them.
///
/// Engineer: store the targets as `Box<dyn Notifier + Send + Sync>`.
#[derive(Default)]
pub struct FanOut {}

impl FanOut {
    /// An empty fan-out.
    pub fn new() -> Self {
        todo!()
    }

    /// Adds a target and returns the fan-out for chaining.
    #[must_use]
    pub fn with(self, target: impl Notifier + Send + Sync + 'static) -> Self {
        let _ = target;
        todo!()
    }

    /// Number of targets.
    pub fn len(&self) -> usize {
        let _ = self;
        todo!()
    }

    /// `true` when there are no targets.
    pub fn is_empty(&self) -> bool {
        let _ = self;
        todo!()
    }

    /// Attempts delivery to every target, including those after a failure.
    /// Returns `Ok(n)` where `n` is the number of targets when all succeed,
    /// otherwise every failure as a [`FanOutError`].
    pub fn broadcast(&self, message: &str) -> Result<usize, FanOutError> {
        let _ = (self, message);
        todo!()
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
    let _ = (notifier, message);
    todo!()
}

/// Forwarding impl: a boxed notifier, including `Box<dyn Notifier>`, is itself a notifier.
impl<N: Notifier + ?Sized> Notifier for Box<N> {
    fn name(&self) -> &str {
        let _ = self;
        todo!()
    }

    fn notify(&self, message: &str) -> Result<(), NotifyError> {
        let _ = (self, message);
        todo!()
    }
}
