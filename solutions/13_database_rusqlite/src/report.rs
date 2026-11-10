use rusqlite::{Connection, Row, params};

use crate::error::Result;
use crate::types::{Money, UtcTimestamp};

/// Units and distinct orders per SKU (cancelled orders excluded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductSales {
    pub sku: String,
    pub units_sold: i64,
    pub order_count: i64,
}

/// Revenue per customer (cancelled orders excluded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerRevenue {
    pub customer_id: i64,
    pub name: String,
    pub order_count: i64,
    pub total: Money,
}

/// Customer ranked by non-cancelled order count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedCustomer {
    pub customer_id: i64,
    pub name: String,
    pub order_count: i64,
    pub rank: i64,
}

/// One non-cancelled order with a running unit total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunningUnits {
    pub order_id: i64,
    pub placed_at: UtcTimestamp,
    pub units: i64,
    pub running_units: i64,
}

/// Sorted by `units_sold` descending, then `sku` ascending.
pub fn units_sold_by_product(conn: &Connection) -> Result<Vec<ProductSales>> {
    let mut stmt = conn.prepare_cached(
        "SELECT l.sku, SUM(l.quantity), COUNT(DISTINCT l.order_id)
         FROM order_lines AS l
         JOIN orders AS o ON o.id = l.order_id
         WHERE o.status <> 'cancelled'
         GROUP BY l.sku
         ORDER BY SUM(l.quantity) DESC, l.sku ASC",
    )?;
    let rows = stmt.query_map([], |row: &Row<'_>| {
        Ok(ProductSales {
            sku: row.get(0)?,
            units_sold: row.get(1)?,
            order_count: row.get(2)?,
        })
    })?;
    let report = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(report)
}

/// Sorted by `customer_id`. Customers without non-cancelled orders are omitted.
pub fn revenue_by_customer(conn: &Connection) -> Result<Vec<CustomerRevenue>> {
    let mut stmt = conn.prepare_cached(
        "SELECT o.customer_id, c.name, COUNT(DISTINCT o.id), SUM(l.quantity * l.unit_price_cents)
         FROM orders AS o
         JOIN customers AS c ON c.id = o.customer_id
         JOIN order_lines AS l ON l.order_id = o.id
         WHERE o.status <> 'cancelled'
         GROUP BY o.customer_id, c.name
         ORDER BY o.customer_id",
    )?;
    let rows = stmt.query_map([], |row: &Row<'_>| {
        Ok(CustomerRevenue {
            customer_id: row.get(0)?,
            name: row.get(1)?,
            order_count: row.get(2)?,
            total: Money::from_cents(row.get(3)?),
        })
    })?;
    let report = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(report)
}

/// Every customer, ranked with SQL `RANK()`. Sorted by rank, then name.
pub fn rank_customers_by_orders(conn: &Connection) -> Result<Vec<RankedCustomer>> {
    let mut stmt = conn.prepare_cached(
        "WITH counts AS (
             SELECT c.id AS customer_id, c.name AS name,
                    (SELECT COUNT(*) FROM orders AS o
                      WHERE o.customer_id = c.id AND o.status <> 'cancelled') AS order_count
             FROM customers AS c
         )
         SELECT customer_id, name, order_count,
                RANK() OVER (ORDER BY order_count DESC) AS rnk
         FROM counts
         ORDER BY rnk, name",
    )?;
    let rows = stmt.query_map([], |row: &Row<'_>| {
        Ok(RankedCustomer {
            customer_id: row.get(0)?,
            name: row.get(1)?,
            order_count: row.get(2)?,
            rank: row.get(3)?,
        })
    })?;
    let report = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(report)
}

/// Non-cancelled orders of one customer, ordered by `(placed_at, order_id)`.
pub fn customer_running_units(conn: &Connection, customer_id: i64) -> Result<Vec<RunningUnits>> {
    let mut stmt = conn.prepare_cached(
        "WITH per_order AS (
             SELECT o.id AS order_id, o.placed_at AS placed_at, SUM(l.quantity) AS units
             FROM orders AS o
             JOIN order_lines AS l ON l.order_id = o.id
             WHERE o.customer_id = ?1 AND o.status <> 'cancelled'
             GROUP BY o.id, o.placed_at
         )
         SELECT order_id, placed_at, units,
                SUM(units) OVER (ORDER BY placed_at, order_id) AS running
         FROM per_order
         ORDER BY placed_at, order_id",
    )?;
    let rows = stmt.query_map(params![customer_id], |row: &Row<'_>| {
        Ok(RunningUnits {
            order_id: row.get(0)?,
            placed_at: row.get(1)?,
            units: row.get(2)?,
            running_units: row.get(3)?,
        })
    })?;
    let report = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(report)
}
