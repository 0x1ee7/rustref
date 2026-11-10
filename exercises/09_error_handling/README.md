# 09 — Error handling

## Scenario

You are maintaining a small service host. It loads a JSON config at startup, runs
multi-step deployment jobs inside transactions, guards environment and file
changes with RAII, isolates third-party plugins from panics, and runs batches of
network-ish tasks that must be retried selectively. Every layer reports failures
differently, and the boundary (`main`) must still produce a clear message.

Each Part adds one error-handling concern. Read the API docs in `src/` first;
the tests are the spec.

## Part 1 — Typed error enums and `#[from]`

Config loading returns a typed error hierarchy built with `thiserror`.

- `ParseError`: `NotUtf8`, `Json`, `NotAnObject`.
- `ConfigError`: `NotFound`, `Unreadable`, `Malformed` (each keeps the path and source),
  and `Setting(SettingError)` as a transparent variant.
- `SettingMissing` is hand-written (`Display` + `Error`) to show that no derive is required.
- `SettingError`: `Missing` (transparent) and `Invalid` (key, raw text, source).

Concepts: `#[error]`, `#[from]`, `#[source]`, `#[error(transparent)]`,
`#[non_exhaustive]`, distinguishing `NotFound` from other I/O failures.

## Part 2 — Source chains and `anyhow` at the edge

- `chain_messages` walks `Error::source()` outermost first and terminates on cycles.
- `get_setting` / `get_setting_as` walk dotted keys through nested objects. A cast
  is never called for a missing key.
- `startup_port` is an application-boundary function that returns `anyhow::Result`
  with `.context(...)`, while keeping the concrete error downcastable.

Concepts: `Error::source`, fat-pointer identity, `Box<dyn Error + Send + Sync>` at
boundaries, `anyhow::Context`, `downcast_ref`.

## Part 3 — Propagation and logs around a fallible step

`run_checked` wraps a step with a named `StepFailed` error and a log trail.
The trail records `start`, `end` (even on panic), then `ok` or `failed`.

Concepts: `?` with `From`, `impl FnOnce` callbacks, Drop-based guards that run
on unwinding, panics vs. errors.

## Part 4 — RAII transactions with savepoints

`Transaction` wraps `&mut HashMap<String, V>` and records an undo log.

- `commit` keeps the writes. Dropping without `commit` rolls back.
- `savepoint` runs a closure. `Err(SavepointRollback)` discards its writes and
  returns `Ok(None)`. Any other error discards them and propagates.
- The transaction does not require `V: Clone`.

Concepts: Drop as rollback, undo logs, `remove_entry`, nested marks, lifetime-bound
borrows of a store, no `V: Clone` bound.

## Part 5 — Scoped guards and resource cleanup

- `TempEnv` records previous values of an injected env map and restores them in
  reverse on drop (including on panic).
- `open_all` opens files in order and names the first failing path.
- `remove_files` skips missing files, counts deletions, and stops at the first real error.

Concepts: scoped guards, `drain(..).rev()`, `io::ErrorKind::NotFound` handling,
generic `AsRef<Path>` parameters.

## Part 6 — Panics as errors at a plugin boundary

`run_isolated` runs a plugin behind `catch_unwind`. It returns the plugin's value,
`PluginError::Failed` for an `Err`, or `PluginError::Panicked` with the payload
message when the plugin unwinds.

Concepts: `catch_unwind`, `UnwindSafe`, `AssertUnwindSafe`, panic payload downcasting.

## Part 7 — Batches, classification and retry

- `run_batch` rejects duplicate task names before anything runs, runs every task,
  and returns all failures in task order.
- `triage` splits failures using the `Classify` trait.
- `retry` retries transient failures with exponential backoff and stops at once on
  a permanent failure.

Concepts: trait-based classification, `io::ErrorKind`, `NonZeroU32` for "at least
one attempt", `Box<dyn FnOnce>` tasks, `partition`, `saturating_mul`.

## Concepts exercised

- `thiserror` enums, `#[from]`, `#[source]`, `#[error(transparent)]`, `#[non_exhaustive]`
- `anyhow` at the application boundary, `Context`, `downcast_ref`
- The `Error::source()` chain and its cycle safety
- `?` and `From` conversions; error type design at library vs. application layers
- Drop-based rollback and restore; panic safety of guards
- `catch_unwind`, `UnwindSafe`, panic payload handling
- Trait-based error classification and retry policy

## Run

```
cargo test -p ex09_error_handling
cargo test -p ex09_error_handling part2
cargo solutions -p ex09_error_handling
```

Until you implement the stubs, every test fails with `not yet implemented`.

## Stretch

- Replace `chain_messages`'s `Vec` of pointers with a `HashSet` keyed by address and
  vtable. Measure whether it matters for the usual chain lengths.
- Add a `Retry-After` case to `retry`: let a `Classify` implementation return a
  suggested delay, and use it instead of the exponential backoff.
- Make `Transaction` nested: a savepoint that returns a child transaction type,
  so that a savepoint cannot outlive its parent. Check the borrow errors that result.
- Add `Display` chain formatting (`"outer: middle: inner"`) using `chain_messages`,
  and compare it with `anyhow`'s `{:#}` output.
