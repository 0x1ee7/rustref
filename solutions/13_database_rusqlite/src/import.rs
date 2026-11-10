use rusqlite::Connection;

use crate::db::unit_of_work;
use crate::error::{Error, Result};
use crate::repo::ProductRepository;
use crate::types::Product;

/// Upserts `products` in batches of `batch_size`, each batch in its own
/// transaction. Returns the number of rows processed. Consumes a one-shot
/// iterator lazily; memory is bounded by `batch_size`.
///
/// `batch_size == 0` is [`Error::InvalidInput`]. A bad row aborts the import:
/// committed batches stay, the failing batch is rolled back, later rows are not read.
pub fn import_products(
    conn: &Connection,
    products: impl IntoIterator<Item = Product>,
    batch_size: usize,
) -> Result<usize> {
    if batch_size == 0 {
        return Err(Error::InvalidInput("batch size must be at least 1"));
    }

    let mut rows = products.into_iter();
    let mut processed = 0;
    loop {
        let batch: Vec<Product> = rows.by_ref().take(batch_size).collect();
        if batch.is_empty() {
            return Ok(processed);
        }
        unit_of_work(conn, |c| ProductRepository::new(c).upsert_many(&batch))?;
        processed += batch.len();
    }
}
