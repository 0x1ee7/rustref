# 19 — Performance: allocation budgets you can test

## Scenario

A log-ingestion service tails application logs, parses millions of lines a
day, aggregates per-target metrics, and ships compact binary headers to a
collector. Latency budgets are set by allocation counts, not guesses: the
parser must not touch the heap, the report writer must allocate once, and
hot-path lookups must not clone keys. This exercise makes those budgets
executable.

The test binary installs a counting `#[global_allocator]` (see
`tests/common/mod.rs`). Counts are per thread, so parallel tests don't
interfere. `count_allocations(|| …)` returns the value plus the number of
`alloc`, `alloc_zeroed` and `realloc` calls the closure made. Only the test
harness uses `unsafe`; your code in `src/` is entirely safe.

Allocation budgets are part of the correctness contract. A function that
returns the right answer but allocates where the spec says it must not fails.

## Part 1 — Zero-allocation log-line parser (`log_line`)

Line format: `<timestamp> <LEVEL> <target> <message>`, e.g.
`2026-11-10T12:00:00Z INFO api::http request completed status=200`.

- `Level { Trace, Debug, Info, Warn, Error }`: ordered least to most severe,
  `as_str()` returns the upper-case token.
- `LogLine<'a>` borrows all three text fields from the input.
- `parse_line(&str) -> Result<LogLine<'_>, ParseError>`:
  - Surrounding ASCII whitespace (including `\r\n`) is ignored.
  - Tokens are separated by any whitespace. The message is everything after
    the target, with internal spacing kept and trailing whitespace trimmed;
    it may be empty.
  - Level tokens are case-sensitive; an unknown token is
    `ParseError::UnknownLevel(String)`.
  - A missing field is `ParseError::MissingField(Field)`, reporting the first
    missing one (`Timestamp`, `Level`, `Target`).
  - Success allocates nothing.
- `parse_lines(&str) -> LogLines<'_>`: an iterator over the lines of a buffer.
  Blank lines are skipped. Bad lines are yielded as `Err` in place. Iterating
  a whole buffer allocates nothing.
- `ParseError` implements `Display` and `std::error::Error`. The `Display`
  output names the missing field or the offending level token.

Concepts: lifetimes on returned data, zero-copy slicing, `Option`/`Result`
plumbing with `?`, iterators that borrow, error enums with `Display`.

## Part 2 — Reused buffers and exact allocation budgets (`report`)

Row format (`write_row` output for one row, always newline-terminated):
`{name:<16} {count:>10} {total_micros:>20}\n`. Long names are not truncated.

- `Row<'a> { name: &'a str, count: u32, total_micros: u64 }`
- `write_row(&mut String, &Row) -> fmt::Result` appends one row. It never
  clears the buffer and, when the buffer has enough capacity, allocates nothing.
- `render_report(&[Row]) -> String` renders all rows. It pre-sizes the output
  with an upper bound per row, `max(name.len(), 16) + 1 + 10 + 1 + 20 + 1`,
  so that a non-empty report costs exactly one allocation, even at the
  maximum `u32`/`u64` widths. An empty report allocates nothing.

Concepts: `std::fmt::Write`/`write!` into `String`, `with_capacity` as a
budget, reusing a buffer across iterations, why a capacity bound must be an
upper bound.

## Part 3 — `Vec` discipline (`vectors`)

- `Entry { id: u32, expires_at: u64 }` (`Copy`).
- `widen_all(&[u32]) -> Vec<u64>` sizes the output from the input and extends
  from the exact-size iterator. Exactly one allocation for a non-empty input;
  an empty input allocates nothing.
- `prune_expired(&mut Vec<Entry>, now) -> usize` removes entries with
  `expires_at <= now` in place with `retain`, keeps order, returns the number
  removed, keeps the capacity, and allocates nothing.
- `live_entries(&[Entry], now) -> Vec<Entry>` returns a new vector of the live
  entries and leaves the input alone. It must allocate to build the new vector.
- Given the same input, `prune_expired` and `live_entries` produce the same
  survivors. The randomised test checks this.

Concepts: `with_capacity`, exact-size `extend`, `retain` vs collect-into-new,
`&mut Vec` vs `&[T]` in signatures, capacity as observable behaviour.

## Part 4 — `InlineVec<T, N>`, a small-vector without `unsafe` (`inline_vec`)

A vector that holds up to `N` elements inline (no heap) and moves everything
to a `Vec` on the push after that.

- `InlineVec::<T, N>::new()`, `Default`.
- `push(&mut self, T)`, `len`, `is_empty`.
- `is_spilled()` is `true` iff `len() > N`.
- `get(usize) -> Option<&T>`, bounds-checked.
- `iter() -> Iter<'_, T>` and `IntoIterator for &InlineVec<T, N>`, yielding
  `&T` in insertion order.
- Storing up to `N` elements (including `new`, pushes, and iteration)
  allocates nothing. The push that exceeds `N` spills all elements to the heap
  in order. `N = 0` spills on the first push.
- Every element is dropped exactly once, both inline and after spilling. Values
  are moved, never cloned.
- The crate stays `unsafe`-free. A natural representation is an enum of
  `[Option<T>; N]` plus a length, and `Vec<T>`. `[const { None }; N]` builds the
  array without `T: Copy`.

Concepts: const generics, enum-based state machines, `mem::replace` to move out
of `&mut self`, `Option` as a placeholder, `Drop` correctness, `IntoIterator`
for references.

## Part 5 — Borrowing strings: `Cow`, borrowed keys, `Box<str>`, `Arc<str>` (`strings`)

- `normalize_header(&str) -> Cow<'_, str>` lowercases ASCII letters only.
  `Borrowed` when nothing changes (zero allocations), `Owned` otherwise.
  Non-ASCII characters are left alone.
- `sanitize_label(&str) -> Cow<'_, str>` replaces spaces with `_`. `Borrowed`
  when there are no spaces.
- `HeaderMap`: keys are normalised names, values are `String`.
  - `insert(&mut self, name, value) -> Option<String>` returns the previous
    value. Replacing an existing key must not reallocate the key.
  - `get(&self, name) -> Option<&str>` is case-insensitive. A lowercase lookup
    allocates nothing: it borrows the normalised key and calls `HashMap::get`
    with a `&str`.
  - `len`, `is_empty`. Names that differ only by case are the same entry.
- `Interner`: `intern(&mut self, &str) -> Arc<str>` returns a shared handle.
  Re-interning an existing string allocates nothing and returns a pointer-equal
  `Arc`. `len`, `is_empty`.
- `Tag`: an immutable label backed by `Box<str>`. `Tag::new(&str)` makes one
  allocation for a non-empty label and none for an empty one. `as_str()`.

Concepts: `Cow` as "borrow unless you must own", `HashMap<String, _>::get(&str)`
via `Borrow`, `Box<str>` vs `String` for immutable data, `Arc<str>` interning,
`Entry`-free insertion.

## Part 6 — Layout: `size_of`, field order, niches, enums (`layout`)

- `WireHeader` is `#[repr(C)]` with fields `version: u8`, `timestamp_ms: u64`,
  `flags: u8`, `length: u16`, `seq: u32`. The field order in the stub is the
  naive one, and under `repr(C)` it is 24 bytes. Reorder the declaration so
  that the size is 16 bytes. The set of fields and their types must not change.
- `WireHeader::new(...)`, `to_bytes(&self) -> [u8; 16]`, `from_bytes([u8; 16])`.
  The wire format is explicit little-endian, field by field, at fixed offsets:
  version @0, flags @1, length @2..4, seq @4..8, timestamp_ms @8..16. Never
  transmute the struct, since padding bytes are uninitialised.
- `Link` stores `Option<NonZeroU32>`, with `0` meaning "no parent". `Link` is 4
  bytes and reuses the `NonZeroU32` niche. `Option<Link>` is 8 bytes: every `u32`
  is a valid id, so `Option` has no spare value for its tag.
- `Value { Null, Bool(bool), Int(i64), Float(f64) }` is 16 bytes, and
  `Option<Value>` is 16 bytes as well.
- `parse_scalar(&str) -> Option<Value>`: `null`, `true`/`false`, a decimal
  `i64`, or a finite `f64`. Anything else, including `NaN`, `inf`, and
  surrounding whitespace, gives `None`.
- `log_line::Level` is one byte, and `Option<Level>` is one byte.

Concepts: `size_of`/`align_of`, `repr(C)` vs `repr(Rust)`, padding and field
ordering, `NonZero*` niches, enum tag layout, explicit serialisation.

## Run

```powershell
cargo test -p ex19_performance                   # everything (all red at first)
cargo test -p ex19_performance part2             # one part
cargo solutions -p ex19_performance              # grade the reference answer
cargo clippy -p ex19_performance --all-targets
```

Stretch (untested):

- Measure bytes as well as calls in the counting allocator, and assert on peak
  live bytes for `render_report`.
- Replace `InlineVec`'s `Option` slots with `MaybeUninit`, add `unsafe`, and
  check it with Miri.
- Make `LogLine` fields `Cow<'a, str>` for unescaping, and measure when the
  borrowed path stays allocation-free.
- Add `#[repr(align(64))]` padding to a per-shard counter and show the effect
  on `size_of` and false sharing, using a real benchmark outside this suite.
- Replace the global counting allocator with a per-thread arena and assert the
  number of arena resets.
