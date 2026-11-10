# 03 · Modules, visibility, scope and drop order

A small internal platform crate: an id service, a catalogue with validated SKUs,
a ledger that cannot be driven into an invalid state, a report renderer whose
formats cannot be extended from outside, a session whose teardown order matters,
a transactional store, a test clock, a plugin registry with a process-wide
instance, and a formula evaluator. Implement everything under `src/`. Tests ship
failing; a Part is done when its tests pass.

The module layout, `pub` / `pub(crate)` / private boundaries and the constructor
signatures are the contract. Decide what is public before you write bodies:
a field that callers may set is a design decision, not a convenience.

Run the tests from the repository root:

```
cargo test -p ex03_modules_and_visibility
cargo test -p ex03_modules_and_visibility part2
```

## Part 1: module tree, facades and id factories (`src/lib.rs`, `src/ids.rs`)

- `lib.rs` declares the module tree and a `prelude` module that re-exports the
  common types. Callers should be able to write `use ex03_modules_and_visibility::prelude::*;`
  and reach `Sku`, `Ledger`, `Table`, `Format`, `Store`, `Registry`, `Env`,
  `evaluate`, `id_factory` and `IdFactory`.
- `id_factory(prefix: &str, width: usize) -> Result<IdFactory, IdError>` returns
  a boxed `FnMut() -> String + Send`. Each call yields `<prefix>-<n>`, where `n`
  starts at 1, is zero-padded to `width`, and is never truncated.
- Validation order: empty prefix, then characters outside `[A-Za-z0-9_-]`,
  then zero width.
- `IdError` has `EmptyPrefix`, `InvalidPrefix(String)` and `ZeroWidth`, and
  implements `Display` and `Error`.

Behaviour and edge cases: two factories are independent counters; a number wider
than `width` is printed in full; the factory is `Send` so it can move to a worker
thread.

Concepts exercised: the module tree and the `prelude` facade pattern, a boxed
`FnMut` as a stateful generator, `Send` bounds on trait objects, error
precedence in validation.

## Part 2: private fields and constructors (`src/catalog/`, `src/ledger.rs`)

- `Sku::new(raw: &str) -> Result<Sku, SkuError>` uppercases ASCII, then checks:
  length 3 to 12 characters, only `[A-Z0-9-]`, no leading or trailing `-`, no `--`.
  Each rule has its own `SkuError` variant.
- `Sku` keeps its string in a private field. `as_str`, `Display`, `FromStr`
  (`"ab".parse::<Sku>()`) and `AsRef<str>` expose it without exposing the field.
- `Ledger::new(opening: i64)` rejects negative openings. `balance`, `deposit` and
  `withdraw` each return the new balance. Deposits and withdrawals must be
  positive. Withdrawals may not exceed the balance. Overflow is reported as an
  error, not a panic.
- Every failed operation leaves the balance unchanged.

Concepts exercised: private fields as the enforcement point for invariants,
`FromStr` and `AsRef` for ergonomic construction, checked arithmetic
(`checked_add`), typed error enums with structured payloads.

## Part 3: sealed traits and report formats (`src/report/`)

- `Table::new(headers: Vec<String>, rows: Vec<Vec<String>>)` checks that the
  header is non-empty and every row matches its width. `headers()` and `rows()`
  return borrowed slices.
- `trait Format` is sealed through a private supertrait. Downstream crates can
  call it but cannot implement it. `name()` returns a lowercase identifier.
  `render(&Table) -> String` produces the output.
- `Csv`, `Json` and `Text` are unit structs implementing `Format`.
  - CSV: RFC-4180 style, `\n` line endings, quote fields containing `,`, `"`,
    `\n` or `\r`, and double embedded quotes.
  - JSON: an array of objects keyed by header, compact, with `\"`, `\\`, `\n`,
    `\r`, `\t` escapes and `\u00xx` (lowercase hex) for other control characters.
  - Text: columns joined by `" | "`, each column padded to its widest character
    count, a dashed separator line, trailing spaces trimmed on each line.
- `format_by_name(&str) -> Option<Box<dyn Format>>` matches ignoring ASCII case.

Concepts exercised: the sealed-trait pattern, trait objects returned from a
factory, `Vec<Vec<String>>` ownership at construction, character-count width
(not byte length), escaping rules.

## Part 4: block scope, shadowing and drop order (`src/drop_order.rs`)

The helpers log every drop into a shared `Sink`, so the tests check the exact
order.

- `Trace` logs `drop <label>` when dropped. `Session` has three `Trace` fields
  declared in the order `conn`, `cache`, `audit`. Fields drop in declaration
  order.
- `scoped_locals`: inner block drops before the outer locals. Logs
  `drop inner`, `drop c`, `drop a` and returns 3.
- `shadowed`: shadowing does not drop the earlier binding. Both bindings drop at
  end of scope in reverse declaration order. Returns 6, logs
  `end of scope`, `drop second`, `drop first`.
- `tail_temporary`: edition 2024 drops block-tail temporaries before the block's
  locals. Returns 4, logs `drop temp`, `drop local`.
- `match_scrutinee`: the scrutinee temporary lives until the end of the `match`.
  Logs `arm: ok` or `arm: err`, then `drop probe`.
- `if_let_scrutinee`: edition 2024 drops the scrutinee temporary before the
  `else` branch. Present logs `then 7`, `drop guard`, returns 7. Absent logs
  `drop guard`, `else`, returns 0.

Concepts exercised: `Drop` and RAII ordering, struct field drop order, shadowing
versus moving, temporary lifetimes in tail expressions, `match` and `if let`
scrutinee rules, the edition 2024 changes.

## Part 5: RAII guards and thread-local state (`src/store.rs`, `src/clock.rs`)

- `Store::begin(&mut self) -> Transaction<'_>` borrows the store exclusively.
  `put` and `delete` stage changes. `get` reads the staged view (staged writes
  and deletes shadow committed data). `commit(self) -> usize` applies the staged
  keys and returns how many were applied. `rollback(self)` discards them.
- Dropping a `Transaction` without committing counts as a rollback. Commit
  counts as a commit. Both counters are visible on `Store`.
- `clock::now_secs()` returns the innermost `ClockOverride` on this thread, or
  the system clock. `ClockOverride::freeze(at)` is an RAII guard. Dropping it
  restores the previous state, and guards nest. `override_depth()` reports how
  many are active on this thread.
- `ClockOverride` is `!Send`. Overrides do not leak to other threads.

Concepts exercised: RAII guards with commit-or-rollback semantics, borrowing
through a guard (`Transaction<'_>`), `thread_local!` with `RefCell`, a `!Send`
guard via `PhantomData<*const ()>`, nested scoped state.

## Part 6: statics versus explicit context (`src/registry/`)

- `Plugin: Send + Sync` has `name(&self) -> &str` and
  `run(&self, input: &str) -> Result<String, PluginError>`.
- `FnPlugin::new(name, closure)` adapts a closure into a `Plugin`.
- `Registry` uses interior mutability, so every method takes `&self`. It can be
  shared behind an `Arc`, or used as a `static`.
  - `register<P: Plugin + 'static>` validates the name. Names match
    `[a-z][a-z0-9_-]*` with length 1 to 32. Duplicates are rejected.
  - `unregister`, `get` (returns `Arc<dyn Plugin>`), `names` (sorted),
    `len`, `is_empty`, `clear`.
  - `run_all(&self, input)` runs every plugin in name order. It snapshots the
    plugins under the lock and runs them outside it, so a plugin may call back
    into the registry without deadlocking.
- `registry::global::global() -> &'static Registry` uses `LazyLock`.
  `boot_id() -> u64` uses `OnceLock`: it is chosen once, is nonzero, and is
  stable across calls.

Concepts exercised: `Send + Sync` bounds, interior mutability with `Mutex`,
`Arc<dyn Trait>` handles, lock scope discipline, `LazyLock` and `OnceLock`,
the trade-off between a global and explicit context.

## Part 7: formula evaluator (`src/formula/`)

A spreadsheet-style evaluator, split into private modules with a small public
surface re-exported from `formula/mod.rs`.

- `tokenize(&str) -> Result<Vec<Token>, FormulaError>`. Identifiers are
  uppercased. Whitespace is discarded. Unknown characters report their byte
  position. Malformed numbers are errors.
- `parse(&str) -> Result<Expr, FormulaError>` is a Pratt parser. A single leading
  `=` is accepted and ignored. Precedence, lowest to highest: `+ -`, `* / %`,
  `^` (right-associative), prefix `- +`. Prefix operators bind tighter than `^`,
  so `-2^2` is `(-2)^2` = 4.
- `Expr::references() -> BTreeSet<String>` returns the cell names referenced,
  sorted and deduplicated. Function names are excluded.
- `Env` holds cell values and functions. It is passed explicitly, with no global
  state. `Env::with_builtins()` installs `ABS` (1 argument), `MIN` and `MAX`
  (at least 1), `SUM` (at least 0), and `ROUND` (1 or 2 arguments, half away
  from zero, digits defaults to 0). `set_cell` validates names. `cell` looks up
  a value, case-insensitively.
- `evaluate(src: &str, env: &Env) -> Result<f64, FormulaError>`:
  - Function lookup happens before arguments are evaluated, so an unknown
    function is reported first.
  - `%` is Rust remainder. Division or remainder by zero is `DivisionByZero`.
  - A non-finite result is `NotFinite`.
  - Unknown cells are `UnknownCell`. Wrong arity is `Arity`.
- `FormulaError` is `#[non_exhaustive]`, so downstream matches need a wildcard arm.

Concepts exercised: module privacy with `pub(super)`, a Pratt parser, `Box`ed
recursive enums, `BTreeSet` for deterministic output, explicit environments
instead of globals, `#[non_exhaustive]` error enums, `f64` edge cases.

## Concepts exercised (overall)

Modules and `pub` / `pub(crate)` / private boundaries, facades and `prelude`,
private fields as invariant guards, `FromStr` and `AsRef`, sealed traits,
`Drop` order and the edition 2024 temporary rules, RAII guards, `thread_local!`,
`Send` and `Sync`, `LazyLock` and `OnceLock`, interior mutability, and
`#[non_exhaustive]`.

## Stretch (untested)

- Replace `Registry`'s `Mutex` with an `RwLock`. Measure whether `run_all`
  benefits.
- Add `Format::render_to(&self, &Table, &mut impl fmt::Write)` and discuss why it
  is not object safe as written.
