//! Part 7 — k-way merge and top-k with `BinaryHeap`.

use crate::log::LogRecord;

/// Lazily merge streams that are each already sorted by `ts` (k-way merge, one heap
/// entry per stream). On equal timestamps the record from the earlier stream comes
/// first. Works on infinite streams. No stream is touched until the first `next()`.
pub fn merge_streams<I>(_streams: impl IntoIterator<Item = I>) -> MergeStreams<I>
where
    I: Iterator<Item = LogRecord>,
{
    todo!()
}

/// Iterator returned by [`merge_streams`].
pub struct MergeStreams<I> {
    _streams: Vec<I>,
    _primed: bool,
}

impl<I: Iterator<Item = LogRecord>> Iterator for MergeStreams<I> {
    type Item = LogRecord;

    fn next(&mut self) -> Option<LogRecord> {
        todo!()
    }
}

/// The `k` servers with the most records, as `(server, count)`, highest count first.
/// Ties on count are ordered by server name ascending. Uses a bounded heap of size `k`
/// rather than sorting every server. `k == 0` → empty.
pub fn top_servers(_records: impl IntoIterator<Item = LogRecord>, _k: usize) -> Vec<(String, usize)> {
    todo!()
}
