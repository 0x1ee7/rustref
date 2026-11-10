//! Part 3: streaming top-K product ranking over sales lines.

use crate::Error;

/// Aggregates `(sku, units)` sales lines and returns the `k` best-selling SKUs as
/// `(sku, total_units)`, ordered by units descending, ties broken by SKU ascending.
///
/// Consumes the iterator exactly once; the input may be far longer than the
/// number of distinct SKUs.
///
/// # Errors
/// - [`Error::NegativeUnits`] for any line with negative units (validated even when `k == 0`).
/// - [`Error::Overflow`] if a SKU's total does not fit in `u64`.
pub fn top_k_products<I, S>(_sales: I, _k: usize) -> Result<Vec<(String, u64)>, Error>
where
    I: IntoIterator<Item = (S, i64)>,
    S: AsRef<str>,
{
    todo!()
}
