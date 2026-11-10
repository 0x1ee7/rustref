//! Data parallelism with scoped threads: no `'static` bounds, no `Arc`.

use std::panic;
use std::thread;

/// Maps `f` over `items` using at most `workers` scoped threads.
///
/// The input is split into contiguous chunks, one per thread, so the output
/// preserves input order. Worker panics are re-raised on the calling thread
/// with their original payload.
///
/// # Panics
///
/// Panics if `workers` is zero, or if `f` panics on any element.
pub fn parallel_map<T, R, F>(items: &[T], workers: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    assert!(workers >= 1, "workers must be >= 1");
    if items.is_empty() {
        return Vec::new();
    }
    let chunk_len = items.len().div_ceil(workers);
    let f = &f;
    thread::scope(|s| {
        let handles: Vec<_> = items
            .chunks(chunk_len)
            .map(|chunk| s.spawn(move || chunk.iter().map(f).collect::<Vec<R>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| match h.join() {
                Ok(part) => part,
                Err(payload) => panic::resume_unwind(payload),
            })
            .collect()
    })
}
