//! Part 6 — a background job that fans out to worker threads while keeping one
//! span tree.

use std::num::NonZeroUsize;

/// Outcome of settling one batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementReport {
    pub batch_id: String,
    /// Amounts `> 0` that were settled.
    pub settled: usize,
    /// Amounts `<= 0` that were rejected.
    pub rejected: usize,
    /// Sum of the settled amounts.
    pub total_cents: i64,
}

/// Settles `amounts_cents` using up to `workers` threads.
///
/// Behaviour (Part 6):
/// - the input is split into contiguous chunks of `ceil(len / workers)` items;
///   worker `i` (0-based) processes chunk `i`;
/// - the report is the same whatever the worker count;
/// - the call runs inside a span named `settle_batch` with a `batch_id` field
///   (Display) and the arguments other than `batch_id` skipped;
/// - each worker runs inside a span named `settle.worker` with a `worker`
///   field, as a child of `settle_batch`;
/// - per amount: DEBUG `"settled"` with `amount_cents` for a positive amount;
///   WARN `"rejected"` with `amount_cents` for a non-positive one;
/// - events emitted on worker threads must reach the same subscriber as the
///   caller and keep the `settle_batch` → `settle.worker` span context.
pub fn settle_batch(batch_id: &str, amounts_cents: &[i64], workers: NonZeroUsize) -> SettlementReport {
    let _ = (batch_id, amounts_cents, workers);
    todo!()
}
