//! Part 3 reference: streaming top-K with a bounded min-heap.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

use crate::Error;

/// A SKU ranked by units; a greater value is a better seller.
#[derive(PartialEq, Eq)]
struct Ranked {
    units: u64,
    sku: String,
}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        // More units ranks higher; on a tie, the lexicographically smaller SKU ranks higher.
        self.units
            .cmp(&other.units)
            .then_with(|| other.sku.cmp(&self.sku))
    }
}

impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Aggregates `(sku, units)` sales lines and returns the `k` best-selling SKUs as
/// `(sku, total_units)`, ordered by units descending, ties broken by SKU ascending.
///
/// # Errors
/// - [`Error::NegativeUnits`] for any line with negative units (validated even when `k == 0`).
/// - [`Error::Overflow`] if a SKU's total does not fit in `u64`.
pub fn top_k_products<I, S>(sales: I, k: usize) -> Result<Vec<(String, u64)>, Error>
where
    I: IntoIterator<Item = (S, i64)>,
    S: AsRef<str>,
{
    let mut totals: HashMap<String, u64> = HashMap::new();
    for (sku, units) in sales {
        let sku = sku.as_ref();
        let units = u64::try_from(units).map_err(|_| Error::NegativeUnits {
            sku: sku.to_string(),
            units,
        })?;
        match totals.get_mut(sku) {
            Some(total) => *total = total.checked_add(units).ok_or(Error::Overflow)?,
            None => {
                totals.insert(sku.to_string(), units);
            }
        }
    }

    // Min-heap of the k best seen so far: the root is the weakest kept seller.
    let mut best: BinaryHeap<Reverse<Ranked>> = BinaryHeap::with_capacity(k);
    for (sku, units) in totals {
        let candidate = Ranked { units, sku };
        if best.len() < k {
            best.push(Reverse(candidate));
        } else if best
            .peek()
            .is_some_and(|Reverse(weakest)| candidate.cmp(weakest).is_gt())
        {
            best.pop();
            best.push(Reverse(candidate));
        }
    }

    // `Reverse` flips the order, so ascending order here is best-first.
    Ok(best
        .into_sorted_vec()
        .into_iter()
        .map(|Reverse(ranked)| (ranked.sku, ranked.units))
        .collect())
}
