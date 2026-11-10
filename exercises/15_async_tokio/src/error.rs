//! Configuration errors shared by every part.

use thiserror::Error;

/// An argument violated a constructor's contract.
///
/// Every part validates eagerly: a bad configuration never reaches a task, a timer or a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// `aggregate` needs a per-call timeout greater than zero.
    #[error("per-call timeout must be greater than zero")]
    ZeroTimeout,
    /// Concurrency limits (`BoundedFetcher`, `read_files`, `ResourcePool::new`) must be at least 1.
    #[error("concurrency limit must be at least 1")]
    ZeroLimit,
    /// `WorkerPool::start` needs at least one worker.
    #[error("worker count must be at least 1")]
    ZeroWorkers,
    /// `Broker::new` needs room for at least one message.
    #[error("broadcast buffer must hold at least 1 message")]
    ZeroBuffer,
    /// A page budget of zero would fetch nothing.
    #[error("max_pages must be at least 1")]
    ZeroMaxPages,
    /// `RetryPolicy::new` needs at least one attempt.
    #[error("attempts must be at least 1")]
    ZeroAttempts,
    /// `CircuitBreaker::new` needs a threshold of at least 1.
    #[error("failure threshold must be at least 1")]
    ZeroFailureThreshold,
    /// Token rate must be finite and strictly positive.
    #[error("rate must be finite and greater than zero")]
    InvalidRate,
    /// Bucket capacity must be finite and strictly positive.
    #[error("capacity must be finite and greater than zero")]
    InvalidCapacity,
    /// A token request must be finite, positive and no larger than the capacity.
    #[error("token request must be finite, positive and at most the capacity")]
    InvalidTokens,
}
