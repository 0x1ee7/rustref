# Exercise 14: Threads and Synchronisation

## Scenario

You are building **linkwatch**, an internal tool that monitors the links in
documentation sites. It needs to:

- check thousands of URLs in parallel without hammering any host,
- download pages with a global cap on concurrent requests,
- keep shared counters and a shutdown flag that many threads touch,
- maintain a bank-style ledger of "credits" where transfers must stay atomic,
- cache expensive lookups, with one loader per key even under contention,
- run a worker pool with a bounded queue and survive a crashing job,
- run periodic jobs that record their failures and stop promptly.

Everything here uses only `std`. The point is to reason about ownership across
threads, lock discipline, and failure modes: poisoning, panics, deadlocks, and
lost wake-ups.

## Parts

Each Part is a module in `src/` and a group of tests in `tests/threads_and_sync.rs`.

### Part 1: data parallelism and link checking (`parallel`, `link`)

- `parallel_map(items, workers, f) -> Vec<R>`: splits `items` into at most
  `workers` chunks, runs each on a scoped thread, and returns results in input
  order. `f` is `Fn(&T) -> R + Sync`, so it borrows its environment safely.
  - Empty input never calls `f`.
  - `workers == 0` panics with a message that mentions `workers`.
  - A panic in `f` propagates to the caller with its original payload.
- `check_links(urls, workers, fetch) -> Vec<LinkResult>`: fetches each
  *distinct* URL once, then mirrors the input order (duplicates included).
  - `Outcome::Status(u16)` or `Outcome::Error(String)` per URL; one failing
    URL never affects the others.
  - `Outcome::is_ok()` is true for 200 through 399 only.

### Part 2: a bounded downloader (`downloader`)

`BoundedDownloader<F>` wraps a fetch function `Fn(&str) -> Result<Vec<u8>, E>`
and caps concurrency with a gate shared by every caller.

- `download_all(&self, urls)` returns bodies in input order, or the error from
  the *earliest* failing index. Every URL is still attempted.
- `new(fetch, 0)` panics with a message that mentions `max_concurrent`.
- `peak_concurrency()` is the highest number of simultaneous fetches observed;
  `in_flight()` is the current number.

### Part 3: atomics and shared state (`stats`)

- `Counters`: named `u64` counters with `add(&self, name, delta) -> u64`. It is
  shared across threads without losing updates. `get` of an unknown name is `0`.
  `snapshot()` is a sorted copy that later updates do not change.
- `StopFlag`: a cooperative shutdown signal. Idempotent `request_stop`, and
  other threads observe it.
- `Handoff<T>`: a single-slot mailbox. A second publish before a take replaces
  the unseen value.

### Part 4: mutexes, lock ordering and poisoning (`bank`)

- `Account`: a balance behind a `Mutex`. `deposit`/`withdraw` reject zero, return
  `Overflow` instead of wrapping, and report `InsufficientFunds` with the
  current balance. `update(f)` runs a closure under the lock.
- `transfer(src, dst, amount)`: `SameAccount` is checked first, then
  `ZeroAmount`. Any error leaves both balances unchanged. Locks are taken in a
  global address order so opposing transfers cannot deadlock.
- A panic inside `update` poisons that account's lock. The account must stay
  usable afterwards.

### Part 5: LRU cache and single-flight loading (`cache`)

- `LruCache<K, V>`: a fixed-capacity least-recently-used cache. `get` and
  `insert` both count as touches. Replacing an existing key does not evict.
  `new(0)` panics with a message that mentions `capacity`.
- `SingleFlight<K, V, E>`: a memoizing loader shared across threads.
  - Concurrent calls for one key run the loader once; everyone gets its value.
  - Successes are kept. Errors are shared with callers already waiting, but
    not kept, so the next call runs the loader again.
  - A loader panic reaches its caller. Waiting callers retry, so one of them
    becomes the new leader.
  - `load_count()` counts loader executions only.

### Part 6: a thread pool with a bounded queue (`pool`)

- `ThreadPool::new(workers, queue_size)` panics on zero workers, with a message
  that mentions `worker`.
- `execute` blocks while the queue is full. `try_execute` returns
  `Err(QueueFull)` immediately instead.
- A panicking job is counted in `PoolStats::panicked`. Its worker keeps serving.
- `shutdown()` stops intake, runs every queued job, joins the workers, and
  returns `PoolStats { completed, panicked }`. Dropping the pool does the same.

### Part 7: periodic jobs and a bounded stop (`scheduler`)

- `Scheduler::add(name, interval, task)` validates in this order: started →
  `AlreadyStarted`; zero interval → `InvalidInterval`; repeated name →
  `DuplicateName`.
- `start()` gives each job its own thread. The job runs immediately, then every
  `interval`. Start twice is `AlreadyStarted`.
- A failing job is recorded as `JobError { job, message }` and keeps running.
  A panicking job is recorded with a message starting `panic: `.
- `stop(timeout)` wakes waiting jobs, joins them, and returns `StopTimeout` if a
  job is still inside its task when the timeout expires. A later `stop` waits
  for the rest. Stop before start is a no-op.
- Dropping a running scheduler stops it and joins its threads.

## Concepts exercised

- `std::thread::scope` and scoped borrows versus `'static` spawns
- `Send` and `Sync`, and which types cross thread boundaries
- `Arc`, `Mutex`, `RwLock`, `Condvar`, `Barrier`, `OnceLock`-style memoization
- Atomics: `fetch_add`, `fetch_max`, ordering choices (`Relaxed`, `Release`/`Acquire`)
- Lock ordering to prevent deadlock; holding locks across no blocking calls
- Poisoning: what it means and how to recover (`PoisonError::into_inner`)
- Panics across threads: `catch_unwind`, `resume_unwind`, `JoinHandle::join`
- Channels: `sync_channel` backpressure, `try_send`, `recv_timeout`
- RAII guards (`Permit`, `MutexGuard`) and Drop-driven shutdown
- Condvar predicate loops and `wait_timeout_while` to avoid lost wake-ups
- Typed errors with `Display` and `std::error::Error`
- API design: `impl Into<String>`, `impl FnOnce`, and `Fn + Sync` bounds

## Run

```sh
cargo test -p ex14_threads_and_sync
cargo test -p ex14_threads_and_sync part2
cargo solutions -p ex14_threads_and_sync
```

Every test here must fail against the stubs. Each one fails with a `todo!()`
panic or an assertion. Once the code is implemented, every test must pass.

Watch for hangs. Every blocking step is bounded by a watchdog. A deadlock fails
the test after about 10 seconds instead of hanging the run.

## Stretch ideas (untested)

- Replace the `Mutex<Receiver>` in the pool with a crossbeam-style MPMC channel.
  Compare the throughput.
- Make `LruCache` safe for concurrent access with sharded locks. Measure the
  contention.
- Add a `RwLock` read path to `Counters::snapshot` and benchmark it against the
  current approach.
- Give `Scheduler` cron-style schedules instead of fixed intervals, and use a
  single timer thread instead of one thread per job.
- Turn `SingleFlight` into an async variant using `std::future` with a manual
  waker. Discuss why it needs a different design.
