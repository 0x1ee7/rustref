use std::fmt;
use std::str::FromStr;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

use crate::error::{Error, Result};

/// Exact money as an integer count of cents. Stored as `INTEGER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money {
    cents: i64,
}

impl Money {
    /// Builds an amount from cents (may be negative; validation is the caller's job).
    pub const fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    /// The amount in cents.
    pub const fn cents(self) -> i64 {
        self.cents
    }

    /// `self + rhs`, or `None` on overflow.
    pub fn checked_add(self, rhs: Money) -> Option<Money> {
        self.cents.checked_add(rhs.cents).map(Self::from_cents)
    }

    /// `self * quantity`, or `None` on overflow.
    pub fn checked_mul(self, quantity: u32) -> Option<Money> {
        self.cents.checked_mul(i64::from(quantity)).map(Self::from_cents)
    }
}

/// Renders as a fixed two-decimal amount: `12.34`, `-0.05`, `0.00`.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.cents < 0 { "-" } else { "" };
        let abs = self.cents.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

impl ToSql for Money {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.cents))
    }
}

impl FromSql for Money {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(Self::from_cents)
    }
}

/// A UTC instant with microsecond precision. Stored as `INTEGER` microseconds
/// since the Unix epoch, so SQL ordering equals chronological ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTimestamp {
    ts: jiff::Timestamp,
}

impl UtcTimestamp {
    /// Wraps an instant, truncating sub-microsecond precision.
    pub fn from_timestamp(ts: jiff::Timestamp) -> Self {
        // Truncation keeps the value within the representable microsecond range.
        let ts = jiff::Timestamp::from_microsecond(ts.as_microsecond())
            .expect("microseconds of a valid timestamp are in range");
        Self { ts }
    }

    /// Builds from microseconds since the epoch; `None` if out of range.
    pub fn from_micros(micros: i64) -> Option<Self> {
        jiff::Timestamp::from_microsecond(micros).ok().map(|ts| Self { ts })
    }

    /// Microseconds since the Unix epoch.
    pub fn as_micros(self) -> i64 {
        self.ts.as_microsecond()
    }

    /// The underlying instant.
    pub fn as_timestamp(self) -> jiff::Timestamp {
        self.ts
    }
}

impl ToSql for UtcTimestamp {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_micros()))
    }
}

impl FromSql for UtcTimestamp {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let micros = i64::column_result(value)?;
        Self::from_micros(micros).ok_or(FromSqlError::OutOfRange(micros))
    }
}

/// Lifecycle state of an order. Stored as lowercase text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Cancelled,
}

impl OrderStatus {
    /// The database representation: `pending`, `paid`, `shipped`, `cancelled`.
    pub const fn as_str(self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Paid => "paid",
            OrderStatus::Shipped => "shipped",
            OrderStatus::Cancelled => "cancelled",
        }
    }
}

/// Parses the database representation; anything else is [`Error::UnknownStatus`].
impl FromStr for OrderStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(OrderStatus::Pending),
            "paid" => Ok(OrderStatus::Paid),
            "shipped" => Ok(OrderStatus::Shipped),
            "cancelled" => Ok(OrderStatus::Cancelled),
            other => Err(Error::UnknownStatus(other.to_owned())),
        }
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ToSql for OrderStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for OrderStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;
        text.parse().map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

/// A registered customer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Customer {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub created_at: UtcTimestamp,
}

/// A catalogue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub sku: String,
    pub name: String,
    pub unit_price: Money,
}

/// An order header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    pub id: i64,
    pub customer_id: i64,
    pub placed_at: UtcTimestamp,
    pub status: OrderStatus,
}

/// One line of an order. `unit_price` is the price snapshotted at order time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderLine {
    pub order_id: i64,
    pub line_no: i64,
    pub sku: String,
    pub quantity: u32,
    pub unit_price: Money,
}

/// A requested line when placing an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewLine<'a> {
    pub sku: &'a str,
    pub quantity: u32,
}

/// An order together with its lines, as returned by `OrderRepository::load_with_items`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderWithItems {
    pub order: Order,
    pub lines: Vec<OrderLine>,
}
