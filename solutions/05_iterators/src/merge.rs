//! Part 7 — k-way merge and top-k with `BinaryHeap`.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

use jiff::Timestamp;

use crate::log::LogRecord;

/// Lazily merge streams that are each already sorted by `ts` (k-way merge, one heap
/// entry per stream). On equal timestamps the record from the earlier stream comes
/// first. Works on infinite streams. No stream is touched until the first `next()`.
pub fn merge_streams<I>(streams: impl IntoIterator<Item = I>) -> MergeStreams<I>
where
    I: Iterator<Item = LogRecord>,
{
    MergeStreams {
        streams: streams.into_iter().collect(),
        heap: BinaryHeap::new(),
        primed: false,
        pending: None,
    }
}

/// Heap entry: the head record of one stream.
struct Head {
    stream: usize,
    record: LogRecord,
}

impl Head {
    fn key(&self) -> (Timestamp, usize) {
        (self.record.ts, self.stream)
    }
}

impl PartialEq for Head {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Head {}

impl PartialOrd for Head {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Head {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

/// Iterator returned by [`merge_streams`].
pub struct MergeStreams<I> {
    streams: Vec<I>,
    /// Min-heap (via `Reverse`) of the current head of each live stream.
    heap: BinaryHeap<Reverse<Head>>,
    primed: bool,
    /// Stream whose emitted head must be replaced before the next pick. Deferring the
    /// refill keeps the pull count minimal: nothing is read ahead of the consumer.
    pending: Option<usize>,
}

impl<I: Iterator<Item = LogRecord>> MergeStreams<I> {
    fn refill(&mut self, stream: usize) {
        if let Some(record) = self.streams[stream].next() {
            self.heap.push(Reverse(Head { stream, record }));
        }
    }
}

impl<I: Iterator<Item = LogRecord>> Iterator for MergeStreams<I> {
    type Item = LogRecord;

    fn next(&mut self) -> Option<LogRecord> {
        if !self.primed {
            self.primed = true;
            for stream in 0..self.streams.len() {
                self.refill(stream);
            }
        }
        if let Some(stream) = self.pending.take() {
            self.refill(stream);
        }
        let Reverse(head) = self.heap.pop()?;
        self.pending = Some(head.stream);
        Some(head.record)
    }
}

/// The `k` servers with the most records, as `(server, count)`, highest count first.
/// Ties on count are ordered by server name ascending. Uses a bounded heap of size `k`
/// rather than sorting every server. `k == 0` → empty.
pub fn top_servers(records: impl IntoIterator<Item = LogRecord>, k: usize) -> Vec<(String, usize)> {
    if k == 0 {
        return Vec::new();
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    for record in records {
        *counts.entry(record.server).or_insert(0) += 1;
    }

    // Min-heap of the best `k` so far: the worst kept entry sits on top and is evicted.
    // Within equal counts, the greater name is "worse" (ties favour ascending names).
    let mut heap: BinaryHeap<Reverse<(usize, Reverse<String>)>> = BinaryHeap::with_capacity(k + 1);
    for (server, count) in counts {
        heap.push(Reverse((count, Reverse(server))));
        if heap.len() > k {
            heap.pop();
        }
    }

    let mut top: Vec<(String, usize)> = heap
        .into_iter()
        .map(|Reverse((count, Reverse(server)))| (server, count))
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top
}
