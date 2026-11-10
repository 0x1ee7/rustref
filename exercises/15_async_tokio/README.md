# 15 — Async Rust with Tokio

## Scenario

You are building the data-access layer of an internal "status aggregator" service. It fans out to
upstream systems, enforces concurrency and rate limits, serves a pub/sub feed, runs a worker pool,
pages through a remote API, pools expensive connections, and retries flaky calls. Every piece must
behave correctly under cancellation (a client disconnects mid-request), under load (a dependency
slows down), and on shutdown (the process receives SIGTERM while work is in flight).

Python's `asyncio` (see `pyref` exercise 15) teaches the same ideas with a single-threaded loop and
a cooperative `CancelledError`. In Rust the tools are different and the mistakes are different:
futures are inert until polled, dropping a future *is* cancellation, `Send` bounds decide where a
task may run, and there is no exception to catch. The exercise is about those differences.

## Conventions

- Public types, traits, and error enums are the contract. Private fields are yours to design.
- Use `Future` from the prelude (edition 2024); do not write `std::future::Future`.
- Tests are black-box integration tests under `tests/`. Timing tests use
  `#[tokio::test(start_paused = true)]`, so sleeps are virtual and the suite is deterministic.
- No global tracing subscriber: tests install one with `tracing::subscriber::set_default`.

## Part 1 — Concurrent fan-out with per-source timeouts (`fanout`)

Upstreams are registered as `(name, factory)` pairs, where the factory is `FnOnce() -> Future`.
Create the futures only when you run them.

```rust
pub enum Outcome<T> { Ok(T), Failed(String), TimedOut }
pub struct SourceResult<T> { pub name: String, pub outcome: Outcome<T> }

pub fn aggregate<T, E, F, Fut>(
    sources: Vec<(String, F)>,
    per_source_timeout: Duration,
) -> impl Future<Output = Result<Vec<SourceResult<T>>, ConfigError>>;
// F: FnOnce() -> Fut + Send + 'static; Fut: Future<Output = Result<T, E>> + Send + 'static
// E: Display + Send + 'static

pub struct StrictError<E> { pub name: String, pub error: E }
pub async fn fetch_all_strict<T, E, F, Fut>(sources: Vec<(String, F)>)
    -> Result<Vec<(String, T)>, StrictError<E>>;
```

Behaviour:

- Every source runs concurrently. Results come back in **registration order**, not completion order.
- A source error maps to `Failed(message)`. Only the per-source timer produces `TimedOut`.
- `Duration::ZERO` is rejected with `ConfigError::ZeroTimeout`, before any factory runs.
- Dropping the `aggregate` future aborts every in-flight source.
- `fetch_all_strict` is all-or-nothing. The first failure returns a `StrictError` that names the
  source. Sibling sources are cancelled and finished before the error is returned.
- `StrictError` implements `Display` (it names the source) and `std::error::Error`, with
  `source()` returning the underlying error.

Concepts: `JoinSet`/`spawn` versus lazy futures, `timeout`, `Drop`-driven cancellation,
`Display`/`Error` source chains, `Send + 'static` bounds on spawned work.

## Part 2 — Cancellation tokens, cancel-safe receiving, shielded work (`cancel`)

```rust
#[derive(Clone, Default)]
pub struct CancelToken;               // new, cancel, is_cancelled, async cancelled()

pub async fn collect_until_cancelled<T>(rx: &mut mpsc::Receiver<T>, token: &CancelToken) -> Vec<T>;

pub enum RunError<E> { Body(E), TimedOut }
pub async fn run_with_cleanup<T, E, C, CFut, F, Fut>(
    body: F, cleanup: C, timeout: Duration) -> Result<T, RunError<E>>;

pub async fn shielded_commit<T, E, F, Fut>(op: F) -> Result<T, E>;
// E: Display + Send + 'static
```

Behaviour:

- Clones share one flag. `cancel` is idempotent. `cancelled()` completes at once if already set and
  wakes every pending waiter otherwise.
- `collect_until_cancelled` checks the token first (biased select). It stops on cancel or when all
  senders are gone. Cancelling does not close the channel; unread messages stay in it.
- `run_with_cleanup` awaits `cleanup` in place after success, a body error, or a timeout. If the
  caller is cancelled while the body is running, cleanup runs on a detached task.
- `shielded_commit` runs `op` on a spawned task, so the operation finishes even if the caller is
  dropped. If the caller is gone when `op` fails, the error is logged at `ERROR` with target
  `commit`. A caller that is still waiting receives the error and nothing is logged.

Concepts: cancel safety of `select!`, `watch` channels, `Drop` and detached cleanup, `spawn`
versus cancellation, `tracing` events and targets.

## Part 3 — Bounded concurrency and blocking IO (`bounded`)

```rust
pub struct FetchError<E> { pub index: usize, pub error: E }
pub struct BoundedFetcher;            // new(limit), peak_in_flight(), async run(...)

impl BoundedFetcher {
    pub fn new(limit: usize) -> Result<Self, ConfigError>;
    pub fn peak_in_flight(&self) -> usize;
    pub async fn run<T, R, E, F, Fut>(&self, items: Vec<T>, fetch: F)
        -> Result<Vec<R>, FetchError<E>>;
    // F: Fn(T) -> Fut + Send + Sync + 'static; Fut: Future<Output = Result<R, E>> + Send
}

pub enum ReadFilesError { Config(ConfigError), Io { index: usize, error: io::Error } }
pub async fn read_files(paths: Vec<PathBuf>, limit: usize)
    -> Result<Vec<(PathBuf, Vec<u8>)>, ReadFilesError>;
```

Behaviour:

- At most `limit` fetches are in flight, across all concurrent `run` calls on the same fetcher.
  `peak_in_flight` reports the highest count ever observed.
- Results keep input order. On the first failure the other fetches are cancelled and finished
  before `run` returns `FetchError { index, error }`.
- `limit == 0` is `ConfigError::ZeroLimit`. For `read_files` this is checked before any IO.
- `read_files` reads with at most `limit` concurrent blocking reads. Each read runs on tokio's
  blocking pool, not on the async workers. Any `io::Error` is reported with the failing index.

Concepts: `Semaphore` as a concurrency limit, shared state with `Arc`, bounded `buffered`-style
execution, `spawn_blocking` for synchronous IO, error types carrying positional context.

## Part 4 — Pub/sub with a lagging-subscriber policy (`broker`)

```rust
pub enum Delivery<M> { Message(M), Lagged(u64), Closed }
pub struct Broker<M: Clone>;          // new(capacity), publish, subscribe, subscriber_count
pub struct Subscription<M>;           // async next(&mut self) -> Delivery<M>
```

Behaviour:

- `publish` returns how many subscribers were offered the message. It never waits. A stalled
  subscriber never blocks a publisher.
- Every subscriber sees every message published after it subscribed, in order.
- A subscriber that falls more than `capacity` messages behind receives `Lagged(n)` with the number
  of messages it missed, then continues from the oldest message still buffered.
- Dropping the broker drains each subscriber's buffer, then yields `Closed`.
- Dropping a `Subscription` unsubscribes it (`subscriber_count` drops).
- `capacity == 0` is `ConfigError::ZeroBuffer`.

Concepts: `broadcast` channels and their lag semantics, `Clone` bounds on fan-out payloads,
designing a backpressure policy explicitly rather than inheriting one.

## Part 5 — Worker pool with per-job replies (`pool`)

```rust
pub enum Shutdown { Drain, Abort }
pub enum PoolError<E> { Closed, Aborted, Handler(E) }
pub struct WorkerPool<T, R, E>;
// start(handler, workers), async submit(&self, job) -> Result<R, PoolError<E>>,
// async close(&self, Shutdown), processed() -> usize
```

Behaviour:

- `workers == 0` is `ConfigError::ZeroWorkers`. Jobs run concurrently across workers.
- `submit` returns the handler's result to the submitter. A handler error goes only to that
  submitter, and the pool keeps running. `processed()` counts successful jobs.
- `close(Drain)` stops accepting new work and finishes what is queued. `close(Abort)` fails queued
  and in-flight submitters with `Aborted` without waiting for the handlers to finish.
- Submitting after close yields `Closed`. A second `close` returns immediately.
- A job whose submitter was dropped before a worker picked it up is skipped.

Concepts: `mpsc` plus `oneshot` request/reply, `Arc`-shared handlers that are `Send + Sync`,
graceful versus forceful shutdown, task lifetime tied to channel closure.

## Part 6 — Lazy pagination and an async resource pool (`resources`)

```rust
pub struct Page<T> { pub items: Vec<T>, pub next_cursor: Option<String> }
pub struct Paginator<T, E, F>;        // new(fetch_page, max_pages), async next() -> Option<Result<T, E>>, pages_fetched()

pub trait Factory: Send + Sync + 'static {
    type Resource: Send + 'static;
    type Error: Send + 'static;
    fn create(&self) -> impl Future<Output = Result<Self::Resource, Self::Error>> + Send;
    fn close(&self, resource: Self::Resource) -> impl Future<Output = ()> + Send;
}
pub enum LeaseError<E> { Closed, Factory(E) }
pub struct ResourcePool<F: Factory>;  // new(factory, size), created(), idle(), async lease(), async close()
pub struct Lease<F: Factory>;         // Deref / DerefMut to F::Resource; releases on drop
```

Behaviour:

- `Paginator` is an async stream. It requests a page only when its items are needed. The first
  request has no cursor. An empty page with a cursor does not end the walk. A page without a cursor
  does. `max_pages` caps requests; `Some(0)` is `ConfigError::ZeroMaxPages`.
- `pages_fetched()` counts requests issued, including a failed one. After a fetch error, `next()`
  yields that error once and then `None` forever with no further requests.
- `ResourcePool` creates at most `size` resources. `lease` reuses an idle resource before creating
  one, and waits when all slots are leased. A cancelled waiter does not consume a slot.
- A failed `create` reports `LeaseError::Factory` and frees its slot. `created()` counts successes
  only.
- Dropping a `Lease`, including while unwinding from a panic, returns the resource and frees the slot.
- `close()` closes idle resources at once. A leased resource is closed when its lease is dropped.
  `lease` after close yields `LeaseError::Closed`. `size == 0` is `ConfigError::ZeroLimit`.

Concepts: pull-based async iteration without `Stream`, `Semaphore` permits as RAII, `Deref` guards,
`Drop` that must not block, return-position `impl Trait` in traits (RPITIT), `Send` futures.

## Part 7 — Retry, circuit breaker, and rate limiting (`resilience`)

```rust
pub trait Clock: Send + Sync { fn now(&self) -> tokio::time::Instant; }
pub struct SystemClock;               // reads tokio's clock (paused in tests)

pub struct RetryPolicy;               // new(attempts, base, max), delay_before_retry(n)
pub async fn retry<T, E, F, Fut, P>(policy: &RetryPolicy, op: F, retryable: P) -> Result<T, E>;

pub enum CircuitState { Closed, Open, HalfOpen }
pub enum CallError<E> { CircuitOpen, Inner(E) }
pub struct CircuitBreaker<C: Clock = SystemClock>;  // new(threshold, reset_timeout, clock), state(), call(op)

pub struct TokenBucket<C: Clock = SystemClock>;     // new(rate, capacity, clock), available(), try_acquire(), async acquire()
```

Behaviour:

- `RetryPolicy::new` with `attempts == 0` is `ConfigError::ZeroAttempts`. The delay before retry
  `n` (1-based) is `base * 2^(n-1)`, capped at `max`. `n == 0` is zero. Large `n` saturates
  instead of overflowing.
- `retry` runs `op` up to `attempts` times. It sleeps between attempts and never sleeps after the
  last one. It returns the last error. If `retryable` rejects an error, it returns that error
  immediately.
- The breaker opens after `threshold` consecutive failures and then rejects calls with
  `CallError::CircuitOpen` without running `op`. Its `state()` moves to `HalfOpen` once
  `reset_timeout` has elapsed on the injected clock. In `HalfOpen`, one trial call at a time is
  admitted. A success closes the circuit. A failure reopens it and restarts the timer. A trial that
  is dropped mid-flight counts as neither. `threshold == 0` is `ConfigError::ZeroFailureThreshold`.
- `TokenBucket` starts full. It refills continuously at `rate` tokens per second, capped at
  `capacity`. `try_acquire` never goes below zero. `acquire(n)` reserves the tokens and sleeps for
  the deficit. Invalid parameters are rejected: `rate` must be positive, `capacity` positive, and
  requests must be positive and no larger than `capacity`.

Concepts: exponential backoff with saturating arithmetic, injected clocks for deterministic state
machines, `Mutex` held only across synchronous sections, cancel-safe trial admission, token-bucket
reservation.

## Concepts exercised

- `async fn`, lazy futures, `Future` and `Send` bounds, `Drop` as cancellation
- `tokio::spawn`, `JoinHandle`, `JoinSet`-style joins, `spawn_blocking`, `abort`
- `select!` biasing and cancel safety, `timeout`, `sleep`, paused virtual time in tests
- `mpsc`, `oneshot`, `watch`, `broadcast` channels and their closure and lag semantics
- `Semaphore` permits, `Arc`-shared state, `Mutex` discipline across `.await`
- RPITIT (`impl Future` in traits), `Deref` guards, `thiserror` error enums with source chains
- `tracing` events and targets captured per test with `set_default`

## Run

```sh
cargo test -p ex15_async_tokio                 # all parts (fail until implemented)
cargo test -p ex15_async_tokio part2           # one part
cargo solutions -p ex15_async_tokio            # reference solution
```

## Stretch ideas (untested)

- Replace the broker's `Lagged` policy with a per-subscriber bounded `mpsc` that disconnects slow
  consumers, and compare the two under load.
- Make `aggregate` report per-source latency, using `Instant` captured inside each source task.
- Add graceful shutdown to `WorkerPool` that also cancels jobs via `CancelToken`, so `Drain` can
  time out and fall back to `Abort`.
- Build `Paginator` on top of `futures::Stream` and compare the ergonomics with the pull API.
- Add jitter to `RetryPolicy` with a seeded RNG injected through a trait, keeping tests deterministic.
