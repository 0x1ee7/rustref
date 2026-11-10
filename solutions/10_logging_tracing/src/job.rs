//! Part 6 — a background job that fans out to worker threads while keeping one
//! span tree.

use std::num::NonZeroUsize;

use tracing::{debug, info_span, instrument, warn};

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
#[instrument(skip(amounts_cents, workers))]
pub fn settle_batch(batch_id: &str, amounts_cents: &[i64], workers: NonZeroUsize) -> SettlementReport {
    let parent = tracing::Span::current();
    let dispatch = tracing::dispatcher::get_default(|d| d.clone());
    let chunk_len = amounts_cents.len().div_ceil(workers.get()).max(1);

    let (settled, rejected, total) = std::thread::scope(|scope| {
        let handles: Vec<_> = amounts_cents
            .chunks(chunk_len)
            .enumerate()
            .map(|(index, chunk)| {
                let dispatch = dispatch.clone();
                let parent = parent.clone();
                scope.spawn(move || {
                    tracing::dispatcher::with_default(&dispatch, || {
                        parent.in_scope(|| {
                            let span = info_span!("settle.worker", worker = index);
                            let _guard = span.enter();
                            process_chunk(chunk)
                        })
                    })
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("settlement worker panicked"))
            .fold((0, 0, 0i64), |(s, r, t), (cs, cr, ct)| (s + cs, r + cr, t + ct))
    });

    SettlementReport {
        batch_id: batch_id.to_owned(),
        settled,
        rejected,
        total_cents: total,
    }
}

/// Returns `(settled, rejected, total)` for one chunk.
fn process_chunk(chunk: &[i64]) -> (usize, usize, i64) {
    let mut settled = 0;
    let mut rejected = 0;
    let mut total = 0;
    for &amount in chunk {
        if amount > 0 {
            debug!(amount_cents = amount, "settled");
            settled += 1;
            total += amount;
        } else {
            warn!(amount_cents = amount, "rejected");
            rejected += 1;
        }
    }
    (settled, rejected, total)
}
