# 13 · Database operations with rusqlite

## Scenario

You are building the persistence layer of an order-management backend. The
service stores customers, a product catalogue, and orders with their lines in
SQLite. Requirements:

- Schema changes ship as ordered, idempotent migrations; a binary must refuse
  to run against a database that is newer than it understands.
- Multi-step writes (place an order: insert the order, its lines, and check
  stock-keeping units exist) are atomic, and a panic mid-transaction must not
  leave half an order behind.
- Money is never a float. Timestamps are stored as instants and compared
  across time zones correctly.
- Listing orders for an API uses keyset pagination with opaque cursors that
  survive concurrent inserts; reports (units sold, revenue, customer ranking)
  are computed in SQL, not in a loop of queries.
- Bulk product imports run in batches; one bad row fails its batch and no
  partial batch is committed.

Python's `sqlite3` hides most of these decisions. In Rust you own them: which
types cross the boundary, who owns the transaction, what a repository borrows,
and what an error means to the caller.

## Schema

Created by migration 1, indexed by migration 2. All tables are `STRICT`.

| table         | columns                                                                                           |
|---------------|---------------------------------------------------------------------------------------------------|
| `customers`   | `id` PK, `email` UNIQUE `COLLATE NOCASE`, `name`, `created_at` (µs since epoch)                   |
| `products`    | `sku` PK, `name`, `unit_price_cents` `CHECK >= 0`                                                  |
| `orders`      | `id` PK, `customer_id` FK → customers, `placed_at` (µs), `status` `CHECK IN ('pending','paid','shipped','cancelled')` |
| `order_lines` | `(order_id, line_no)` PK, `sku` FK → products, `quantity` `CHECK >= 1`, `unit_price_cents` (snapshot at order time) |

Migration 2 adds the indexes the keyset query and the reports need.

## Parts

### Part 1 — Connections and type mappings
`connect` must enable foreign keys and a busy timeout. `Money` stores integer
cents and refuses overflow instead of wrapping. `UtcTimestamp` stores
microseconds; instants compare by value, not by how they were written.
`OrderStatus` parses only the four known strings. `Error` must be `Send + Sync`.

### Part 2 — Migrations
`migrate` applies only pending versions, in order, each in its own transaction.
Re-running is a no-op. A failing migration rolls back and its error exposes the
underlying `rusqlite::Error` through `source()`. A database whose schema version
is higher than the newest known migration yields `SchemaTooNew`. Gaps and
out-of-order version lists are rejected before anything runs.

### Part 3 — Unit of work
`unit_of_work` opens a transaction, or a savepoint when already inside one.
Commit on `Ok`, roll back on `Err` and on panic. Nested units compose: an inner
rollback undoes only its own work. The outermost unit takes the write lock up
front (`BEGIN IMMEDIATE`).

### Part 4 — Repositories
Repositories borrow a `&Connection` and return owned domain values. Customer
emails are unique case-insensitively. Name search is a case-insensitive
substring match in which `%` and `_` are literal characters and input is always
bound, never interpolated. `place` validates every SKU before inserting and
reports all unknown ones (sorted, deduplicated). It snapshots each product's
price onto its line. An unknown customer leaves no trace. Zero or negative
quantities are rejected.

### Part 5 — Bulk import
`import_products` consumes any iterator of products, commits one transaction per
batch, and returns how many rows it processed. A zero batch size is an error. A
row that fails validation rolls back its whole batch, but earlier batches stay
committed; a second connection observes this lazily, batch by batch.

### Part 6 — Pagination
`page_keyset` returns up to `limit` orders after an opaque `OrderCursor`. The
cursor encodes `(placed_at, id)`, so ties are broken deterministically and
orders inserted before the cursor do not shift later pages. A full page yields
a `next_cursor`; a short page does not. Malformed cursors are `InvalidCursor`,
never a panic. `page_offset` exists to show what goes wrong with offsets.

### Part 7 — Reports and loading
`load_with_items` fetches a customer's orders with their lines in at most
two statements, regardless of how many orders there are. `units_sold_by_product`,
`revenue_by_customer` (integer cents, summed in SQL), `rank_customers_by_orders`
(dense ranking that includes zero-order customers) and `customer_running_units`
(window function over placed orders, cancelled excluded) are all single queries.

## Concepts exercised

- `rusqlite` connection lifetimes, `Transaction`/`Savepoint` and RAII rollback
- Typed `ToSql`/`FromSql` newtypes and why orphan rules shape them
- `thiserror` error enums with `#[from]`, `#[source]` and `Send + Sync` bounds
- Lifetime-carrying repositories (`PhantomData<&'conn Connection>`)
- Keyset vs offset pagination, opaque cursors, strict decoding
- Iterator consumption with batching, `IntoIterator` bounds
- Property testing (`proptest`) for cursor round trips
- Trace hooks (`trace_v2`) to count statements for N+1 detection

## Run

```sh
cargo test -p ex13_database_rusqlite                # all parts
cargo test -p ex13_database_rusqlite part2          # only Part 2
cargo solutions -p ex13_database_rusqlite           # reference solution
```

## Stretch

- Replace the cursor base64 with a signed token (HMAC) and reject tampering.
- Add a `WITH RECURSIVE` query for a customer's referral tree.
- Move `import_products` onto a `tokio::task::spawn_blocking` boundary and
  reason about why the `Connection` must not cross an `.await`.
- Add `PRAGMA journal_mode = WAL` and measure the reader/writer difference.
