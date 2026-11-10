//! Black-box tests for the order-management persistence layer.
//! Run one part with `cargo test -p ex13_database_rusqlite part3`.

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration, Instant};

use ex13_database_rusqlite::*;
use jiff::Timestamp;
use proptest::prelude::ProptestConfig;
use proptest::{prop_assert, prop_assert_eq, proptest};
use rusqlite::trace::{TraceEvent, TraceEventCodes};
use rusqlite::{Connection, params};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ts(s: &str) -> UtcTimestamp {
    UtcTimestamp::from_timestamp(s.parse::<Timestamp>().expect("valid RFC 3339 timestamp"))
}

/// In-memory database, migrated to the latest version.
fn fresh() -> Connection {
    let mut conn = connect(":memory:").expect("connect in-memory");
    migrate(&mut conn, MIGRATIONS).expect("migrate fresh database");
    conn
}

/// File-backed, migrated database. Keep the `TempDir` alive for the test.
fn fresh_file(dir: &TempDir) -> (std::path::PathBuf, Connection) {
    let path = dir.path().join("orders.db");
    let mut conn = connect(&path).expect("connect file database");
    migrate(&mut conn, MIGRATIONS).expect("migrate file database");
    (path, conn)
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count rows")
}

fn product(i: usize) -> Product {
    Product {
        sku: format!("SKU-{i:05}"),
        name: format!("Item {i}"),
        unit_price: Money::from_cents(i as i64 + 1),
    }
}

fn add_customer(conn: &Connection, email: &str, name: &str) -> Customer {
    CustomerRepository::new(conn)
        .add(email, name, ts("2026-01-01T00:00:00Z"))
        .expect("add customer")
}

fn add_product(conn: &Connection, sku: &str, cents: i64) {
    ProductRepository::new(conn)
        .upsert(sku, sku, Money::from_cents(cents))
        .expect("upsert product");
}

fn place(conn: &Connection, customer_id: i64, at: &str, lines: &[(&str, u32)]) -> i64 {
    let lines: Vec<NewLine<'_>> = lines
        .iter()
        .map(|&(sku, quantity)| NewLine { sku, quantity })
        .collect();
    OrderRepository::new(conn)
        .place(customer_id, ts(at), &lines)
        .expect("place order")
}

/// Base64url (unpadded) encoder for building malformed cursor tokens in tests.
fn b64url(input: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in input.as_bytes().chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        for i in 0..=chunk.len() {
            let idx = (n >> (18 - 6 * i)) & 63;
            out.push(ALPHABET[idx as usize] as char);
        }
    }
    out
}

thread_local! {
    static STATEMENTS: Cell<usize> = const { Cell::new(0) };
}

fn on_statement(_: TraceEvent<'_>) {
    STATEMENTS.with(|c| c.set(c.get() + 1));
}

/// Counts statements executed on this thread while `f` runs.
fn count_statements(conn: &mut Connection, f: impl FnOnce(&Connection)) -> usize {
    conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(on_statement))
        .expect("install trace callback");
    let before = STATEMENTS.with(Cell::get);
    f(conn);
    STATEMENTS.with(Cell::get) - before
}

// ---------------------------------------------------------------------------
// Part 1: connection policy and SQL type mappings
// ---------------------------------------------------------------------------

#[test]
fn part1_connect_enables_foreign_keys() {
    let conn = connect(":memory:").expect("connect");
    let on: i64 = conn
        .pragma_query_value(None, "foreign_keys", |r| r.get(0))
        .expect("read pragma");
    assert_eq!(on, 1, "foreign_keys must be ON for every connection");
}

#[test]
fn part1_connect_sets_busy_timeout_of_at_least_one_second() {
    let conn = connect(":memory:").expect("connect");
    let ms: i64 = conn
        .pragma_query_value(None, "busy_timeout", |r| r.get(0))
        .expect("read pragma");
    assert!(ms >= 1000, "busy_timeout is {ms} ms, expected at least 1000");
}

#[test]
fn part1_busy_writer_waits_for_lock_instead_of_failing() {
    let dir = TempDir::new().expect("tempdir");
    let (path, setup) = fresh_file(&dir);
    drop(setup);

    let holder = connect(&path).expect("holder");
    holder.execute_batch("BEGIN IMMEDIATE").expect("hold write lock");
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        holder.execute_batch("COMMIT").expect("release lock");
    });

    let writer = connect(&path).expect("writer");
    let started = Instant::now();
    CustomerRepository::new(&writer)
        .add("late@example.com", "Late", ts("2026-01-01T00:00:00Z"))
        .expect("write should wait for the lock, not fail with SQLITE_BUSY");
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "write returned before the lock was released"
    );
    releaser.join().expect("releaser thread");
}

#[test]
fn part1_money_is_stored_as_integer_cents() {
    let conn = fresh();
    conn.execute_batch("CREATE TABLE scratch (v INTEGER NOT NULL) STRICT")
        .expect("scratch table");
    conn.execute("INSERT INTO scratch (v) VALUES (?1)", params![Money::from_cents(-1234)])
        .expect("insert money");
    let raw: i64 = conn
        .query_row("SELECT v FROM scratch", [], |r| r.get(0))
        .expect("raw value");
    assert_eq!(raw, -1234, "Money must be stored as its cents count");
    let back: Money = conn
        .query_row("SELECT v FROM scratch", [], |r| r.get(0))
        .expect("typed value");
    assert_eq!(back, Money::from_cents(-1234));
}

#[test]
fn part1_money_displays_with_two_decimals() {
    assert_eq!(Money::from_cents(1234).to_string(), "12.34");
    assert_eq!(Money::from_cents(-5).to_string(), "-0.05");
    assert_eq!(Money::from_cents(0).to_string(), "0.00");
    assert_eq!(Money::from_cents(100).to_string(), "1.00");
}

#[test]
fn part1_money_checked_arithmetic_reports_overflow() {
    assert_eq!(
        Money::from_cents(250).checked_mul(4),
        Some(Money::from_cents(1000))
    );
    assert_eq!(Money::from_cents(i64::MAX).checked_mul(2), None);
    assert_eq!(Money::from_cents(i64::MAX).checked_add(Money::from_cents(1)), None);
    assert_eq!(
        Money::from_cents(1).checked_add(Money::from_cents(2)),
        Some(Money::from_cents(3))
    );
}

#[test]
fn part1_utc_timestamp_truncates_to_micros_and_round_trips() {
    let precise: Timestamp = "2026-03-01T12:00:00.123456789Z".parse().expect("parse");
    let micro: Timestamp = "2026-03-01T12:00:00.123456Z".parse().expect("parse");
    let stored = UtcTimestamp::from_timestamp(precise);
    assert_eq!(stored, UtcTimestamp::from_timestamp(micro), "sub-micro digits must be dropped");
    assert_eq!(stored.as_micros(), micro.as_microsecond());

    let conn = fresh();
    conn.execute_batch("CREATE TABLE scratch (v INTEGER NOT NULL) STRICT")
        .expect("scratch table");
    conn.execute("INSERT INTO scratch (v) VALUES (?1)", params![stored])
        .expect("insert timestamp");
    let back: UtcTimestamp = conn
        .query_row("SELECT v FROM scratch", [], |r| r.get(0))
        .expect("read timestamp");
    assert_eq!(back, stored);
}

#[test]
fn part1_utc_timestamp_compares_instants_not_offsets() {
    // Same instant written with two different offsets.
    assert_eq!(
        ts("2026-01-01T10:00:00+02:00"),
        ts("2026-01-01T08:00:00Z"),
        "offsets must be normalised to the same instant"
    );
    assert!(ts("2026-01-01T00:00:00.5Z") < ts("2026-01-01T00:00:01Z"));
}

#[test]
fn part1_order_status_parses_known_values_and_rejects_others() {
    assert_eq!("shipped".parse::<OrderStatus>().expect("parse"), OrderStatus::Shipped);
    for status in [
        OrderStatus::Pending,
        OrderStatus::Paid,
        OrderStatus::Shipped,
        OrderStatus::Cancelled,
    ] {
        assert_eq!(status.as_str().parse::<OrderStatus>().expect("round trip"), status);
        assert_eq!(status.to_string(), status.as_str());
    }
    match "refunded".parse::<OrderStatus>() {
        Err(Error::UnknownStatus(s)) => assert_eq!(s, "refunded"),
        other => panic!("expected UnknownStatus, got {other:?}"),
    }
}

#[test]
fn part1_order_status_is_stored_as_text() {
    let conn = fresh();
    conn.execute_batch("CREATE TABLE scratch (v TEXT NOT NULL) STRICT")
        .expect("scratch table");
    conn.execute("INSERT INTO scratch (v) VALUES (?1)", params![OrderStatus::Paid])
        .expect("insert status");
    let raw: String = conn
        .query_row("SELECT v FROM scratch", [], |r| r.get(0))
        .expect("raw status");
    assert_eq!(raw, "paid");
    let back: OrderStatus = conn
        .query_row("SELECT v FROM scratch", [], |r| r.get(0))
        .expect("typed status");
    assert_eq!(back, OrderStatus::Paid);
}

#[test]
fn part1_error_is_send_sync_and_std_error() {
    fn assert_bounds<T: Send + Sync + std::error::Error + 'static>(_: &T) {}
    let conn = fresh();
    let err = OrderRepository::new(&conn)
        .page_offset(0, 0)
        .expect_err("a zero limit must be rejected");
    assert_bounds(&err);
    assert!(matches!(err, Error::InvalidInput(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// Part 2: versioned migrations
// ---------------------------------------------------------------------------

#[test]
fn part2_fresh_database_migrates_to_latest_version() {
    assert!(!MIGRATIONS.is_empty(), "the shipped schema must have at least one migration");
    let mut conn = connect(":memory:").expect("connect");
    let version = migrate(&mut conn, MIGRATIONS).expect("migrate");
    assert_eq!(version as usize, MIGRATIONS.len());
    let stored: u32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("user_version");
    assert_eq!(stored, version, "user_version must equal the returned version");
}

#[test]
fn part2_shipped_migrations_are_numbered_from_one() {
    assert_eq!(MIGRATIONS.len(), 2, "the shipped schema has exactly two migrations");
    for (i, m) in MIGRATIONS.iter().enumerate() {
        assert_eq!(m.version as usize, i + 1, "migration {:?} is out of sequence", m.name);
        assert!(!m.name.is_empty(), "migration {} needs a name", m.version);
        assert!(!m.statements.is_empty(), "migration {} has no statements", m.version);
    }
}

#[test]
fn part2_migrate_is_idempotent() {
    let mut conn = fresh();
    let again = migrate(&mut conn, MIGRATIONS).expect("second migrate must be a no-op");
    assert_eq!(again as usize, MIGRATIONS.len());
}

#[test]
fn part2_creates_order_schema_tables() {
    let conn = fresh();
    for table in ["customers", "products", "orders", "order_lines"] {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(found, 1, "table {table} must exist after migrating");
    }
}

const V1_CREATE: Migration = Migration {
    version: 1,
    name: "create_widgets",
    statements: &["CREATE TABLE widgets (id INTEGER PRIMARY KEY, label TEXT NOT NULL)"],
};

const V2_ADD_QTY: Migration = Migration {
    version: 2,
    name: "add_qty",
    statements: &["ALTER TABLE widgets ADD COLUMN qty INTEGER NOT NULL DEFAULT 0"],
};

#[test]
fn part2_applies_only_pending_versions() {
    let mut conn = connect(":memory:").expect("connect");
    assert_eq!(migrate(&mut conn, &[V1_CREATE]).expect("v1"), 1);
    // Re-running v1 must not re-execute CREATE TABLE (which would fail).
    assert_eq!(migrate(&mut conn, &[V1_CREATE, V2_ADD_QTY]).expect("v1+v2"), 2);
    assert_eq!(migrate(&mut conn, &[V1_CREATE, V2_ADD_QTY]).expect("again"), 2);
    conn.execute("INSERT INTO widgets (label) VALUES ('a')", [])
        .expect("insert with default qty");
    let qty: i64 = conn
        .query_row("SELECT qty FROM widgets", [], |r| r.get(0))
        .expect("qty column exists");
    assert_eq!(qty, 0);
}

const BROKEN_V2: Migration = Migration {
    version: 2,
    name: "broken",
    statements: &["CREATE TABLE half_done (id INTEGER)", "THIS IS NOT SQL"],
};

#[test]
fn part2_failed_migration_rolls_back_and_reports_its_version() {
    let mut conn = connect(":memory:").expect("connect");
    let err = migrate(&mut conn, &[V1_CREATE, BROKEN_V2]).expect_err("v2 must fail");
    match &err {
        Error::Migration { version, name, .. } => {
            assert_eq!(*version, 2);
            assert_eq!(*name, "broken");
        }
        other => panic!("expected Error::Migration, got {other:?}"),
    }
    assert!(
        std::error::Error::source(&err).is_some(),
        "the driver error must be reachable through source()"
    );
    let stored: u32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("user_version");
    assert_eq!(stored, 1, "v1 stays applied, v2 is not");
    let half: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'half_done'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(half, 0, "the first statement of v2 must have been rolled back");
    assert!(conn.is_autocommit(), "no transaction may be left open after a failure");
}

#[test]
fn part2_rejects_database_newer_than_this_binary() {
    let mut conn = fresh();
    conn.pragma_update(None, "user_version", 99)
        .expect("set user_version");
    match migrate(&mut conn, MIGRATIONS) {
        Err(Error::SchemaTooNew { found, supported }) => {
            assert_eq!(found, 99);
            assert_eq!(supported as usize, MIGRATIONS.len());
        }
        other => panic!("expected SchemaTooNew, got {other:?}"),
    }
}

#[test]
fn part2_rejects_gaps_and_wrong_start_without_writing() {
    let mut conn = connect(":memory:").expect("connect");
    let gap = Migration { version: 3, name: "gap", statements: &["CREATE TABLE g (x)"] };
    match migrate(&mut conn, &[V1_CREATE, gap]) {
        Err(Error::MigrationOrder { expected, found }) => {
            assert_eq!((expected, found), (2, 3));
        }
        other => panic!("expected MigrationOrder for a gap, got {other:?}"),
    }
    match migrate(&mut conn, &[V2_ADD_QTY]) {
        Err(Error::MigrationOrder { expected, found }) => {
            assert_eq!((expected, found), (1, 2));
        }
        other => panic!("expected MigrationOrder for a missing v1, got {other:?}"),
    }
    let stored: u32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .expect("user_version");
    assert_eq!(stored, 0, "a rejected sequence must not apply anything");
}

#[test]
fn part2_schema_rejects_unknown_order_status() {
    let conn = fresh();
    let customer = add_customer(&conn, "check@example.com", "Check");
    let result = conn.execute(
        "INSERT INTO orders (customer_id, placed_at, status) VALUES (?1, 0, 'refunded')",
        [customer.id],
    );
    assert!(result.is_err(), "the CHECK constraint must reject unknown statuses");
}

// ---------------------------------------------------------------------------
// Part 3: unit of work and savepoints
// ---------------------------------------------------------------------------

#[test]
fn part3_commits_when_body_succeeds() {
    let conn = fresh();
    unit_of_work(&conn, |c| {
        CustomerRepository::new(c).add("a@example.com", "A", ts("2026-01-01T00:00:00Z"))?;
        Ok(())
    })
    .expect("unit of work");
    assert_eq!(count(&conn, "customers"), 1);
    assert!(conn.is_autocommit());
}

#[test]
fn part3_rolls_back_and_returns_the_same_error() {
    let conn = fresh();
    let result: Result<()> = unit_of_work(&conn, |c| {
        CustomerRepository::new(c).add("a@example.com", "A", ts("2026-01-01T00:00:00Z"))?;
        Err(Error::InvalidInput("boom"))
    });
    assert!(matches!(result, Err(Error::InvalidInput("boom"))), "got {result:?}");
    assert_eq!(count(&conn, "customers"), 0, "the insert must be rolled back");
    assert!(conn.is_autocommit());
}

#[test]
fn part3_rolls_back_on_panic_and_keeps_unwinding() {
    let conn = fresh();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = unit_of_work::<()>(&conn, |c| {
            CustomerRepository::new(c).add("p@example.com", "P", ts("2026-01-01T00:00:00Z"))?;
            panic!("body exploded");
        });
    }));
    assert!(outcome.is_err(), "the panic must keep unwinding");
    assert_eq!(count(&conn, "customers"), 0, "a panic must roll the transaction back");
    assert!(conn.is_autocommit(), "no transaction may survive a panic");
}

#[test]
fn part3_inner_failure_rolls_back_only_its_savepoint() {
    let conn = fresh();
    conn.execute_batch("BEGIN IMMEDIATE").expect("outer begin");
    let inner: Result<()> = unit_of_work(&conn, |c| {
        CustomerRepository::new(c).add("inner@example.com", "Inner", ts("2026-01-01T00:00:00Z"))?;
        Err(Error::InvalidInput("inner failed"))
    });
    assert!(inner.is_err());
    CustomerRepository::new(&conn)
        .add("outer@example.com", "Outer", ts("2026-01-01T00:00:00Z"))
        .expect("outer write after failed savepoint");
    conn.execute_batch("COMMIT").expect("outer commit");

    let repo = CustomerRepository::new(&conn);
    assert_eq!(repo.get_by_email("inner@example.com").expect("lookup"), None);
    assert!(repo.get_by_email("outer@example.com").expect("lookup").is_some());
    assert!(conn.is_autocommit(), "the outer transaction must be closed by COMMIT");
}

#[test]
fn part3_released_savepoint_is_committed_with_outer() {
    let conn = fresh();
    conn.execute_batch("BEGIN IMMEDIATE").expect("outer begin");
    unit_of_work(&conn, |c| {
        CustomerRepository::new(c).add("nested@example.com", "N", ts("2026-01-01T00:00:00Z"))?;
        Ok(())
    })
    .expect("inner ok");
    conn.execute_batch("COMMIT").expect("outer commit");
    assert_eq!(count(&conn, "customers"), 1);
}

#[test]
fn part3_outer_rollback_discards_released_savepoint() {
    let conn = fresh();
    conn.execute_batch("BEGIN IMMEDIATE").expect("outer begin");
    unit_of_work(&conn, |c| {
        CustomerRepository::new(c).add("gone@example.com", "G", ts("2026-01-01T00:00:00Z"))?;
        Ok(())
    })
    .expect("inner ok");
    conn.execute_batch("ROLLBACK").expect("outer rollback");
    assert_eq!(count(&conn, "customers"), 0);
}

#[test]
fn part3_begin_immediate_takes_write_lock_up_front() {
    let dir = TempDir::new().expect("tempdir");
    let (path, conn) = fresh_file(&dir);
    let other = connect(&path).expect("other connection");
    other.busy_timeout(Duration::ZERO).expect("no waiting for the rival");

    unit_of_work(&conn, |_c| {
        // Nothing has been written yet inside the unit of work, but BEGIN IMMEDIATE
        // must already hold the write lock.
        let rival = CustomerRepository::new(&other).add("rival@example.com", "R", ts("2026-01-01T00:00:00Z"));
        assert!(rival.is_err(), "another writer must be blocked as soon as the unit of work starts");
        Ok(())
    })
    .expect("unit of work");
}

// ---------------------------------------------------------------------------
// Part 4: repositories and order placement
// ---------------------------------------------------------------------------

#[test]
fn part4_add_customer_returns_the_persisted_row() {
    let conn = fresh();
    let repo = CustomerRepository::new(&conn);
    let created = repo
        .add("Ada@Example.com", "Ada Lovelace", ts("2026-02-03T04:05:06Z"))
        .expect("add");
    assert!(created.id > 0);
    assert_eq!(created.email, "Ada@Example.com");
    assert_eq!(created.name, "Ada Lovelace");
    assert_eq!(created.created_at, ts("2026-02-03T04:05:06Z"));
    assert_eq!(repo.get(created.id).expect("get"), Some(created));
}

#[test]
fn part4_duplicate_email_is_rejected_in_any_case() {
    let conn = fresh();
    let repo = CustomerRepository::new(&conn);
    repo.add("dup@example.com", "One", ts("2026-01-01T00:00:00Z")).expect("first");
    match repo.add("DUP@example.com", "Two", ts("2026-01-01T00:00:00Z")) {
        Err(Error::DuplicateEmail(email)) => assert_eq!(email, "DUP@example.com"),
        other => panic!("expected DuplicateEmail, got {other:?}"),
    }
    assert_eq!(count(&conn, "customers"), 1);
}

#[test]
fn part4_get_by_email_ignores_case() {
    let conn = fresh();
    let added = add_customer(&conn, "Mixed.Case@Example.com", "Mixed");
    let found = CustomerRepository::new(&conn)
        .get_by_email("mixed.case@example.COM")
        .expect("lookup");
    assert_eq!(found, Some(added));
}

#[test]
fn part4_search_by_name_is_case_insensitive_substring() {
    let conn = fresh();
    add_customer(&conn, "g@example.com", "Grace Hopper");
    add_customer(&conn, "a@example.com", "Alan Turing");
    add_customer(&conn, "h@example.com", "Hedy LAMARR");
    let names: Vec<String> = CustomerRepository::new(&conn)
        .search_by_name("HOP")
        .expect("search")
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, vec!["Grace Hopper".to_string()]);
}

#[test]
fn part4_search_treats_like_wildcards_literally() {
    let conn = fresh();
    add_customer(&conn, "u1@example.com", "100% Cotton Fan");
    add_customer(&conn, "u2@example.com", "100 Cotton Fan");
    add_customer(&conn, "u3@example.com", "Blue_Mug");
    add_customer(&conn, "u4@example.com", "BlueXMug");
    let repo = CustomerRepository::new(&conn);
    let percent: Vec<String> = repo.search_by_name("%").expect("search %").into_iter().map(|c| c.name).collect();
    assert_eq!(percent, vec!["100% Cotton Fan".to_string()]);
    let underscore: Vec<String> = repo.search_by_name("_").expect("search _").into_iter().map(|c| c.name).collect();
    assert_eq!(underscore, vec!["Blue_Mug".to_string()]);
}

#[test]
fn part4_search_rejects_empty_term() {
    let conn = fresh();
    assert!(matches!(
        CustomerRepository::new(&conn).search_by_name(""),
        Err(Error::InvalidInput(_))
    ));
}

#[test]
fn part4_sql_injection_attempts_are_inert() {
    let conn = fresh();
    let repo = CustomerRepository::new(&conn);
    let evil = "x'); DROP TABLE customers; --@example.com";
    repo.add(evil, "Mallory", ts("2026-01-01T00:00:00Z")).expect("add");
    assert_eq!(repo.get_by_email(evil).expect("lookup").expect("present").name, "Mallory");
    assert!(repo.search_by_name("' OR '1'='1").expect("search").is_empty());
    assert_eq!(count(&conn, "customers"), 1, "the table must survive");
}

#[test]
fn part4_product_upsert_inserts_then_updates() {
    let conn = fresh();
    let repo = ProductRepository::new(&conn);
    repo.upsert("MUG-1", "Mug", Money::from_cents(1200)).expect("insert");
    repo.upsert("MUG-1", "Ceramic Mug", Money::from_cents(1350)).expect("update");
    assert_eq!(
        repo.get("MUG-1").expect("get"),
        Some(Product {
            sku: "MUG-1".into(),
            name: "Ceramic Mug".into(),
            unit_price: Money::from_cents(1350),
        })
    );
    assert_eq!(count(&conn, "products"), 1);
}

#[test]
fn part4_upsert_rejects_negative_price_with_sku() {
    let conn = fresh();
    match ProductRepository::new(&conn).upsert("BAD-1", "Bad", Money::from_cents(-1)) {
        Err(Error::InvalidPrice { sku, .. }) => assert_eq!(sku, "BAD-1"),
        other => panic!("expected InvalidPrice, got {other:?}"),
    }
    assert_eq!(ProductRepository::new(&conn).get("BAD-1").expect("get"), None);
}

#[test]
fn part4_upsert_many_validates_every_row_before_writing() {
    let conn = fresh();
    let batch = vec![
        product(1),
        Product { sku: "NEG".into(), name: "Negative".into(), unit_price: Money::from_cents(-5) },
    ];
    assert!(matches!(
        ProductRepository::new(&conn).upsert_many(&batch),
        Err(Error::InvalidPrice { .. })
    ));
    assert_eq!(count(&conn, "products"), 0, "no row may be written when any row is invalid");
}

#[test]
fn part4_place_order_snapshots_unit_price() {
    let conn = fresh();
    let customer = add_customer(&conn, "snap@example.com", "Snap");
    add_product(&conn, "CHAIR", 4500);
    let order_id = place(&conn, customer.id, "2026-05-01T10:00:00Z", &[("CHAIR", 2)]);
    add_product(&conn, "CHAIR", 9900);
    let lines = OrderRepository::new(&conn).lines(order_id).expect("lines");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].unit_price, Money::from_cents(4500), "the price must be frozen at order time");
    assert_eq!(lines[0].quantity, 2);
}

#[test]
fn part4_place_order_numbers_lines_from_one_in_input_order() {
    let conn = fresh();
    let customer = add_customer(&conn, "lines@example.com", "Lines");
    add_product(&conn, "A", 100);
    add_product(&conn, "B", 200);
    let order_id = place(&conn, customer.id, "2026-05-01T10:00:00Z", &[("B", 1), ("A", 2)]);
    let lines = OrderRepository::new(&conn).lines(order_id).expect("lines");
    let shape: Vec<(i64, &str, u32)> = lines.iter().map(|l| (l.line_no, l.sku.as_str(), l.quantity)).collect();
    assert_eq!(shape, vec![(1, "B", 1), (2, "A", 2)]);
}

#[test]
fn part4_place_order_reports_unknown_skus_sorted_and_deduplicated() {
    let conn = fresh();
    let customer = add_customer(&conn, "skus@example.com", "Skus");
    add_product(&conn, "KNOWN", 100);
    let result = OrderRepository::new(&conn).place(
        customer.id,
        ts("2026-05-01T10:00:00Z"),
        &[
            NewLine { sku: "ZZ", quantity: 1 },
            NewLine { sku: "KNOWN", quantity: 1 },
            NewLine { sku: "AA", quantity: 1 },
            NewLine { sku: "ZZ", quantity: 2 },
        ],
    );
    match result {
        Err(Error::UnknownProducts(skus)) => {
            assert_eq!(skus, vec!["AA".to_string(), "ZZ".to_string()]);
        }
        other => panic!("expected UnknownProducts, got {other:?}"),
    }
    assert_eq!(count(&conn, "orders"), 0, "a rejected order must leave no header behind");
}

#[test]
fn part4_place_order_rejects_empty_lines_and_zero_quantity() {
    let conn = fresh();
    let customer = add_customer(&conn, "empty@example.com", "Empty");
    add_product(&conn, "X", 100);
    let repo = OrderRepository::new(&conn);
    assert!(matches!(
        repo.place(customer.id, ts("2026-05-01T10:00:00Z"), &[]),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        repo.place(customer.id, ts("2026-05-01T10:00:00Z"), &[NewLine { sku: "X", quantity: 0 }]),
        Err(Error::InvalidInput(_))
    ));
    assert_eq!(count(&conn, "orders"), 0);
}

#[test]
fn part4_place_order_unknown_customer_leaves_no_trace() {
    let conn = fresh();
    add_product(&conn, "X", 100);
    match OrderRepository::new(&conn).place(999, ts("2026-05-01T10:00:00Z"), &[NewLine { sku: "X", quantity: 1 }]) {
        Err(Error::UnknownCustomer(id)) => assert_eq!(id, 999),
        other => panic!("expected UnknownCustomer, got {other:?}"),
    }
    assert_eq!(count(&conn, "orders"), 0);
    assert_eq!(count(&conn, "order_lines"), 0);
}

#[test]
fn part4_set_status_updates_and_rejects_unknown_order() {
    let conn = fresh();
    let customer = add_customer(&conn, "status@example.com", "Status");
    add_product(&conn, "X", 100);
    let order_id = place(&conn, customer.id, "2026-05-01T10:00:00Z", &[("X", 1)]);
    let repo = OrderRepository::new(&conn);
    assert_eq!(repo.get(order_id).expect("get").expect("present").status, OrderStatus::Pending);
    repo.set_status(order_id, OrderStatus::Shipped).expect("set");
    assert_eq!(repo.get(order_id).expect("get").expect("present").status, OrderStatus::Shipped);
    match repo.set_status(9999, OrderStatus::Paid) {
        Err(Error::OrderNotFound(id)) => assert_eq!(id, 9999),
        other => panic!("expected OrderNotFound, got {other:?}"),
    }
    assert_eq!(repo.get(9999).expect("get"), None);
}

// ---------------------------------------------------------------------------
// Part 5: bulk import and upsert
// ---------------------------------------------------------------------------

#[test]
fn part5_import_consumes_a_one_shot_iterator_and_counts_rows() {
    let conn = fresh();
    let processed = import_products(&conn, (0..2_500).map(product), 500).expect("import");
    assert_eq!(processed, 2_500);
    assert_eq!(count(&conn, "products"), 2_500);
    assert_eq!(
        ProductRepository::new(&conn).get("SKU-02499").expect("get").expect("present").name,
        "Item 2499"
    );
}

#[test]
fn part5_import_is_an_upsert_not_a_duplicate_insert() {
    let conn = fresh();
    import_products(&conn, (0..10).map(product), 4).expect("first import");
    let renamed = (0..10).map(|i| Product { name: format!("Renamed {i}"), ..product(i) });
    let processed = import_products(&conn, renamed, 4).expect("second import");
    assert_eq!(processed, 10);
    assert_eq!(count(&conn, "products"), 10);
    assert_eq!(
        ProductRepository::new(&conn).get("SKU-00003").expect("get").expect("present").name,
        "Renamed 3"
    );
}

#[test]
fn part5_import_rejects_zero_batch_size() {
    let conn = fresh();
    assert!(matches!(
        import_products(&conn, (0..3).map(product), 0),
        Err(Error::InvalidInput(_))
    ));
    assert_eq!(count(&conn, "products"), 0);
}

#[test]
fn part5_bad_row_rolls_back_its_batch_and_stops_the_import() {
    let conn = fresh();
    let rows = (0..2_500).map(|i| {
        if i == 1_200 {
            Product { unit_price: Money::from_cents(-1), ..product(i) }
        } else {
            product(i)
        }
    });
    match import_products(&conn, rows, 500) {
        Err(Error::InvalidPrice { sku, .. }) => assert_eq!(sku, "SKU-01200"),
        other => panic!("expected InvalidPrice, got {other:?}"),
    }
    // Batches [0,500) and [500,1000) committed; [1000,1500) rolled back.
    assert_eq!(count(&conn, "products"), 1_000);
    let repo = ProductRepository::new(&conn);
    assert!(repo.get("SKU-00999").expect("get").is_some());
    assert_eq!(repo.get("SKU-01000").expect("get"), None, "the failing batch must be rolled back");
    assert_eq!(repo.get("SKU-01300").expect("get"), None, "rows after the bad one must not be read");
}

#[test]
fn part5_import_commits_per_batch_and_pulls_lazily() {
    let dir = TempDir::new().expect("tempdir");
    let (path, writer) = fresh_file(&dir);
    let reader = connect(&path).expect("reader");
    let rows = (0..2_500usize).map(move |i| {
        if i % 1_000 == 0 {
            let committed: i64 = reader
                .query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))
                .expect("count from another connection");
            assert_eq!(committed, i as i64, "exactly {i} rows must be committed before item {i} is pulled");
        }
        product(i)
    });
    let processed = import_products(&writer, rows, 1_000).expect("import");
    assert_eq!(processed, 2_500);
}

// ---------------------------------------------------------------------------
// Part 6: keyset and offset pagination with opaque cursors
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn part6_cursor_round_trips_any_position(
        micros in -1_000_000_000_000i64..4_000_000_000_000_000i64,
        id in 1i64..i64::MAX,
    ) {
        let cursor = OrderCursor {
            placed_at: UtcTimestamp::from_micros(micros).expect("in range"),
            order_id: id,
        };
        let token = cursor.encode();
        prop_assert!(!token.contains('='), "token must be unpadded: {}", token);
        prop_assert!(
            token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "token must be URL-safe: {}", token
        );
        prop_assert_eq!(OrderCursor::decode(&token).expect("decode"), cursor);
    }
}

#[test]
fn part6_cursor_decode_rejects_malformed_tokens() {
    let bad = [
        String::new(),
        "not base64!".to_string(),
        format!("{}=", b64url("1:12")),
        b64url("12"),
        b64url("12:x"),
        b64url("x:1"),
        b64url("+1:1"),
        b64url("1:2:3"),
        b64url(&format!("{}:1", i64::MAX)),
    ];
    for token in &bad {
        assert!(
            matches!(OrderCursor::decode(token), Err(Error::InvalidCursor)),
            "token {token:?} must be rejected with InvalidCursor"
        );
    }
}

#[test]
fn part6_keyset_walk_visits_every_order_once() {
    let conn = fresh();
    let customer = add_customer(&conn, "walk@example.com", "Walk");
    add_product(&conn, "P", 100);
    let mut expected = Vec::new();
    for day in 1..=5 {
        expected.push(place(&conn, customer.id, &format!("2026-05-0{day}T00:00:00Z"), &[("P", 1)]));
    }
    let repo = OrderRepository::new(&conn);
    let mut seen = Vec::new();
    let mut cursor: Option<OrderCursor> = None;
    let mut pages = 0;
    loop {
        // Round-trip the token through its string form, as an HTTP client would.
        let after = cursor.map(|c| OrderCursor::decode(&c.encode()).expect("decode"));
        let page = repo.page_keyset(after.as_ref(), 2).expect("page");
        seen.extend(page.items.iter().map(|o| o.id));
        pages += 1;
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
        assert!(pages < 10, "pagination must terminate");
    }
    assert_eq!(seen, expected, "every order exactly once, in placement order");
    assert_eq!(pages, 3);
}

#[test]
fn part6_keyset_full_last_page_yields_an_empty_final_page() {
    let conn = fresh();
    let customer = add_customer(&conn, "full@example.com", "Full");
    add_product(&conn, "P", 100);
    for day in 1..=4 {
        place(&conn, customer.id, &format!("2026-06-0{day}T00:00:00Z"), &[("P", 1)]);
    }
    let repo = OrderRepository::new(&conn);
    let first = repo.page_keyset(None, 2).expect("page 1");
    assert_eq!(first.items.len(), 2);
    let second = repo.page_keyset(first.next_cursor.as_ref(), 2).expect("page 2");
    assert_eq!(second.items.len(), 2);
    assert!(second.next_cursor.is_some(), "a full page must advertise a next cursor");
    let third = repo.page_keyset(second.next_cursor.as_ref(), 2).expect("page 3");
    assert!(third.items.is_empty());
    assert_eq!(third.next_cursor, None);
}

#[test]
fn part6_keyset_orders_ties_on_timestamp_by_id() {
    let conn = fresh();
    let customer = add_customer(&conn, "tie@example.com", "Tie");
    add_product(&conn, "P", 100);
    let ids: Vec<i64> = (0..3).map(|_| place(&conn, customer.id, "2026-07-01T09:00:00Z", &[("P", 1)])).collect();
    let page = OrderRepository::new(&conn).page_keyset(None, 10).expect("page");
    let got: Vec<i64> = page.items.iter().map(|o| o.id).collect();
    assert_eq!(got, ids);
    assert_eq!(page.next_cursor, None, "a short page has no next cursor");
}

#[test]
fn part6_keyset_pages_do_not_overlap_when_older_rows_arrive() {
    let conn = fresh();
    let customer = add_customer(&conn, "late-arrival@example.com", "Late");
    add_product(&conn, "P", 100);
    for day in 10..=13 {
        place(&conn, customer.id, &format!("2026-08-{day}T00:00:00Z"), &[("P", 1)]);
    }
    let repo = OrderRepository::new(&conn);
    let first = repo.page_keyset(None, 2).expect("page 1");
    let first_ids: Vec<i64> = first.items.iter().map(|o| o.id).collect();

    // An older order arrives between the two requests.
    let older = place(&conn, customer.id, "2026-08-01T00:00:00Z", &[("P", 1)]);

    let second = repo.page_keyset(first.next_cursor.as_ref(), 2).expect("page 2");
    let second_ids: Vec<i64> = second.items.iter().map(|o| o.id).collect();
    assert_eq!(second_ids, vec![first_ids[1] + 1, first_ids[1] + 2]);
    assert!(!second_ids.contains(&older), "the older order sits before the cursor");
}

#[test]
fn part6_offset_pages_shift_when_older_rows_arrive() {
    let conn = fresh();
    let customer = add_customer(&conn, "shift@example.com", "Shift");
    add_product(&conn, "P", 100);
    for day in 10..=13 {
        place(&conn, customer.id, &format!("2026-08-{day}T00:00:00Z"), &[("P", 1)]);
    }
    let repo = OrderRepository::new(&conn);
    let first: Vec<i64> = repo.page_offset(0, 2).expect("page 1").iter().map(|o| o.id).collect();
    place(&conn, customer.id, "2026-08-01T00:00:00Z", &[("P", 1)]);
    let second: Vec<i64> = repo.page_offset(2, 2).expect("page 2").iter().map(|o| o.id).collect();
    let overlap: Vec<i64> = first.iter().copied().filter(|id| second.contains(id)).collect();
    assert_eq!(overlap.len(), 1, "offset paging repeats a row after an insert before it: {first:?} vs {second:?}");
}

#[test]
fn part6_zero_limit_is_invalid_input() {
    let conn = fresh();
    let repo = OrderRepository::new(&conn);
    assert!(matches!(repo.page_keyset(None, 0), Err(Error::InvalidInput(_))));
    assert!(matches!(repo.page_offset(0, 0), Err(Error::InvalidInput(_))));
}

// ---------------------------------------------------------------------------
// Part 7: loading without N+1, and reporting queries
// ---------------------------------------------------------------------------

#[test]
fn part7_load_with_items_groups_lines_in_placement_order() {
    let conn = fresh();
    let ann = add_customer(&conn, "ann@example.com", "Ann");
    let ben = add_customer(&conn, "ben@example.com", "Ben");
    add_product(&conn, "A", 100);
    add_product(&conn, "B", 200);
    let late = place(&conn, ann.id, "2026-09-03T00:00:00Z", &[("A", 1), ("B", 2)]);
    let early = place(&conn, ann.id, "2026-09-01T00:00:00Z", &[("B", 1)]);
    let middle = place(&conn, ann.id, "2026-09-02T00:00:00Z", &[("A", 4)]);
    place(&conn, ben.id, "2026-09-02T00:00:00Z", &[("A", 1)]);
    OrderRepository::new(&conn).set_status(early, OrderStatus::Cancelled).expect("cancel");

    let loaded = OrderRepository::new(&conn).load_with_items(ann.id).expect("load");
    let ids: Vec<i64> = loaded.iter().map(|w| w.order.id).collect();
    assert_eq!(ids, vec![early, middle, late], "cancelled orders stay, ordered by (placed_at, id)");
    assert_eq!(loaded[0].order.status, OrderStatus::Cancelled);
    assert_eq!(loaded[2].lines.iter().map(|l| l.line_no).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(loaded[2].lines[1].sku, "B");
    assert_eq!(loaded[1].lines.len(), 1);
}

#[test]
fn part7_load_with_items_for_customer_without_orders_is_empty() {
    let conn = fresh();
    let nobody = add_customer(&conn, "nobody@example.com", "Nobody");
    assert!(OrderRepository::new(&conn).load_with_items(nobody.id).expect("load").is_empty());
}

#[test]
fn part7_load_with_items_uses_a_bounded_number_of_statements() {
    let mut conn = fresh();
    let small = add_customer(&conn, "small@example.com", "Small");
    let large = add_customer(&conn, "large@example.com", "Large");
    add_product(&conn, "A", 100);
    add_product(&conn, "B", 200);
    for day in 1..=2 {
        place(&conn, small.id, &format!("2026-10-0{day}T00:00:00Z"), &[("A", 1), ("B", 1)]);
    }
    for day in 1..=20 {
        place(&conn, large.id, &format!("2026-11-{day:02}T00:00:00Z"), &[("A", 1), ("B", 1)]);
    }

    let mut small_len = 0;
    let small_stmts = count_statements(&mut conn, |c| {
        small_len = OrderRepository::new(c).load_with_items(small.id).expect("load").len();
    });
    let mut large_len = 0;
    let large_stmts = count_statements(&mut conn, |c| {
        large_len = OrderRepository::new(c).load_with_items(large.id).expect("load").len();
    });

    assert_eq!((small_len, large_len), (2, 20));
    assert_eq!(
        small_stmts, large_stmts,
        "statement count must not grow with the number of orders (N+1 detected)"
    );
    assert!(large_stmts <= 2, "expected at most 2 statements, ran {large_stmts}");
}

#[test]
fn part7_units_sold_excludes_cancelled_and_sorts_by_units_then_sku() {
    let conn = fresh();
    let x = add_customer(&conn, "x@example.com", "X");
    add_product(&conn, "A", 100);
    add_product(&conn, "B", 200);
    add_product(&conn, "C", 300);
    place(&conn, x.id, "2026-12-01T00:00:00Z", &[("A", 2), ("B", 1)]);
    let cancelled = place(&conn, x.id, "2026-12-02T00:00:00Z", &[("A", 1)]);
    OrderRepository::new(&conn).set_status(cancelled, OrderStatus::Cancelled).expect("cancel");
    let paid = place(&conn, x.id, "2026-12-03T00:00:00Z", &[("B", 3), ("C", 1)]);
    OrderRepository::new(&conn).set_status(paid, OrderStatus::Paid).expect("pay");

    let report = units_sold_by_product(&conn).expect("report");
    let shape: Vec<(&str, i64, i64)> = report.iter().map(|r| (r.sku.as_str(), r.units_sold, r.order_count)).collect();
    assert_eq!(shape, vec![("B", 4, 2), ("A", 2, 1), ("C", 1, 1)]);
}

#[test]
fn part7_revenue_by_customer_is_exact_integer_cents() {
    let conn = fresh();
    let x = add_customer(&conn, "rev-x@example.com", "Xena");
    let y = add_customer(&conn, "rev-y@example.com", "Yusuf");
    let z = add_customer(&conn, "rev-z@example.com", "Zoe");
    add_product(&conn, "TEN", 10);
    add_product(&conn, "SEVEN", 7);
    place(&conn, x.id, "2027-01-01T00:00:00Z", &[("TEN", 3), ("SEVEN", 1)]);
    place(&conn, x.id, "2027-01-02T00:00:00Z", &[("TEN", 1)]);
    let y_order = place(&conn, y.id, "2027-01-01T00:00:00Z", &[("TEN", 5)]);
    OrderRepository::new(&conn).set_status(y_order, OrderStatus::Cancelled).expect("cancel");
    for day in 1..=3 {
        place(&conn, z.id, &format!("2027-02-0{day}T00:00:00Z"), &[("TEN", 1)]);
    }

    let report = revenue_by_customer(&conn).expect("report");
    let shape: Vec<(i64, &str, i64, Money)> = report
        .iter()
        .map(|r| (r.customer_id, r.name.as_str(), r.order_count, r.total))
        .collect();
    assert_eq!(
        shape,
        vec![
            (x.id, "Xena", 2, Money::from_cents(30 + 7 + 10)),
            (z.id, "Zoe", 3, Money::from_cents(30)),
        ],
        "Yusuf only has a cancelled order and must be omitted"
    );
}

#[test]
fn part7_rank_customers_includes_zero_orders_and_shares_ties() {
    let conn = fresh();
    add_product(&conn, "P", 100);
    let ann = add_customer(&conn, "rank-ann@example.com", "Ann");
    let ben = add_customer(&conn, "rank-ben@example.com", "Ben");
    let cy = add_customer(&conn, "rank-cy@example.com", "Cy");
    add_customer(&conn, "rank-dee@example.com", "Dee");
    let eve = add_customer(&conn, "rank-eve@example.com", "Eve");
    for (customer, n) in [(&ann, 2), (&ben, 2), (&cy, 1)] {
        for day in 1..=n {
            place(&conn, customer.id, &format!("2027-03-0{day}T00:00:00Z"), &[("P", 1)]);
        }
    }
    let gone = place(&conn, eve.id, "2027-03-01T00:00:00Z", &[("P", 1)]);
    OrderRepository::new(&conn).set_status(gone, OrderStatus::Cancelled).expect("cancel");

    let report = rank_customers_by_orders(&conn).expect("rank");
    let shape: Vec<(&str, i64, i64)> = report
        .iter()
        .map(|r| (r.name.as_str(), r.order_count, r.rank))
        .collect();
    assert_eq!(
        shape,
        vec![("Ann", 2, 1), ("Ben", 2, 1), ("Cy", 1, 3), ("Dee", 0, 4), ("Eve", 0, 4)]
    );
}

#[test]
fn part7_running_units_accumulate_in_placement_order() {
    let conn = fresh();
    let x = add_customer(&conn, "run@example.com", "Run");
    let other = add_customer(&conn, "run-other@example.com", "Other");
    add_product(&conn, "P", 100);
    let o1 = place(&conn, x.id, "2027-04-02T00:00:00Z", &[("P", 3)]);
    let o2 = place(&conn, x.id, "2027-04-01T00:00:00Z", &[("P", 5)]);
    let o3 = place(&conn, x.id, "2027-04-03T00:00:00Z", &[("P", 9)]);
    let o4 = place(&conn, x.id, "2027-04-02T00:00:00Z", &[("P", 2)]);
    place(&conn, other.id, "2027-04-01T00:00:00Z", &[("P", 100)]);
    OrderRepository::new(&conn).set_status(o3, OrderStatus::Cancelled).expect("cancel");

    let rows = customer_running_units(&conn, x.id).expect("running units");
    let ids: Vec<i64> = rows.iter().map(|r| r.order_id).collect();
    assert_eq!(ids, vec![o2, o1, o4], "cancelled order excluded, ties broken by order id");
    let units: Vec<i64> = rows.iter().map(|r| r.units).collect();
    let running: Vec<i64> = rows.iter().map(|r| r.running_units).collect();
    assert_eq!(units, vec![5, 3, 2]);
    assert_eq!(running, vec![5, 8, 10]);
}
