//! Part 5 reference: stock reconciliation, volume price tiers and SLA banding.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use crate::Error;

/// An amount of money in minor units (cents). Integer arithmetic only; no floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money {
    cents: i64,
}

impl Money {
    /// Wraps an amount given in cents.
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    /// The amount in cents.
    pub fn cents(&self) -> i64 {
        self.cents
    }

    /// `self * qty`, or `None` on overflow.
    pub fn checked_mul(self, qty: u32) -> Option<Money> {
        self.cents
            .checked_mul(i64::from(qty))
            .map(Money::from_cents)
    }
}

/// Renders as `major.minor` with two decimals, e.g. `12.05`.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.cents < 0 { "-" } else { "" };
        let abs = self.cents.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

/// The result of comparing a warehouse count with the system of record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    /// SKUs expected but absent from the count.
    pub missing: BTreeSet<String>,
    /// SKUs counted but not expected.
    pub unexpected: BTreeSet<String>,
    /// SKUs in both with differing quantities, mapped to `counted - expected`.
    pub mismatched: BTreeMap<String, i64>,
}

impl Reconciliation {
    /// `true` iff all three collections are empty.
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty() && self.mismatched.is_empty()
    }
}

/// Compares expected against counted stock.
///
/// # Errors
/// [`Error::NegativeQuantity`] for any negative quantity on either side. If several
/// are negative, the smallest SKU is reported, independent of map iteration order.
pub fn reconcile(
    expected: &HashMap<String, i64>,
    counted: &HashMap<String, i64>,
) -> Result<Reconciliation, Error> {
    let worst = expected
        .iter()
        .chain(counted.iter())
        .filter(|(_, quantity)| **quantity < 0)
        .min_by(|a, b| a.0.cmp(b.0));
    if let Some((sku, &quantity)) = worst {
        return Err(Error::NegativeQuantity {
            sku: sku.clone(),
            quantity,
        });
    }

    let mut missing = BTreeSet::new();
    let mut mismatched = BTreeMap::new();
    for (sku, &expected_qty) in expected {
        match counted.get(sku) {
            None => {
                missing.insert(sku.clone());
            }
            Some(&counted_qty) if counted_qty != expected_qty => {
                // Both sides are non-negative, so the difference fits in i64.
                mismatched.insert(sku.clone(), counted_qty - expected_qty);
            }
            Some(_) => {}
        }
    }
    let unexpected = counted
        .keys()
        .filter(|sku| !expected.contains_key(*sku))
        .cloned()
        .collect();

    Ok(Reconciliation {
        missing,
        unexpected,
        mismatched,
    })
}

/// Volume price tiers keyed by their minimum quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceTiers {
    tiers: BTreeMap<u32, Money>,
}

impl PriceTiers {
    /// Builds the tier table. Tiers may be given in any order.
    ///
    /// # Errors
    /// [`Error::NoPriceTiers`] when empty; [`Error::FirstTierMustStartAtOne`] when
    /// the lowest `min_qty` is not 1; [`Error::DuplicateTier`] for a repeated
    /// `min_qty`; [`Error::NonPositivePrice`] for a price `<= 0`.
    pub fn new(tiers: impl IntoIterator<Item = (u32, Money)>) -> Result<Self, Error> {
        let mut table = BTreeMap::new();
        for (min_qty, price) in tiers {
            if price.cents() <= 0 {
                return Err(Error::NonPositivePrice);
            }
            if table.insert(min_qty, price).is_some() {
                return Err(Error::DuplicateTier(min_qty));
            }
        }
        if table.is_empty() {
            return Err(Error::NoPriceTiers);
        }
        if !table.contains_key(&1) {
            return Err(Error::FirstTierMustStartAtOne);
        }
        Ok(Self { tiers: table })
    }

    /// Price per unit for `qty`: the tier with the largest `min_qty <= qty` applies.
    ///
    /// # Errors
    /// [`Error::ZeroQuantity`] if `qty == 0`.
    pub fn unit_price(&self, qty: u32) -> Result<Money, Error> {
        if qty == 0 {
            return Err(Error::ZeroQuantity);
        }
        let (_, price) = self
            .tiers
            .range(..=qty)
            .next_back()
            .expect("the first tier starts at 1, so one always applies");
        Ok(*price)
    }

    /// `unit_price(qty) * qty`, exact.
    ///
    /// # Errors
    /// [`Error::ZeroQuantity`] if `qty == 0`; [`Error::Overflow`] if the total overflows.
    pub fn line_total(&self, qty: u32) -> Result<Money, Error> {
        let unit = self.unit_price(qty)?;
        unit.checked_mul(qty).ok_or(Error::Overflow)
    }
}

/// Support SLA band for a response time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaTier {
    Critical,
    High,
    Normal,
    Low,
}

/// Bands a response time in minutes: `<= 15` Critical, `<= 60` High, `<= 240`
/// Normal, otherwise Low (including infinity). Upper bounds are inclusive.
///
/// # Errors
/// [`Error::InvalidResponseTime`] for a negative or NaN value.
pub fn sla_tier(response_minutes: f64) -> Result<SlaTier, Error> {
    if response_minutes.is_nan() || response_minutes < 0.0 {
        return Err(Error::InvalidResponseTime(response_minutes));
    }
    Ok(if response_minutes <= 15.0 {
        SlaTier::Critical
    } else if response_minutes <= 60.0 {
        SlaTier::High
    } else if response_minutes <= 240.0 {
        SlaTier::Normal
    } else {
        SlaTier::Low
    })
}
