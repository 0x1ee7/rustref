# 08 — Advanced traits and the object model

## Scenario

You are building the internal platform library for an ops team: alert fan-out,
a small data-access layer, a text-processing pipeline, a database connection
and HTTP request builder, and a streaming ingest path. Each module is a place
where a trait design choice has consequences for callers, for performance, and
for what the compiler will let you do later.

The exercises are about choosing between `dyn` and generics, associated items
and type parameters, blanket impls, supertraits and upcasting, typestate, and
GATs/RPITIT. Every Part has an API and behavioural spec in the tests. The
tests are the source of truth; the `todo!()` stubs in `src/` declare the
surface you must implement.

## Setup

Edit only files under `exercises/08_advanced_traits/`. Keep the public API from
the stubs. You may add private fields, helpers, and private modules. Do not add
dependencies: `serde`, `serde_json`, and `thiserror` are already available.

## Part 1 — trait objects vs generics (`dispatch`)

`Notifier` is a dyn-compatible trait: `name(&self)` and `notify(&self, message)`.
Two designs meet in `FanOut`, which owns heterogeneous targets, and in
`send_one`, which is generic over the target.

- `RecordingNotifier::new(name)` accepts every message and records it.
  `RecordingNotifier::failing(name)` rejects every message with
  `NotifyError::Unavailable { channel: name }` and records nothing.
  `sent()` returns the accepted messages in order.
- `notify` takes `&self`, so recording needs interior mutability. The notifier
  must stay usable from several threads at once.
- `FanOut::with(target)` adds a target and returns `self`. `len`/`is_empty`
  report the count. `broadcast(message)` returns `Ok(n)` with the target count
  when everything succeeds. Otherwise it returns `FanOutError`, which contains
  `attempted` and the `(channel, error)` pairs in registration order. Every
  target is attempted, even after a failure. The error's `Display` reads
  `"<failures>/<attempted> notifiers failed"`.
- `FanOut` is `Send + Sync` so one broadcaster can be shared across threads.
- `send_one<N: Notifier + ?Sized>` accepts `&dyn Notifier` as well as concrete
  types.
- `Box<N>` implements `Notifier` whenever `N` does, including `N = dyn Notifier`.
  This lets a boxed trait object be passed wherever a notifier is expected.

**Edge cases:** an empty `FanOut` broadcasts `Ok(0)`; a failing target among
healthy ones still lets the later targets run.

## Part 2 — associated types and consts vs generic parameters (`model`)

`Model` has `const TABLE`, `type Key: Copy + Ord + Display`, `key(&self)`, and
`columns()`. A model type fixes exactly one table and one key type, so these
belong on the trait as associated items.

- `create_table_sql::<M>()` renders `CREATE TABLE <TABLE> (<col> <type>, ...)`
  in declaration order. `Text` → `TEXT`, `Integer` → `INTEGER`,
  `Boolean` → `BOOLEAN`.
- `Repository<M>` is an in-memory table keyed by `M::Key`. `insert` fails with
  `RepoError::DuplicateKey { table, key }` and leaves the stored row unchanged.
  `get`, `remove`, `len`, `is_empty`, and `iter` behave as their names suggest.
  `iter` yields rows in ascending key order.
- `Repository` must work with borrowed keys (`&'static str`), not only `Copy`
  integers.
- `Validator<T: ?Sized>` is generic over the value type, so one `MaxLen` can
  check both `str` and `[T]`. `MaxLen` counts `str` length in chars, not bytes,
  and counts slice length in elements. A failure message reads
  `"length <n> exceeds maximum <max>"`.

**Edge cases:** multi-byte UTF-8 text must not be counted by byte length; a
duplicate insert must not overwrite.

## Part 3 — blanket impls, extension traits, sealed traits (`ext`)

- `JsonExt` is implemented by a single blanket impl for `T: Serialize + ?Sized`.
  Callers get `to_json` and `to_json_pretty` on structs, on `str`, and on
  slices. Pretty output uses two-space indentation. Serializer errors are
  returned, not panicked on: a `HashMap` with tuple keys cannot become a JSON
  object.
- `StrExt` adds `truncate_chars(max)`, which never splits a character, and
  `slugify()`. `slugify` lowercases ASCII alphanumerics, treats every other
  character (including non-ASCII letters) as a separator, collapses runs, and
  trims the edges.
- `ResultExt::tag_err(tag)` maps `Err(e)` to `Err("<tag>: <e>")` and passes `Ok`
  through unchanged.
- `SqlValue` is a sealed trait. It is implemented for `i64`, `bool`, `str`,
  `String`, and `Option<T>`. Text is single-quoted with `'` doubled, `bool`
  renders as `TRUE`/`FALSE`, `None` renders as `NULL`, and `Some` delegates to
  the inner value. Downstream crates cannot implement `SqlValue` because the
  supertrait `Sealed` lives in a private module.

**Edge cases:** `truncate_chars(0)` on an empty string is `""`; `slugify` of
all separators is `""`.

## Part 4 — supertraits, upcasting, `Any` maps, factories (`plugins`)

`Stage: Named + Send` is a pipeline step with `apply(&mut self, &str) -> String`.
`Named` is split out so a `Box<dyn Stage>` can be upcast to `Box<dyn Named>`.

- Built-ins: `upper`, `trim`, `truncate` (requires config key `max`, an
  integer ≥ 1, counted in chars), and `sequence` (stateful: prefixes each call
  with a 1-based counter, e.g. `1: a`, then `2: b`).
- `StageRegistry` maps names to boxed factories. `register` fails with
  `Duplicate` on a repeated name. `create` fails with `UnknownStage`, or with the
  factory's own `InvalidConfig`. Factories may capture state, so they are
  closures: `Fn(&StageConfig) -> FactoryResult + Send + Sync + 'static`.
  `names()` is sorted.
- `StageConfig` holds string key/value pairs. `with` replaces an existing value.
- `Pipeline` runs stages in order, feeding each output to the next. Stage state
  persists across `run` calls. A pipeline must be movable to another thread.
- `boxed_as_named` upcasts `Box<dyn Stage>` to `Box<dyn Named>`. Trait upcasting
  is stable from Rust 1.86.
- `Services` is a typed map holding at most one value per type: `insert`
  returns the previous value of that type, `get`/`get_mut`/`remove`/`contains`
  are keyed by `TypeId`, and distinct types never collide.

**Edge cases:** `truncate` with a missing, zero, or non-numeric `max` returns
`InvalidConfig { stage: "truncate", .. }`. A stage that is never registered
gives `UnknownStage` with the name.

## Part 5 — typestate, validated newtypes, `Deref` (`typestate`)

- `Endpoint::parse("host:port")` splits on the **last** `:`. The host must be
  non-empty. The port must be an integer in 1..=65535. Errors are
  `Malformed` or `InvalidPort`. `Display` prints `host:port`.
- `Connection<Disconnected>` → `connect()` → `Connection<Connected>`. Only
  `Connected` has `execute`, `executed`, and `disconnect`. A failed `connect`
  returns the disconnected connection together with `ConnectError::Refused`, so
  the caller can retry or reroute. Hosts ending in `.unreachable` are refused
  (this simulates a dead server). `execute` rejects blank statements with
  `EmptyStatement` and does not count them. A new session starts at zero.
- `RequestBuilder<NoUrl>` → `url()` → `RequestBuilder<HasUrl>` → `build()`.
  `build` exists only after a URL is set, so an unfinished request cannot
  compile to a `Request`. `header` is valid in any state. `url` accepts only
  absolute `http://` or `https://` URLs with a non-empty rest; other absolute
  schemes give `UnsupportedScheme`; anything else gives `Invalid`. Header
  lookup on `Request` is case-insensitive and returns the first match.
- `Email` is a validated newtype: no whitespace, exactly one `@`, non-empty
  local part, and a domain with a `.` that neither starts nor ends with one.
  Implement `FromStr`, `AsRef<str>`, `Borrow<str>`, and `Display`. Do **not**
  implement `Deref<Target = str>`: it would let every `&str` method bypass the
  validation boundary. `as_str()` is the read accessor. `Borrow<str>` lets a
  `HashMap<Email, _>` be queried with `&str`, which requires the derived `Hash`
  to agree with `str`'s hash.

**Edge cases:** `:5432` is malformed (empty host); `db:0` and `db:70000` are
invalid ports; `ftp://…` is an unsupported scheme, not an invalid URL.

## Part 6 — GATs, RPITIT, lazy `Deref` (`lending`)

- `LendingIterator` has a GAT, `type Item<'a> where Self: 'a`. Items borrow the
  iterator's own buffer, so one buffer is reused for every chunk.
- `ChunkReader<R: Read>::new(reader, NonZeroUsize)` yields
  `Option<io::Result<&[u8]>>`. Each `next` does one read. `ErrorKind::Interrupted`
  is retried silently. Other errors are yielded as `Some(Err(_))`. End of
  stream is `None`. An exact multiple of the chunk size gives no empty final
  chunk. Input can be empty.
- `Catalog` returns `impl Iterator<Item = &Entry> + '_` from `entries()`
  (RPITIT), with a default `total_size`. `MemoryCatalog` and `ShardedCatalog`
  implement it with concrete iterator types; nothing is boxed. `ShardedCatalog`
  chains its shards in order. Empty shards are fine.
- `largest(&catalog)` returns the entry with the greatest size. On a tie, the
  **first** one in catalog order wins. An empty catalog yields `None`.
- `Lazy<T, F>::new(init)` stores `init` without running it. The first access
  runs it exactly once, even when several threads race on that first access.
  `Deref` forces initialisation. `Lazy::force(&lazy)` returns the same
  instance as deref does. `Lazy::is_initialized(&lazy)` never triggers
  initialisation. Use associated functions for `force` and `is_initialized`
  so they never shadow methods on `T`.
- `ApiClient::new(base)` strips trailing `/`. `url_for(path)` joins with exactly
  one `/`, whatever slashes the inputs carry.

**Edge cases:** an `Interrupted` read must not reach the caller; a `Lazy` must
not run its initialiser just because it was constructed.

## Concepts exercised

- Dyn compatibility rules; `dyn Trait` vs generics; `?Sized` bounds; forwarding
  impls for `Box<N>`
- Interior mutability behind `&self` (`Mutex`); `Send`/`Sync` bounds on trait
  objects
- Associated consts and types vs generic parameters; `Ord`/`Display` bounds on
  associated types; borrowed keys
- Blanket impls over `?Sized` types; extension traits; sealed traits
- Supertraits; trait upcasting; `Box<dyn Any>` with `TypeId`; closure factories
  as `Box<dyn Fn + Send + Sync>`
- Typestate with `PhantomData`; consuming state transitions that return the
  value on failure; validated newtypes and `Borrow` vs `Deref`
- GATs for lending iterators; RPITIT; `OnceLock`-backed lazy values with
  `Deref`

## Run

```sh
cargo test -p ex08_advanced_traits                 # all parts (fail against stubs)
cargo test -p ex08_advanced_traits part2           # one part; prefix is part<N>_
cargo solutions -p ex08_advanced_traits            # reference solution, all green
```

## Stretch (untested)

- Give `FanOut` a `broadcast_concurrent` that uses scoped threads. Decide
  whether `Send + Sync` should be a bound on the method or on the type.
- Make `Repository` generic over the storage (`BTreeMap` vs `HashMap`) with a
  trait, and check which bounds leak into the public API.
- Add a `Validator` impl for `Email` and compose validators with a tuple impl.
- Replace `Box<dyn Stage>` in `Pipeline` with a generic `Vec<S>` and measure
  the trade-off: what can no longer be mixed in one pipeline?
- Write a `Lazy` variant that returns `Result` from its initialiser and
  caches only successes.
