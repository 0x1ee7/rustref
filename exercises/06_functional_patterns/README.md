# 06 · Functional patterns in Rust

You maintain the back-office services of a small retail platform: a shopping cart
backend, a payments ledger, a config loader for the edge workers, a signup API and
the nightly sales ETL. The team wants this code written in a functional style: enums
as the domain model, exhaustive `match`, immutable state with reducers, folds that
short-circuit on the first invalid event, and errors as values, with every error
reported where the caller needs them.

Modules (all `pub`, see `src/`):

- `compose` — closure composition helpers
- `cart` — Redux-style reducer over an `Action` enum
- `ledger` — event-sourced bank account
- `config` — `key = value` config parser
- `validate` — accumulate-all-errors validation of untyped JSON
- `etl` — CSV sales ETL and ranked report

## Part 1 · Enums as ADTs and the cart reducer (`cart`)

- `Cart { items: Vec<LineItem>, coupon: Option<String> }` (`Default` = empty cart).
- `LineItem { sku, name, unit_price_cents: u64, quantity: u32 }`.
- `Action` is an enum: `AddItem { sku, name, unit_price_cents, quantity }`,
  `RemoveItem { sku }`, `SetQuantity { sku, quantity }`, `ApplyCoupon(Option<String>)`,
  `ClearCart`.
- `reduce(&Cart, &Action) -> Result<Cart, CartError>`: pure. Returns a new cart; the input
  cart and the action are never modified. The `match` over `Action` has no wildcard arm.
  - `AddItem` appends a new line, or, if the sku exists, adds `quantity` to that line and
    keeps its position, name and price. `quantity == 0` -> `ZeroQuantity`. Quantity overflow
    -> `Overflow`.
  - `RemoveItem` removes the line. Unknown sku is a no-op that returns an equal cart.
  - `SetQuantity` on an unknown sku -> `UnknownSku`. `quantity == 0` removes the line.
  - `ApplyCoupon(Some(code))`: `code` must be known to `coupon_percent`, else
    `UnknownCoupon(code)`. `ApplyCoupon(None)` clears the coupon.
  - `ClearCart` returns `Cart::default()`.
- `coupon_percent(code) -> Option<u8>` looks up the read-only table `COUPONS`
  (`SAVE10` -> 10, `HALF` -> 50). Matching is case-sensitive.
- `subtotal_cents(&Cart) -> Result<u64, CartError>`: sum of `unit_price_cents * quantity`
  over all lines; `Overflow` on overflow.
- `total_cents(&Cart) -> Result<u64, CartError>`: subtotal minus `subtotal * percent / 100`
  (floored). An absent or unknown coupon gives no discount.

Concepts: enums with data as ADTs, exhaustive `match` (no `_` arm, so adding a variant is
a compile error at the reducer), `&` in / owned out reducers, `checked_add` and
`CartError` instead of silent wrap-around, why `Clone` at the boundary is the price of
immutability.

## Part 2 · Composition and closures (`compose`)

- `compose(f, g) -> impl Fn(A) -> C`: right-to-left, `compose(f, g)(x) == f(g(x))`.
  Types may change across the boundary (`B` and `C` are independent).
- `pipeline(steps: Vec<Box<dyn Fn(T) -> T>>) -> impl Fn(T) -> T`: left-to-right over
  the steps. Empty `steps` is the identity. The returned closure is callable repeatedly
  and owns its steps.

Concepts: `impl Fn` return types vs `Box<dyn Fn>` storage, `Fn` vs `FnOnce` bounds,
`move` closures capturing configuration, folding a list of functions.

## Part 3 · Patterns: an event-sourced account (`ledger`)

- `Event` enum: `Opened { account_id, owner }`, `Deposited { amount_cents: u64 }`,
  `Withdrawn { amount_cents: u64 }`, `Frozen`, `Unfrozen`.
- `Account { account_id, owner, balance_cents: u64, status: Status }`;
  `Status` is `Open` or `Frozen`.
- `apply(state: Option<&Account>, event: &Event) -> Result<Account, LedgerError>`.
  `None` means the account does not exist yet. Checks run in this order:
  1. `(Some, Opened)` -> `AlreadyOpened`; `(None, Opened)` creates a zero-balance open account.
  2. `(None, _)` -> `NotOpened`.
  3. `Deposited`/`Withdrawn` with amount `0` -> `NonPositiveAmount`.
  4. `Deposited`/`Withdrawn` on a frozen account -> `AccountFrozen`.
  5. `Withdrawn` above the balance -> `InsufficientFunds { balance_cents, requested_cents }`.
  6. `Deposited` past `u64::MAX` -> `BalanceOverflow`.
  - `Frozen` is idempotent. `Unfrozen` sets `Open`.
- `decide_withdraw(&Account, amount_cents: u64) -> Result<Event, LedgerError>`: a command
  handler. It returns the `Withdrawn` event to record if the withdrawal is allowed, using the
  same rules as `apply`. The account is not changed.

Concepts: matching on tuples of enums, guards, `@` bindings and range patterns
(`amount @ 1..`), nested destructuring, "decide then evolve" split between commands and
events, `checked_add` for money.

## Part 4 · Folds that stop at the first bad event (`ledger`, `cart`)

- `replay(events: &[Event]) -> Result<Account, LedgerError>`: `try_fold` from `None`.
  Stops at the first invalid event and returns its error. An empty log -> `EmptyLog`.
- `snapshots(events: &[Event]) -> Result<Vec<Account>, LedgerError>`: the state after
  each event, built with `scan`, collected into a `Result` so it stops at the first error.
  `snapshots(e).unwrap().len() == e.len()`, and its last element equals `replay(e)`.
- `replay_from(start: &Account, events: &[Event]) -> Result<Account, LedgerError>`:
  resumes from a snapshot and applies only the tail.
- `total_deposited(events: &[Event]) -> Option<u64>`: sum of `Deposited` amounts with
  `try_fold` and `checked_add`; `None` on overflow. Other events are ignored.
- `cart::run_actions(cart: Cart, actions: impl IntoIterator<Item = Action>) -> Result<Cart, CartError>`:
  folds the reducer over the actions, stopping at the first error.

Concepts: `fold` vs `try_fold` vs `scan`, `collect::<Result<Vec<_>, _>>()`
short-circuiting, Option as a fold accumulator, "replay from snapshot" for event sourcing.

## Part 5 · Option/Result combinators and `?` (`config`)

- `Config { host: String, port: u16, debug: bool, workers: u8 }`.
- `ConfigError` (`thiserror`, `PartialEq`) with variants `MalformedLine { line }`,
  `DuplicateKey { line, key }`, `UnknownKey(String)`, `MissingKey(&'static str)`,
  `EmptyHost`, `InvalidPort(String)`, `PortOutOfRange`, `InvalidBool(String)`,
  `WorkersOutOfRange`. Messages are in `src/config.rs`.
- `parse_port(&str) -> Result<u16, ConfigError>`: trims whitespace. Non-integer ->
  `InvalidPort(raw)`. Integers outside `1..=65535` -> `PortOutOfRange`, including values too
  large for `i64` (check `IntErrorKind`, do not report them as non-integers).
- `parse_ports(&str) -> Result<Vec<u16>, ConfigError>`: comma-separated, `collect` into a
  `Result`; first error wins.
- `parse_bool(&str) -> Result<bool, ConfigError>`: case-insensitive. `true/yes/1/on` ->
  `true`; `false/no/0/off` -> `false`; otherwise `InvalidBool(raw)`. No trimming here.
- `parse_optional_port(Option<&str>) -> Result<Option<u16>, ConfigError>`: uses `transpose`.
- `parse_config(&str) -> Result<Config, ConfigError>`: fail-fast, first error in file order.
  - Lines are `key = value` (both sides trimmed). Blank lines and lines starting with `#`
    (after trimming) are ignored.
  - Per line, checks run in this order: no `=` -> `MalformedLine { line }`; key not in
    `host`, `port`, `debug`, `workers` -> `UnknownKey`; key already seen -> `DuplicateKey`.
    Line numbers are 1-based.
  - After the scan: missing `host` -> `MissingKey("host")`; missing `port` ->
    `MissingKey("port")`; empty host -> `EmptyHost`; then `port` via `parse_port`; `debug`
    (default `false`) via `parse_bool`; `workers` (default 4) must be `1..=64`, else
    `WorkersOutOfRange`, and it must be an integer.

Concepts: `Option`/`Result` combinators (`ok_or_else`, `map`, `and_then`, `transpose`),
`?` as the propagation operator, `FromIterator for Result`, `IntErrorKind`, `thiserror`
for library errors, errors as values vs panics.

## Part 6 · Accumulate-all-errors validation (`validate`)

Signup payloads arrive as untyped `serde_json::Value`. Unlike `?`, validation must report
every problem, not just the first.

- `ValidationError { path: String, message: String }`. `Display` prints `path: message`,
  or only `message` when `path` is empty (root errors).
- `Checked<T> = Result<T, Vec<ValidationError>>`.
- `combine(Checked<A>, Checked<B>) -> Checked<(A, B)>`: applicative-style: both sides are
  evaluated, and failures from both are concatenated in order.
- `collect_all(impl IntoIterator<Item = Checked<T>>) -> Checked<Vec<T>>`: keeps every error
  in order.
- `require_object(&Value) -> Checked<&Map<String, Value>>`: root path `""`, message
  `expected object, got <type>`.
- `field(&Map, name) -> Checked<&Value>`: missing -> path `name`, message `missing`.
- `expect_string(path, &Value) -> Checked<&str>`: `expected string, got <type>`.
- `expect_integer(path, &Value) -> Checked<i64>`: `expected integer, got <type>`.
  Type names: `null`, `boolean`, `number` (floats too), `string`, `array`, `object`.
- `validate_signup(&Value) -> Checked<Signup>`, where `Signup { username: String, age: u8,
  email: String, tags: Vec<String> }`. Rules, reported in this order:
  - root must be an object.
  - `username`: must be a string (type error alone if not). Otherwise both rules are
    checked: at least 3 characters -> `length must be at least 3`; only `a-z`, `0-9`, `_`
    -> `may only contain a-z, 0-9 and _`.
  - `age`: integer (type error alone if not), `13..=120` -> `must be between 13 and 120`.
  - `email`: string (type error alone if not). Shape: exactly one `@`, both parts
    non-empty, no whitespace anywhere, the domain contains a `.` not at its start or end ->
    `must look like name@domain.tld`.
  - `tags`: optional. Absent or `null` -> `[]`. Otherwise an array (`tags: expected array,
    got <type>`); each element is a string (`tags[i]: expected string, got <type>`), and
    non-empty (`tags[i]: must not be empty`).

Missing required fields report `missing` at their path. Errors come out in the rule order above.

Concepts: error accumulation vs fail-fast, applicative combination of `Result`s, `?` vs
`combine`, `serde_json::Value` as untyped input, `Option` vs `null` in JSON, why the
combinators are plain functions rather than a trait hierarchy.

## Part 7 · ETL pipeline and ranked report (`etl`)

Rows are read from CSV into `RawRow { region, product, units, unit_price, sold_on }`
(all `String`, via `serde`). Columns: `unit_price` is dollars (`"12.50"`), `sold_on` is ISO-8601.

- `load_csv(&str) -> Result<Vec<RawRow>, csv::Error>`: header row required.
- `parse_cents(&str) -> Option<u64>`: exact money parsing (no floats). `"12.5"` -> `1250`,
  `"12"` -> `1200`, `"0.01"` -> `1`, `"0"` -> `0`. Accepts digits, optionally `.` followed by
  one or two digits. No sign, no whitespace, no exponent, no empty integer part (`".5"`),
  no empty fraction (`"12."`), no third decimal. Overflow -> `None`.
- `Sale { region, product, units: u32, unit_price_cents: u64, sold_on: jiff::civil::Date }`.
- `parse_sale(&RawRow) -> Result<Sale, EtlError>`: trims every field, lowercases the region,
  parses `units` (`u32`; negative -> error), `unit_price` (`parse_cents`), `sold_on`
  (`jiff::civil::Date`). The first failing field is reported as
  `InvalidField { field, value }` with the trimmed raw value.
- `clean_sales(impl IntoIterator<Item = RawRow>) -> (Vec<Sale>, Vec<RowError>)`: drops rows
  whose region is blank after trimming and rows whose `units` parses to exactly `0`. Other
  rows are parsed in order. A parse failure is collected as `RowError { row, error }`, where
  `row` is the zero-based index in the input, and processing continues.
- `revenue_cents(&Sale) -> Option<u64>`: `units * unit_price_cents`, `None` on overflow.
- `totals_by(&[Sale], key: impl Fn(&Sale) -> String) -> Result<BTreeMap<String, u64>, EtlError>`:
  sums revenue per key. A group whose total overflows -> `EtlError::Overflow(key)`.
- `ranked(&BTreeMap<String, u64>) -> Vec<(String, u64)>`: total descending, then name
  ascending. Use `sort_by_key` with `Reverse`.
- `distinct_regions(&[Sale]) -> Vec<String>`: unique regions, uppercased, sorted.
- `format_cents(cents: u64, currency: &str) -> String`: `format_cents(1_234_567, "$") == "$12,345.67"`.
  Thousands separators on the dollar part.
- `build_report(impl IntoIterator<Item = RawRow>, currency: &str) -> Result<Report, EtlError>`:
  `Report { lines, rejected }`. `clean_sales`, then `totals_by` by region, then `ranked`,
  then one line per region: `"us: $47.49"` (region as lowercased in `Sale`). `rejected`
  carries the `RowError`s from cleaning.

Concepts: ETL as a chain of iterator adapters, `filter`/`map`/`partition`, `BTreeMap` for
deterministic ordering, `sort_by_key` + `Reverse` for multi-key sorts, `Option` vs `Result`
for error domains, decimal money without floats, `serde` for CSV rows.

## Running

```
cargo test -p ex06_functional_patterns                 # all parts
cargo test -p ex06_functional_patterns part3           # one part
cargo solutions -p ex06_functional_patterns            # reference answer key
cargo clippy -p ex06_functional_patterns --all-targets
```

## Stretch (untested)

- Replace `Clone`-on-every-reduce with structural sharing: store `items` as `Rc<[LineItem]>`
  or use an immutable list and measure allocations.
- Add `lift(fn)` turning a plain function into `Fn(Result<A, E>) -> Result<B, E>`.
- Make `replay` report the index of the failing event (`(usize, LedgerError)`).
- Use `proptest` to prove `total_cents <= subtotal_cents` for every coupon.
