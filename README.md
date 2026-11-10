# rustref — Rust refresher, test-driven

Real-world Rust exercises for an experienced engineer. The Rust sibling of
`pyref`: same philosophy (one topic per directory, a spec, failing tests, a
reference answer), designed around production Rust — ownership and borrowing,
traits, enums and pattern matching, `Result`-based errors, iterators, RAII,
`Send`/`Sync` — plus Rust-specific topics (ownership & lifetimes, macros,
performance).

## Setup

Requires stable Rust ≥ 1.88 (edition 2024). Dependencies (tokio, serde, axum,
rusqlite, jiff, tracing, …) are declared once in the workspace.

```powershell
cargo test --workspace --no-fail-fast     # everything (all red at first)
```

## Workflow

```powershell
cargo test -p ex05_iterators              # one topic
cargo test -p ex05_iterators part2        # one part (test fns are part<N>_...)
cargo solutions -p ex05_iterators         # grade the reference answer instead of yours
cargo clippy -p ex05_iterators --all-targets
```

1. Read `exercises/<topic>/README.md` — it, together with `tests/`, is the spec.
2. `src/` is yours. `lib.rs` declares the public API (Rust needs it to compile)
   with `todo!()` bodies; add private fields, modules and helpers freely.
3. Iterate until green, then compare with `solutions/<topic>/src`.

## How the harness works

- Tests are black-box integration tests in `exercises/<topic>/tests/`; each
  `todo!()` panics inside its own test only.
- `solutions/` is a second Cargo workspace. Each member has the same package
  name as its exercise, the reference `src/`, and `[[test]]` targets pointing at
  `../../exercises/<topic>/tests/*.rs` — so the same tests grade both.
- `cargo solutions` is an alias (`.cargo/config.toml`) for
  `cargo test --manifest-path solutions/Cargo.toml --target-dir target`; run it
  from the repo root. Both workspaces share `target/`.

With the stubs every test fails; with `cargo solutions` every test passes.

> `solutions/` is the answer key. Don't open it until your own version is green.

## Exercises

| # | Topic | What you build | Tests |
|---|---|---|---|
| 01 | [Collections: ops_toolkit](exercises/01_collections) | TTL + LRU response cache · Per-tenant sliding-window rate limit · Streaming top-K over sales lines · Layered configuration with deep merge · Money, reconciliation, price tiers, SLA bands · Records, grouping, lazy maps | 63 |
| 02 | [Function parameters](exercises/02_function_parameters) | Headers: borrowed vs owned, AsRef and Into · Request builder: &mut self with deferred errors · Encoding and variadic-style arguments · Dependency injection: a trait, a closure predicate, and dyn · Typestate SQL builder (consuming self) · PATCH: absent vs null vs a value | 64 |
| 03 | [Modules, visibility, scope and drop order](exercises/03_modules_and_visibility) | module tree, facades and id factories · private fields and constructors · sealed traits and report formats · block scope, shadowing and drop order · RAII guards and thread-local state · statics versus explicit context · formula evaluator | 92 |
| 04 | [Functions deep dive](exercises/04_closures_and_functions) | closures and the Fn traits · returning closures, fn pointers and callback registries · higher-order retry and timed · memoisation and a TTL cache · middleware chains · rate limiting with interior mutability · command registry | 68 |
| 05 | [Iterators, containers and lazy pipelines](exercises/05_iterators) | The iterator protocol and container traits · A lazy log pipeline · A lazily paginated API client · Resource-owning iterators and batched writes · Collecting, extending, and an RAII finaliser · Adapters and extension traits · K-way merge and top-k | 82 |
| 06 | [Functional patterns in Rust](exercises/06_functional_patterns) | Enums as ADTs and the cart reducer · Composition and closures · Patterns: an event-sourced account · Folds that stop at the first bad event · Option/Result combinators and ? · Accumulate-all-errors validation · ETL pipeline and ranked report | 71 |
| 07 | [Structs, traits and value types](exercises/07_structs_and_traits) | Money: exact minor units, checked arithmetic, operators · Sku newtype and Product: validated constructors, setters, builders · Account trait: shared default methods, composition, overrides · ShoppingCart: collections, Index, IntoIterator, atomic batches · Employee: validated newtypes, TryFrom, FromStr, serde · Notifier trait objects, transports, dispatch · Version: SemVer precedence, manual Eq/Hash/Ord, typed bumps | 67 |
| 08 | [Advanced traits and the object model](exercises/08_advanced_traits) | trait objects vs generics · associated types and consts vs generic parameters · blanket impls, extension traits, sealed traits · supertraits, upcasting, Any maps, factories · typestate, validated newtypes, Deref · GATs, RPITIT, lazy Deref | 63 |
| 09 | [Error handling](exercises/09_error_handling) | Typed error enums and #[from] · Source chains and anyhow at the edge · Propagation and logs around a fallible step · RAII transactions with savepoints · Scoped guards and resource cleanup · Panics as errors at a plugin boundary · Batches, classification and retry | 63 |
| 10 | [Logging and tracing with tracing](exercises/10_logging_tracing) | Events, fields and #[instrument] · Spans, context and request ids · Filtering with EnvFilter · JSON output · Custom Layers · Composing a stack and fanning out work | 83 |
| 11 | [Ownership, borrowing, lifetimes and smart pointers](exercises/11_ownership_and_lifetimes) | Cow escaping · zero-copy INI parser · splitting borrows and moving out of &mut · an org chart with Rc, RefCell, Cell and Weak · string interners · arena dependency graph with cached order · copy-on-write snapshots and cross-thread counting | 81 |
| 12 | [Files and serde](exercises/12_files_and_serde) | Streaming hashes, checksums, scanning · Diffing and applying changes safely · Serde wire formats · Atomic writes · CSV reports · Binary snapshot with a checksum | 66 |
| 13 | [Database operations with rusqlite](exercises/13_database_rusqlite) | Connections and type mappings · Migrations · Unit of work · Repositories · Bulk import · Pagination · Reports and loading | 63 |
| 14 | [Exercise 14: Threads and Synchronisation](exercises/14_threads_and_sync) | data parallelism and link checking · a bounded downloader · atomics and shared state · mutexes, lock ordering and poisoning · LRU cache and single-flight loading · a thread pool with a bounded queue · periodic jobs and a bounded stop | 74 |
| 15 | [Async Rust with Tokio](exercises/15_async_tokio) | Concurrent fan-out with per-source timeouts · Cancellation tokens, cancel-safe receiving, shielded work · Bounded concurrency and blocking IO · Pub/sub with a lagging-subscriber policy · Worker pool with per-job replies · Lazy pagination and an async resource pool · Retry, circuit breaker, and rate limiting | 78 |
| 16 | [Testing a billing module](exercises/16_testing) | Line items, money and totals · Discounts and tax · Dates, timestamps and due dates · Charging and retries · Receipts and logs · CSV export | 90 |
| 17 | [Dates and times with jiff](exercises/17_time_jiff) | instants, zones, explicit disambiguation · timestamp parsing and epoch conversion · wall-clock vs elapsed arithmetic · calendars: month arithmetic, billing, business days · recurring meetings with an explicit DST policy · booking slots across time zones · injected clocks, humanized time, monotonic stopwatch | 76 |
| 18 | [Deployment: a container-ready HTTP service](exercises/18_deployment) | Configuration · HTTP API · Graceful shutdown · Command-line interface · Build metadata · Container image | 76 |
| 19 | [Performance: allocation budgets you can test](exercises/19_performance) | Zero-allocation log-line parser · Reused buffers and exact allocation budgets · Vec discipline · `InlineVec<T, N>`, a small-vector without unsafe · Borrowing strings: Cow, borrowed keys, `Box<str>`, `Arc<str>` · Layout: size_of, field order, niches, enums | 54 |
| 20 | [Declarative macros](exercises/20_macros) | Collections: count!, hashmap!, btreeset! · string_enum!: an enum with a text form · newtype!: opt-in wrappers · bail! and ensure! · routes!: a validated route table · assert_matches!: pattern assertions that return bindings | 54 |

Topic notes:

- **16 Testing** is inverted: you write `suite/billing_suite.rs` against the provided
  `billing/billing.rs`; `tests/grader.rs` builds throwaway crates (baseline + 20 mutants,
  `cargo test --offline`, target dir `target/ex16-mutants`) and requires your suite to
  kill every mutant (~45 s). Its count above includes the reference suite's 68 tests.
- **18 Deployment** — you also own `Dockerfile`, `.dockerignore` and `build.rs`.
- **19 Performance** installs a counting `#[global_allocator]` in the test binaries to
  enforce allocation budgets.
- **20 Macros** stubs expand to `panic!`, so the stub build prints unreachable-code
  warnings in the tests; they disappear once the macros are implemented.

