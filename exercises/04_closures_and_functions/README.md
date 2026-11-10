# 04 · Functions deep dive

You are building the function-level toolkit that a platform team ships with its
Rust services: closures and the `Fn` hierarchy, callback registries, retry and
timing wrappers, memoisation with a TTL, HTTP-style middleware, a rate limiter
shared across threads, and a CLI command registry.

Every public item below must exist in the crate root or in the module named in
its heading. The tests import the crate and are the spec. Time is always
injected through `Clock` (see `src/lib.rs`), so tests never sleep.

## Part 1: closures and the `Fn` traits

Module `closures`.

- `make_multiplier(factor: i64) -> impl Fn(i64) -> i64`: a `move` closure that
  owns `factor`.
- `make_counter(start: u32) -> impl FnMut() -> u32`: returns the current value,
  then increments. Increments saturate at `u32::MAX`.
- `compose(f, g) -> impl Fn(A) -> C`: `compose(f, g)(x) == g(f(x))`, generic
  over the intermediate types.
- `make_adders(n: usize) -> Vec<impl Fn(i64) -> i64>`: the adder at index `i`
  adds `i`. Each closure owns its own offset. The `Vec` is homogeneous because
  all the closures come from one closure expression.
- `call_n_times(n, f: FnMut() -> T) -> Vec<T>`: calls `f` `n` times and collects
  the results. The `FnMut` state stays visible to the caller afterwards.
- `run_once(job: FnOnce() -> T) -> T`: runs a job that may consume its captures.
- `make_greeter(greeting: String) -> impl Fn(&str) -> String`: reusable, and it
  accepts a borrowed `&str` of any lifetime.

## Part 2: returning closures, `fn` pointers and callback registries

Modules `callbacks`.

- `Operation = fn(i64, i64) -> Option<i64>` and
  `lookup_operation(name) -> Option<Operation>`: `add`, `sub`, `mul`, `div`.
  Checked arithmetic returns `None` on overflow and on division by zero. The
  result is a plain function pointer, not a boxed closure.
- `discount_rule(kind, amount) -> Option<Box<dyn Fn(u64) -> u64 + Send + Sync>>`:
  - `"percent"`: discount is `floor(price * amount / 100)`, with `amount` in
    `0..=100`. Use widening arithmetic so large prices do not overflow.
  - `"flat"`: discount is `amount`, saturating at zero.
  - Any other kind, or an out-of-range percent, returns `None`.
  - Different arms return different closure types, so the result is boxed.
- `at_least(min) -> impl Fn(u64) -> bool`: an inclusive threshold predicate.
- `EventBus<E>`: `new`, `subscribe(Fn(&E) + Send + Sync) -> SubscriptionId`,
  `unsubscribe(id) -> bool`, `publish(&self, &E) -> usize` (listeners run in
  subscription order, returns how many ran), `len`, `is_empty`.
  - Listeners are `Fn`, not `FnMut`, so `publish` takes `&self`.
  - Ids are never reused, so a stale id cannot unsubscribe a newer listener.

## Part 3: higher-order `retry` and `timed`

Module `decorators`.

- `RetryPolicy::new(attempts, base_backoff) -> Result<_, PolicyError>`:
  `attempts` counts the first call and must be at least 1. Accessor
  `attempts()`. `backoff_after(failed_attempt)` is
  `base * 2^(failed_attempt - 1)`, saturating at `Duration::MAX`.
- `retry(&policy, op: FnMut(u32) -> Result<T, E>, sleep: FnMut(Duration)) -> Result<T, E>`:
  - `op` receives the 1-based attempt number.
  - `E: Retryable` (a trait you define via its `is_retryable()` method in
    `decorators`; the tests implement it on their own error type).
  - Non-retryable errors are returned at once. After a retryable failure,
    `sleep(backoff_after(n))` is called, even when the delay is zero. It is
    not called after a success or after the final failure.
  - The last error is returned unchanged.
- `timed(clock, sink: FnMut(Duration), f: FnMut(A) -> R) -> impl FnMut(A) -> R`:
  each call reports its elapsed clock time to `sink`. The report happens in a
  `Drop` guard, so a panicking call is still measured. The result or the panic
  passes through unchanged.
- Stacking order matters. `timed` outside `retry` includes the backoff sleeps.
  `retry` outside `timed` gives one measurement per attempt.

## Part 4: memoisation and a TTL cache

Module `memo`.

- `Memoized::new(f: FnMut(A) -> R)`, with `A: Hash + Eq + Clone`. `get(&mut self, arg) -> &R`
  returns a reference into the cache. `stats()` returns
  `CacheStats { hits, misses }`. `clear()` drops entries and resets counters.
  Tuples work as multi-argument keys. A panicking computation caches nothing.
- `TtlCache::new(ttl, max_entries: Option<NonZeroUsize>, clock, f) -> Result<_, CacheConfigError>`:
  - A zero TTL is an error.
  - An entry is fresh while `now - stored_at < ttl`. At exactly `ttl` it is
    recomputed. Hits do not refresh `stored_at`.
  - With `max_entries`, inserting a new key into a full cache evicts the least
    recently used entry. A hit counts as a use. Expired entries are not evicted
    eagerly.
  - `get`, `stats`, `len`, `is_empty`, `clear`.

## Part 5: middleware chains

Module `middleware`.

- `Request::new(path)`, `with_header(name, value)`, `header(name)`.
  Header names are case-insensitive. Setting a header again replaces the value.
- `Response::new(status, body)`, `with_header`, `header`, plus public `status`
  and `body` fields.
- `Pipeline::new(endpoint)`, `layer(middleware) -> Pipeline`, `handle(&self, request) -> Response`.
  - A middleware is `Fn(Request, &dyn Fn(Request) -> Response) -> Response`.
    It calls `next` to continue, or returns early to short-circuit.
  - The first layer added is the outermost.
  - The pipeline is `Send + Sync`, so it can be shared with `Arc` across threads.
- `require_header(name) -> impl Fn(...)`: passes through when the header is
  present, otherwise returns `400` with a body that names the header.

## Part 6: rate limiting with interior mutability

Module `rate_limit`.

- `RateLimiter::new(max_calls, period, clock) -> Result<_, ConfigError>`:
  `ZeroCalls` and `ZeroPeriod` are rejected.
- `try_acquire(&self) -> Result<(), RateLimited>`: a sliding window. A call at
  `now` is allowed when fewer than `max_calls` recorded calls satisfy
  `now - t < period`. A rejection carries `retry_after`, the time until the
  oldest recorded call leaves the window. Rejected calls are not recorded.
  The method takes `&self` and uses a `Mutex` internally, so one limiter can
  serve many threads with a single budget.
- `limited(&limiter, f) -> impl Fn(A) -> Result<R, RateLimited> + '_`: gates a
  closure through the limiter.

## Part 7: command registry

Module `commands`.

- `CommandRegistry::new()`: each registry is independent. `Default` is also derived.
- `register(name: &'static str, help: &'static str, handler) -> Result<(), RegistryError>`:
  the handler is `Fn(&[String]) -> Result<String, CommandError> + Send + Sync + 'static`.
  Empty or whitespace-containing names are `InvalidName`. A repeated name is
  `Duplicate`.
- `dispatch(name, args) -> Result<String, DispatchError>`: unknown names give
  `UnknownCommand(name)`. Handler errors come back as `DispatchError::Command`.
- `names() -> Vec<&'static str>`: sorted. `help(name) -> Option<&'static str>`.

## Concepts exercised

- `Fn` / `FnMut` / `FnOnce`: which one a closure implements, and why the bound
  you choose is part of the API
- `move` captures and why each closure in a loop needs its own copy
- `impl Trait` in return position versus `Box<dyn Fn>`, and when type erasure is required
- `fn` pointers from non-capturing closures
- Higher-ranked closure signatures (`for<'a> Fn(&'a str)`) and `&dyn Fn` callbacks
- `Send + Sync` bounds on callbacks, and sharing them through `Arc`
- Generic decorators with injected clock and sleep hooks; `Drop` guards for
  panic-safe measurement
- `HashMap` entry API, `Hash + Eq + Clone` key bounds, memoisation without
  cloning cached values
- Interior mutability (`Mutex`) behind `&self`, and poisoning recovery
- Error enums with `thiserror`, including `#[from]` and `transparent`

## Run

```
cargo test -p ex04_closures_and_functions                 # all tests (fail until implemented)
cargo test -p ex04_closures_and_functions part2           # one part
cargo solutions -p ex04_closures_and_functions            # grade the reference solution
cargo clippy -p ex04_closures_and_functions --all-targets
```

## Stretch

- Make `retry` async-aware: take an `AsyncFnMut` operation and an async sleep,
  and decide how cancellation interacts with the backoff.
- Replace `TtlCache`'s O(n) LRU scan with an intrusive linked list or a
  `slab`-backed ordering, and measure the difference at 100k entries.
- Make `RateLimiter` lock-free for the common path with an `AtomicU64` token
  bucket. Compare it with the sliding window for fairness under contention.
- Turn `Pipeline` into a `tower::Layer`/`Service` stack and show what the
  generic version gains and loses in compile time and error messages.
- Add a `Pipeline::layer_fn` that accepts closures returning `impl Future`.
