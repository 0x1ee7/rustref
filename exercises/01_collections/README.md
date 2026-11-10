# 01 · Collections: `ops_toolkit`

## Scenario

You own a small toolkit that the API gateway and two back-office jobs share:

- the gateway needs a **response cache** and a **per-tenant rate limiter**;
- the nightly sales job needs **top sellers** from a stream too large to hold in memory;
- the platform team needs **layered configuration** (CLI > environment > file > defaults);
- the warehouse and billing jobs need **stock reconciliation**, **volume pricing**,
  **SLA banding**, and **grouping** of shipment records.

Everything is in-process, synchronous and deterministic. Time is injected through a
`Clock` trait so tests (and you) control expiry and windows.

Run from the repo root. Stubs in `src/` panic with `todo!()` until you implement them.

## Parts

Each part is one module under `src/`. Public signatures in the stubs are the contract.

### Part 1: `cache::TtlCache<K, V, C = SystemClock>`: TTL + LRU response cache

- `new(max_size, ttl, clock)`. `Err(ZeroCapacity)` if `max_size == 0`; `Err(ZeroTtl)` if `ttl` is zero.
- `put(key, value)` inserts or replaces. Replacing refreshes TTL and recency. Expired entries are purged first, then least-recently-used entries are evicted until `len() <= max_size`.
- `get(&Q) -> Option<&V>` (`K: Borrow<Q>`, so `get("abc")` works on `String` keys). A live hit becomes most recent and counts as a hit. A missing or expired key counts as a miss, and expired entries are removed. A read never extends TTL.
- Expiry is exact: an entry written at `t` with TTL `d` is expired when `now >= t + d`.
- `keys()` lists live keys, least recently used first. `contains_key`, `keys`, `len`, `is_empty` do not touch recency or counters.
- `hits()` / `misses()` count `get` outcomes only.

Edge cases: expired entries do not count toward `len`; a TTL that overflows `Instant` never expires; purging expired entries happens before LRU eviction, so a live entry is never evicted to make room for a dead one.

### Part 2: `limiter::{ClientId, SlidingWindowLimiter<C>}`: per-tenant rate limit

- `ClientId::new(name)`. Equality and `Hash` are ASCII case-insensitive (`"Acme"` == `"ACME"`). `Eq` and `Hash` must agree, because `ClientId` is a `HashMap` key.
- `SlidingWindowLimiter::new(limit, window, clock)`. `Err(ZeroLimit)` / `Err(ZeroWindow)` for zero arguments.
- `allow(&ClientId) -> bool`: admits and records the request iff fewer than `limit` events are live. A denied request records nothing.
- An event at `t` is live while `now < t + window`. It leaves exactly at `t + window`.
- `remaining(&ClientId) -> u32`: `limit - live`, floored at 0. An unknown client has the full limit.
- `retry_after(&ClientId) -> Duration`: zero if `remaining > 0`; otherwise the time until the oldest live event leaves.
- `prune() -> usize`: drops clients with no live events and returns how many were dropped. Live state is never forgotten.

Edge cases: denied attempts must not extend the window; clients are independent; `prune` is the only place state is dropped.

### Part 3: `topk::top_k_products`: streaming top-K over sales lines

- `top_k_products(sales, k) -> Result<Vec<(String, u64)>, Error>` where `sales: IntoIterator<Item = (S, i64)>` and `S: AsRef<str>`.
- Sums units per SKU, then returns the `k` best sellers: units descending, ties broken by SKU ascending.
- Consumes the iterator once. The input can be far longer than the number of distinct SKUs, so memory is `O(distinct SKUs)`, and selection is `O(n log k)` rather than a full sort.
- `k == 0` returns an empty vector, but negative lines are still validated.
- `Err(NegativeUnits)` for any negative line. `Err(Overflow)` if a SKU total does not fit in `u64`.

### Part 4: `config::{Value, Layer, Layers, Config, deep_merge, parse_env}`: layered configuration

- `Value` is a nested tree: `Null`, `Bool`, `Int(i64)`, `Str`, `List`, `Map(BTreeMap)`. Maps are ordered, so equal maps compare and hash equal regardless of insertion order.
- `Layers { defaults, file, env, cli }`: `None` layers count as empty. `Config::new(&layers)` snapshots the layers, so later edits to `layers` do not leak into the config.
- `get("db.pool.size")` walks dotted paths. The highest-precedence layer that contains the full path wins. A `Null` still counts as present. Empty segments and paths through a scalar are absent.
- `source(path) -> Option<Layer>` reports the winning layer. `effective()` returns the fully merged tree as an owned value.
- `deep_merge(base, overlay)`: maps merge key by key and recurse; every other combination (lists, scalars, null) is replaced by the overlay. Inputs are not mutated.
- `parse_env(vars, prefix)`: variables named `<prefix>_<PATH>`, with `__` splitting segments and keys lowercased (`APP_DB__POOL__SIZE=10` becomes `db.pool.size = 10`). Values coerce to `Bool` (`true`/`false`, any case) or `Int` (optional `-`, then ASCII digits, fitting `i64`); everything else stays `Str`. Unrelated variables are ignored.
- Errors: `EmptyEnvSegment` for `APP_` or `APP_A__`; `EnvPathConflict` when two variables target the same leaf, or a leaf and a map (the result must not depend on input order).

### Part 5: `inventory::{Money, Reconciliation, reconcile, PriceTiers, SlaTier, sla_tier}`

- `Money` is integer cents. `Display` renders `12.05`. `checked_mul(qty) -> Option<Money>`. No floats anywhere in money arithmetic.
- `reconcile(expected, counted) -> Result<Reconciliation, Error>` compares two `HashMap<String, i64>`. `missing` = expected but not counted; `unexpected` = counted but not expected; `mismatched` = `counted - expected` where both exist. A counted `0` against an expected `3` is a mismatch of `-3`, not a missing SKU. Any negative quantity is `NegativeQuantity`, and the smallest such SKU is reported, independent of map iteration order.
- `PriceTiers::new(tiers)`: `(min_qty, unit_price)` pairs in any order. Errors: `NoPriceTiers`, `FirstTierMustStartAtOne`, `DuplicateTier(min_qty)`, `NonPositivePrice`.
- `unit_price(qty)`: the tier with the largest `min_qty <= qty` applies. `line_total(qty) = unit_price * qty`, exact. Errors: `ZeroQuantity`; `Overflow` on overflow.
- `sla_tier(minutes) -> Result<SlaTier, Error>`: `<= 15` Critical, `<= 60` High, `<= 240` Normal, otherwise Low (infinity is Low). Upper bounds are inclusive. Negative or NaN is `InvalidResponseTime`.

### Part 6: `records::{Shipment, group_by, LazyMap}`: records, grouping, lazy maps

- `Shipment` is immutable through its API: accessors plus `with_qty(qty) -> Shipment`, which returns a new record. Equality and `Hash` follow all fields.
- `group_by(items, key) -> Vec<(K, Vec<T>)>`: groups in first-seen key order; items keep input order within a group. Consumes the iterator once.
- `LazyMap<K, V, F>`: `get_or_create(key) -> &mut V` runs the factory only for a missing key. `get`, `contains_key`, `len`, `is_empty` are read-only and never create entries. `Clone` produces an independent map.

## Concepts exercised

- Choosing collections: `HashMap`, `BTreeMap`/`BTreeSet` (ordered output), `VecDeque`, `BinaryHeap` with `Reverse` for bounded top-K.
- Entry API and `Borrow<Q>` lookups; why `get` takes `&Q` and not `&K`.
- Ownership of keys in `put`/`insert` vs. borrowed lookups; `AsRef<str>` and `Into<String>` parameters.
- Custom `Eq`/`Hash` that must agree; `Hash` derived for value types.
- Iterator consumption: `IntoIterator` parameters, single-pass aggregation, `inspect`.
- Integer arithmetic: `checked_*`, `i64`/`u64` boundaries, no floating point for money.
- Injected time (`Clock` trait) and deterministic tests.
- Error enums with `thiserror`; `Result` with precise variants for callers to match.
- Recursive data (`enum Value`) with ordered maps; merging without mutating inputs.
- Property tests (`proptest`) against a naive reference model.

## Run

```text
cargo test -p ex01_collections                  # all parts (fail against the stubs)
cargo test -p ex01_collections part2            # one part
cargo solutions -p ex01_collections             # reference solution, must pass
```

## Stretch (untested)

- Make `TtlCache` thread-safe with a sharded lock, and benchmark it against a single `Mutex`.
- Replace the sliding-window limiter with a token bucket, and compare behaviour under bursts.
- Add a streaming `top_k` that works over `Iterator<Item = Result<_, _>>` and short-circuits on the first error.
- Implement `Serialize`/`Deserialize` for `Value` (serde) and round-trip a real config file.
- Add `ClientId` interning (`Arc<str>`) and measure the allocation difference in `allow`.
