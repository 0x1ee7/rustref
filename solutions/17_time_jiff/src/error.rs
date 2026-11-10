use jiff::civil;

/// Everything that can go wrong in this crate.
///
/// Marked `#[non_exhaustive]` so new failure modes can be added without a
/// breaking change.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TimeError {
    /// The IANA name is not in the bundled tz database.
    #[error("unknown time zone {0:?}")]
    UnknownZone(String),
    /// The wall time falls in a DST gap and the policy does not resolve it.
    #[error("wall time {0} does not exist in this zone (DST gap)")]
    Nonexistent(civil::DateTime),
    /// The wall time occurs twice (DST overlap) and the policy rejects it.
    #[error("wall time {0} is ambiguous in this zone (DST overlap)")]
    Ambiguous(civil::DateTime),
    /// Malformed or out-of-range input.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// Operation not valid in the current state (for example, stopwatch misuse).
    #[error("invalid state: {0}")]
    InvalidState(&'static str),
    /// Error reported by jiff (range overflow, parse failure, and similar).
    #[error(transparent)]
    Jiff(#[from] jiff::Error),
}
