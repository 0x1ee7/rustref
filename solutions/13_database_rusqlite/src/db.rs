use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::error::{Error, Result};

/// Opens a connection with the project policy applied: `foreign_keys = ON`,
/// a busy timeout of at least one second, and the schema untouched.
/// `":memory:"` is accepted.
pub fn connect(path: impl AsRef<Path>) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(conn)
}

/// One schema step. `version` is 1-based and must equal its position in the slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub statements: &'static [&'static str],
}

/// The shipped schema: v1 tables, v2 indexes.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_tables",
        statements: &[
            "CREATE TABLE customers (
                id INTEGER PRIMARY KEY,
                email TEXT NOT NULL UNIQUE COLLATE NOCASE,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL
            ) STRICT",
            "CREATE TABLE products (
                sku TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                unit_price_cents INTEGER NOT NULL CHECK (unit_price_cents >= 0)
            ) STRICT",
            "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER NOT NULL REFERENCES customers(id),
                placed_at INTEGER NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('pending', 'paid', 'shipped', 'cancelled'))
            ) STRICT",
            "CREATE TABLE order_lines (
                order_id INTEGER NOT NULL REFERENCES orders(id),
                line_no INTEGER NOT NULL,
                sku TEXT NOT NULL REFERENCES products(sku),
                quantity INTEGER NOT NULL CHECK (quantity >= 1),
                unit_price_cents INTEGER NOT NULL CHECK (unit_price_cents >= 0),
                PRIMARY KEY (order_id, line_no)
            ) STRICT",
        ],
    },
    Migration {
        version: 2,
        name: "create_indexes",
        statements: &[
            "CREATE INDEX orders_by_customer ON orders (customer_id, placed_at, id)",
            "CREATE INDEX orders_by_placed ON orders (placed_at, id)",
            "CREATE INDEX order_lines_by_sku ON order_lines (sku)",
        ],
    },
];

/// Applies every migration newer than `PRAGMA user_version`, each in its own
/// `BEGIN IMMEDIATE` transaction together with the version bump. Returns the
/// resulting `user_version`. Re-running is a no-op.
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<u32> {
    for (index, migration) in migrations.iter().enumerate() {
        let expected = u32::try_from(index + 1).expect("fewer than u32::MAX migrations");
        if migration.version != expected {
            return Err(Error::MigrationOrder { expected, found: migration.version });
        }
    }

    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let latest = migrations.last().map_or(0, |m| m.version);
    if current > latest {
        return Err(Error::SchemaTooNew { found: current, supported: latest });
    }

    for migration in migrations.iter().filter(|m| m.version > current) {
        apply(conn, migration)?;
    }

    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn apply(conn: &mut Connection, migration: &Migration) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let outcome: rusqlite::Result<()> = (|| {
        for sql in migration.statements {
            tx.execute_batch(sql)?;
        }
        tx.pragma_update(None, "user_version", migration.version)
    })();
    match outcome {
        // Dropping `tx` without commit rolls the whole migration back.
        Err(source) => Err(Error::Migration {
            version: migration.version,
            name: migration.name,
            source,
        }),
        Ok(()) => {
            tx.commit()?;
            Ok(())
        }
    }
}

/// Runs `f` atomically on `conn`.
///
/// Outside a transaction: `BEGIN IMMEDIATE`, commit on `Ok`, roll back on `Err`
/// or panic. Inside an existing transaction: a savepoint with the same rules.
/// The connection is never left in a transaction when this returns.
pub fn unit_of_work<T>(conn: &Connection, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    if conn.is_autocommit() {
        // Dropping an uncommitted `Transaction` rolls back, including during unwinding.
        let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
        let value = f(&tx)?;
        tx.commit()?;
        return Ok(value);
    }

    conn.execute_batch("SAVEPOINT uow_savepoint")?;
    let mut guard = SavepointGuard { conn, armed: true };
    let value = f(conn)?;
    conn.execute_batch("RELEASE uow_savepoint")?;
    guard.armed = false;
    Ok(value)
}

/// Rolls back and releases the savepoint unless disarmed; covers errors and panics.
struct SavepointGuard<'c> {
    conn: &'c Connection,
    armed: bool,
}

impl Drop for SavepointGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            // Nothing sensible to do with a failure while unwinding or returning an error.
            let _ = self
                .conn
                .execute_batch("ROLLBACK TO uow_savepoint; RELEASE uow_savepoint");
        }
    }
}
