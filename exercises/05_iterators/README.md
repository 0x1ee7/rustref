# 05 — Iterators, containers and lazy pipelines

## Scenario

A multi-region ingestion service receives log lines from a fleet of servers. Everything
is lazy: records are parsed as they are consumed, pages of an upstream API are fetched
only when pulled, large files are read in batches, and statistics are folded without
being stored. You build the pieces that a production ingestion path is made of.

Crate: `ex05_iterators` (`exercises/05_iterators/`). Stubs live in `src/`, black-box
tests in `tests/`.

## Part 1 — The iterator protocol and container traits

Implement `Countdown`, a one-shot countdown from `start` down to 1, and `Dataset<T>`, a
re-iterable snapshot of records.

- `Countdown::new(start: u32)`: yields `start, start-1, ..., 1`. `new(0)` is empty.
  Double-ended (`rev()` counts up), exact-size, fused. `u32::MAX` must not overflow.
- `Dataset<T>`: `new(impl IntoIterator<Item = T>)` snapshots the records. Provide `len`,
  `is_empty`, `iter`, `iter_mut`. Implement `FromIterator`, `Extend`, and `IntoIterator`
  for `Dataset<T>`, `&Dataset<T>` and `&mut Dataset<T>`.
- Edge cases: borrowing passes are independent; the owned form consumes the dataset;
  generators work as input.

## Part 2 — A lazy log pipeline

Parse, filter, and enrich log lines without collecting them.

Line format: `<rfc3339-timestamp-with-offset> <LEVEL> <server> <message...>`

- `Level` (`Debug < Info < Warning < Error < Critical`): `ALL`, `is_error`, `Display`
  (wire spelling), and `FromStr` (exact upper-case spelling only; errors with
  `UnknownLevel`).
- `parse_line(&str) -> Result<LogRecord, LogParseError>`: trims surrounding whitespace,
  keeps the message's internal spacing. Naive timestamps (no offset) are rejected.
  `FieldCount` reports how many fields were found.
- `parse_records(lines)`: lazy. Blank lines are skipped; errors carry the 1-based line
  number, counting blank lines.
- `read_records(BufRead)`: lazy line-by-line reading; must not read ahead of the
  consumer. Returns `ReadError` (parse or I/O).
- `filter_level(records, minimum)`: `Result` returned eagerly, so an unknown minimum is
  reported at call time.
- `enrich(records, &HashMap)`: sets `region`, defaulting to `"unknown"`. Lazy.
- `first_error(records)`: stops consuming at the first `Error`/`Critical`.
- `parse_attrs(&str)`: `key=value` pairs, double-quoted values with `\"` and `\\`
  escapes, `a=` is empty. Errors report the byte offset of the offending token.

## Part 3 — A lazily paginated API client

The client never talks to a network; it takes a `fetch` closure from cursor to page.

- `pages(fetch, start)`: yields `Result<Page<T>, PaginateError<E>>`, one per page.
  Fetches only when pulled. `start` is the first cursor requested (`None` = first page).
- `items(fetch, start)`: flattens pages, skipping empty ones.
- Errors: a fetch failure is yielded once and the iterator finishes. A repeated cursor
  is detected *before* it is requested again and yields `PaginationLoopError`.
- A finished client makes no further requests.

## Part 4 — Resource-owning iterators and batched writes

- `read_batches(open, size)`: yields `Vec<String>` batches of raw lines (newline kept).
  `open` runs on the first `next()`, not at construction. The resource is dropped as
  soon as the input is exhausted, and when the iterator is dropped early. An open
  failure is reported once. Panics with `"batch size must be at least 1"` when
  `size == 0`.
- `write_in_batches(rows, size, write)`: splits rows into batches, sums the counts
  `write` returns, and stops at the first error. Pulls only what the failing batch
  needs. Same zero-size panic.

## Part 5 — Collecting, extending, and an RAII finaliser

- `Stats { count, mean, minimum, maximum }`: `Default` is the empty state (`mean` etc.
  are `None`).
- `RunningStats`: `new`, `with_on_close(FnOnce(Stats))`, `push`, `snapshot`, `reset`
  (keeps the finaliser), `Extend<f64>`, `Default`. The mean must stay accurate at large
  offsets (use Welford's update).
- `FromIterator<f64> for Stats`.
- The finaliser runs exactly once, from `Drop`, including on early return and on unwind.

## Part 6 — Adapters and extension traits

- `IterExt::chunked(size)`: `Vec`s of up to `size`; lazy over infinite input; exact
  `size_hint`. Panics with `"batch size must be at least 1"` on zero.
- `IterExt::moving_average(window)`: mean of each full window; emits nothing until the
  first full window; lazy. Panics with `"window must be at least 1"` on zero.
- `leading_below(values, threshold)`: stops at the first breach without pulling past it.
- `running_peak(values)`: running maximum; empty in, empty out.
- `gaps(&[Timestamp])`: adjacent differences; fewer than two timestamps → nothing.
- `group_by_server(&[LogRecord])`: runs of *consecutive* records sharing a server.
  Non-adjacent records of the same server are separate groups.
- `retry_delays(base, cap)`: infinite exponential backoff clamped to `cap`; the first
  delay is `base` clamped; never overflows.

## Part 7 — K-way merge and top-k

- `merge_streams(streams)`: lazy k-way merge of streams each sorted by `ts`. Works on
  infinite streams. Equal timestamps: the earlier stream first. No stream is touched
  until the first `next()`, and the merge never pulls more from a stream than the
  consumer needs.
- `top_servers(records, k)`: `(server, count)` with the `k` highest counts, ties ordered
  by server name ascending. Use a bounded heap of size `k`, not a full sort. `k == 0`
  gives an empty result.

## Concepts exercised

- `Iterator`, `DoubleEndedIterator`, `ExactSizeIterator`, `FusedIterator`, `size_hint`
- `IntoIterator` for owned, shared, and mutable forms; `FromIterator`; `Extend`
- Adapters as zero-cost lazy pipelines: `scan`, `take_while`, `chunk_by`, `successors`
- Fallible iteration: `Result` items, `collect` into `Result`, eager vs lazy validation
- Ownership of resources inside iterators (`BufRead`, `Drop`, early `break`)
- RAII finalisers, panics during unwind
- Extension traits (`IterExt`) with blanket impls
- `BinaryHeap` with `Reverse`, custom `Ord` for heap entries
- `Timestamp` and `Duration` arithmetic without overflow
- Error types: `Display`, `source()`, `From` for `?`

## Run

```sh
cargo test -p ex05_iterators                 # all parts (fail against the stubs)
cargo test -p ex05_iterators part2           # one part
cargo solutions -p ex05_iterators            # reference solution
```

## Stretch ideas (untested)

- Make `moving_average` O(1) per value without accumulating float drift (e.g. Kahan
  summation, or recompute the sum every `window` steps).
- Let `pages` resume from a saved cursor after an error without restarting the sequence.
- Tie-break `merge_streams` by server name as a secondary key, and document the choice.
- Replace `top_servers` with a streaming approximation (Space-Saving) and compare memory.
