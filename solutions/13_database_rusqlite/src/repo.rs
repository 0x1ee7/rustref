use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, ErrorCode, OptionalExtension, Params, Row, params};

use crate::db::unit_of_work;
use crate::error::{Error, Result};
use crate::types::{
    Customer, Money, NewLine, Order, OrderLine, OrderStatus, OrderWithItems, Product, UtcTimestamp,
};

/// Customer persistence.
pub struct CustomerRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> CustomerRepository<'conn> {
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Inserts and returns the persisted row. Duplicate email (any case) is
    /// [`Error::DuplicateEmail`].
    pub fn add(&self, email: &str, name: &str, created_at: UtcTimestamp) -> Result<Customer> {
        let inserted = self
            .conn
            .prepare_cached("INSERT INTO customers (email, name, created_at) VALUES (?1, ?2, ?3)")?
            .execute(params![email, name, created_at]);
        match inserted {
            Ok(_) => Ok(Customer {
                id: self.conn.last_insert_rowid(),
                email: email.to_owned(),
                name: name.to_owned(),
                created_at,
            }),
            // The unique index on `customers.email` is the only place this message can come from.
            Err(rusqlite::Error::SqliteFailure(err, Some(msg)))
                if err.code == ErrorCode::ConstraintViolation && msg.contains("customers.email") =>
            {
                Err(Error::DuplicateEmail(email.to_owned()))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn get(&self, id: i64) -> Result<Option<Customer>> {
        let found = self
            .conn
            .query_row(
                "SELECT id, email, name, created_at FROM customers WHERE id = ?1",
                params![id],
                customer_from_row,
            )
            .optional()?;
        Ok(found)
    }

    /// Case-insensitive email lookup (the column collation does the work).
    pub fn get_by_email(&self, email: &str) -> Result<Option<Customer>> {
        let found = self
            .conn
            .query_row(
                "SELECT id, email, name, created_at FROM customers WHERE email = ?1",
                params![email],
                customer_from_row,
            )
            .optional()?;
        Ok(found)
    }

    /// Case-insensitive literal substring match on name. An empty term is
    /// [`Error::InvalidInput`]. `%` and `_` match literally.
    pub fn search_by_name(&self, term: &str) -> Result<Vec<Customer>> {
        if term.is_empty() {
            return Err(Error::InvalidInput("search term must not be empty"));
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, email, name, created_at FROM customers
             WHERE instr(lower(name), lower(?1)) > 0
             ORDER BY id",
        )?;
        let rows = stmt.query_map(params![term], customer_from_row)?;
        let customers = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(customers)
    }
}

fn customer_from_row(row: &Row<'_>) -> rusqlite::Result<Customer> {
    Ok(Customer {
        id: row.get(0)?,
        email: row.get(1)?,
        name: row.get(2)?,
        created_at: row.get(3)?,
    })
}

/// Catalogue persistence.
pub struct ProductRepository<'conn> {
    conn: &'conn Connection,
}

impl<'conn> ProductRepository<'conn> {
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Insert or update by SKU. A negative price is [`Error::InvalidPrice`].
    pub fn upsert(&self, sku: &str, name: &str, unit_price: Money) -> Result<()> {
        check_price(sku, unit_price)?;
        self.conn
            .prepare_cached(
                "INSERT INTO products (sku, name, unit_price_cents) VALUES (?1, ?2, ?3)
                 ON CONFLICT (sku) DO UPDATE SET
                     name = excluded.name,
                     unit_price_cents = excluded.unit_price_cents",
            )?
            .execute(params![sku, name, unit_price])?;
        Ok(())
    }

    /// Validates every product before writing any of them, then writes them in one unit of work.
    pub fn upsert_many(&self, products: &[Product]) -> Result<()> {
        for product in products {
            check_price(&product.sku, product.unit_price)?;
        }
        unit_of_work(self.conn, |conn| {
            let repo = ProductRepository::new(conn);
            for product in products {
                repo.upsert(&product.sku, &product.name, product.unit_price)?;
            }
            Ok(())
        })
    }

    pub fn get(&self, sku: &str) -> Result<Option<Product>> {
        let found = self
            .conn
            .query_row(
                "SELECT sku, name, unit_price_cents FROM products WHERE sku = ?1",
                params![sku],
                product_from_row,
            )
            .optional()?;
        Ok(found)
    }
}

fn check_price(sku: &str, price: Money) -> Result<()> {
    if price.cents() < 0 {
        return Err(Error::InvalidPrice {
            sku: sku.to_owned(),
            reason: "price must not be negative",
        });
    }
    Ok(())
}

fn product_from_row(row: &Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        sku: row.get(0)?,
        name: row.get(1)?,
        unit_price: row.get(2)?,
    })
}

/// Opaque keyset cursor for `(placed_at, id)` ordering. Encoded as an unpadded,
/// URL-safe token; the format is private to this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderCursor {
    pub placed_at: UtcTimestamp,
    pub order_id: i64,
}

const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

impl OrderCursor {
    /// URL-safe token with no `=` padding.
    pub fn encode(&self) -> String {
        let payload = format!("{}:{}", self.placed_at.as_micros(), self.order_id);
        base64url_encode(payload.as_bytes())
    }

    /// Any malformed token is [`Error::InvalidCursor`].
    ///
    /// Decoding is strict: the token must be canonical, so re-encoding the
    /// decoded value must reproduce it byte for byte.
    pub fn decode(token: &str) -> Result<Self> {
        let bytes = base64url_decode(token).ok_or(Error::InvalidCursor)?;
        let text = String::from_utf8(bytes).map_err(|_| Error::InvalidCursor)?;
        let (micros, order_id) = text.split_once(':').ok_or(Error::InvalidCursor)?;
        let placed_at = UtcTimestamp::from_micros(parse_strict_int(micros)?).ok_or(Error::InvalidCursor)?;
        let cursor = OrderCursor {
            placed_at,
            order_id: parse_strict_int(order_id)?,
        };
        if cursor.encode() != token {
            return Err(Error::InvalidCursor);
        }
        Ok(cursor)
    }
}

/// Accepts an optional leading `-` followed by ASCII digits only (no `+`, no spaces).
fn parse_strict_int(text: &str) -> Result<i64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidCursor);
    }
    text.parse().map_err(|_| Error::InvalidCursor)
}

fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(chunk.get(1).copied().unwrap_or(0));
        let b2 = u32::from(chunk.get(2).copied().unwrap_or(0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        // A 1-byte chunk yields 2 characters, 2 bytes yield 3, 3 bytes yield 4.
        for i in 0..=chunk.len() {
            let shift = 18 - 6 * u32::try_from(i).expect("at most 4 characters per chunk");
            let index = (n >> shift) & 0x3f;
            out.push(char::from(B64_ALPHABET[index as usize]));
        }
    }
    out
}

fn base64url_decode(token: &str) -> Option<Vec<u8>> {
    if token.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(token.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for byte in token.bytes() {
        let value = B64_ALPHABET.iter().position(|&a| a == byte)?;
        acc = (acc << 6) | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from(acc >> bits).ok()?);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// One keyset page. `next_cursor` is `Some` when the page is full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPage {
    pub items: Vec<Order>,
    pub next_cursor: Option<OrderCursor>,
}

/// Order persistence.
pub struct OrderRepository<'conn> {
    conn: &'conn Connection,
}

const ORDER_COLUMNS: &str = "SELECT id, customer_id, placed_at, status FROM orders";

impl<'conn> OrderRepository<'conn> {
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    /// Places an order atomically and returns its id. Prices are snapshotted
    /// from the catalogue. See README Part 4 for every error case.
    pub fn place(&self, customer_id: i64, placed_at: UtcTimestamp, lines: &[NewLine<'_>]) -> Result<i64> {
        if lines.is_empty() {
            return Err(Error::InvalidInput("an order needs at least one line"));
        }
        if lines.iter().any(|line| line.quantity == 0) {
            return Err(Error::InvalidInput("line quantity must be at least 1"));
        }

        unit_of_work(self.conn, |conn| {
            let customer_exists = conn
                .query_row("SELECT 1 FROM customers WHERE id = ?1", params![customer_id], |_| Ok(()))
                .optional()?
                .is_some();
            if !customer_exists {
                return Err(Error::UnknownCustomer(customer_id));
            }

            let mut prices = Vec::with_capacity(lines.len());
            let mut unknown = BTreeSet::new();
            for line in lines {
                let price = conn
                    .query_row(
                        "SELECT unit_price_cents FROM products WHERE sku = ?1",
                        params![line.sku],
                        |row| row.get::<_, Money>(0),
                    )
                    .optional()?;
                match price {
                    Some(price) => prices.push(price),
                    None => {
                        unknown.insert(line.sku.to_owned());
                    }
                }
            }
            if !unknown.is_empty() {
                return Err(Error::UnknownProducts(unknown.into_iter().collect()));
            }

            conn.prepare_cached("INSERT INTO orders (customer_id, placed_at, status) VALUES (?1, ?2, ?3)")?
                .execute(params![customer_id, placed_at, OrderStatus::Pending])?;
            let order_id = conn.last_insert_rowid();

            let mut insert = conn.prepare_cached(
                "INSERT INTO order_lines (order_id, line_no, sku, quantity, unit_price_cents)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            let mut line_no = 0_i64;
            for (line, price) in lines.iter().zip(prices) {
                line_no += 1;
                insert.execute(params![order_id, line_no, line.sku, line.quantity, price])?;
            }
            Ok(order_id)
        })
    }

    pub fn get(&self, order_id: i64) -> Result<Option<Order>> {
        let found = self
            .conn
            .query_row(
                &format!("{ORDER_COLUMNS} WHERE id = ?1"),
                params![order_id],
                order_from_row,
            )
            .optional()?;
        Ok(found)
    }

    /// Unknown id is [`Error::OrderNotFound`].
    pub fn set_status(&self, order_id: i64, status: OrderStatus) -> Result<()> {
        let changed = self
            .conn
            .prepare_cached("UPDATE orders SET status = ?1 WHERE id = ?2")?
            .execute(params![status, order_id])?;
        if changed == 0 {
            return Err(Error::OrderNotFound(order_id));
        }
        Ok(())
    }

    /// Lines ordered by `line_no`.
    pub fn lines(&self, order_id: i64) -> Result<Vec<OrderLine>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT order_id, line_no, sku, quantity, unit_price_cents
             FROM order_lines WHERE order_id = ?1 ORDER BY line_no",
        )?;
        let rows = stmt.query_map(params![order_id], line_from_row)?;
        let lines = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(lines)
    }

    /// Keyset page after `after` in `(placed_at, id)` order. `limit == 0` is
    /// [`Error::InvalidInput`].
    pub fn page_keyset(&self, after: Option<&OrderCursor>, limit: usize) -> Result<OrderPage> {
        if limit == 0 {
            return Err(Error::InvalidInput("page limit must be at least 1"));
        }
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let items = match after {
            None => fetch_orders(
                self.conn,
                &format!("{ORDER_COLUMNS} ORDER BY placed_at, id LIMIT ?1"),
                params![limit],
            )?,
            Some(cursor) => fetch_orders(
                self.conn,
                &format!(
                    "{ORDER_COLUMNS} WHERE (placed_at, id) > (?1, ?2) ORDER BY placed_at, id LIMIT ?3"
                ),
                params![cursor.placed_at, cursor.order_id, limit],
            )?,
        };
        let full = items.len() == usize::try_from(limit).unwrap_or(usize::MAX);
        let next_cursor = if full {
            items.last().map(|o| OrderCursor {
                placed_at: o.placed_at,
                order_id: o.id,
            })
        } else {
            None
        };
        Ok(OrderPage { items, next_cursor })
    }

    /// Positional page. `limit == 0` is [`Error::InvalidInput`].
    pub fn page_offset(&self, offset: usize, limit: usize) -> Result<Vec<Order>> {
        if limit == 0 {
            return Err(Error::InvalidInput("page limit must be at least 1"));
        }
        fetch_orders(
            self.conn,
            &format!("{ORDER_COLUMNS} ORDER BY placed_at, id LIMIT ?1 OFFSET ?2"),
            params![
                i64::try_from(limit).unwrap_or(i64::MAX),
                i64::try_from(offset).unwrap_or(i64::MAX)
            ],
        )
    }

    /// Every order of `customer_id` (cancelled included) with its lines, in
    /// `(placed_at, id)` order, using a bounded number of statements.
    pub fn load_with_items(&self, customer_id: i64) -> Result<Vec<OrderWithItems>> {
        let orders = fetch_orders(
            self.conn,
            &format!("{ORDER_COLUMNS} WHERE customer_id = ?1 ORDER BY placed_at, id"),
            params![customer_id],
        )?;

        let mut stmt = self.conn.prepare_cached(
            "SELECT l.order_id, l.line_no, l.sku, l.quantity, l.unit_price_cents
             FROM order_lines AS l
             JOIN orders AS o ON o.id = l.order_id
             WHERE o.customer_id = ?1
             ORDER BY l.order_id, l.line_no",
        )?;
        let rows = stmt.query_map(params![customer_id], line_from_row)?;
        let mut lines_by_order: HashMap<i64, Vec<OrderLine>> = HashMap::new();
        for line in rows {
            let line = line?;
            lines_by_order.entry(line.order_id).or_default().push(line);
        }

        let loaded = orders
            .into_iter()
            .map(|order| {
                let lines = lines_by_order.remove(&order.id).unwrap_or_default();
                OrderWithItems { order, lines }
            })
            .collect();
        Ok(loaded)
    }
}

fn fetch_orders(conn: &Connection, sql: &str, params: impl Params) -> Result<Vec<Order>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, order_from_row)?;
    let orders = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(orders)
}

fn order_from_row(row: &Row<'_>) -> rusqlite::Result<Order> {
    Ok(Order {
        id: row.get(0)?,
        customer_id: row.get(1)?,
        placed_at: row.get(2)?,
        status: row.get(3)?,
    })
}

fn line_from_row(row: &Row<'_>) -> rusqlite::Result<OrderLine> {
    Ok(OrderLine {
        order_id: row.get(0)?,
        line_no: row.get(1)?,
        sku: row.get(2)?,
        quantity: row.get(3)?,
        unit_price: row.get(4)?,
    })
}
