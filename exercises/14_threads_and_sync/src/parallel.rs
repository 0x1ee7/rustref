//! Part 1: data parallelism with scoped threads.

/// Maps `f` over `items` using at most `workers` scoped threads.
///
/// Output order matches input order. Requirements:
/// - empty input returns an empty `Vec` without calling `f`;
/// - `workers == 0` panics with a message mentioning `workers`;
/// - a panic inside `f` propagates to the caller with its original payload;
/// - `f` runs on up to `workers` distinct threads (chunked, not per-item).
pub fn parallel_map<T, R, F>(items: &[T], workers: usize, f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    let _ = (items, workers, f);
    todo!()
}
