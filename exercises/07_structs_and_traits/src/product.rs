//! Part 2: validated SKU newtype and product with setters and a builder.

use std::fmt;
use std::str::FromStr;

use crate::money::Money;

/// Errors from [`Sku`] and [`Product`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProductError {
    /// The SKU is not 4–16 characters of `A-Z`, `0-9` or `-`.
    #[error("invalid SKU {0:?}: expected 4-16 characters of A-Z, 0-9 or '-'")]
    InvalidSku(String),
    /// The name is empty after trimming.
    #[error("product name is blank")]
    BlankName,
    /// The name is longer than the maximum number of characters.
    #[error("product name is longer than {max} characters")]
    NameTooLong { max: usize },
    /// `restock` was called with zero.
    #[error("restock count must be greater than zero")]
    ZeroRestock,
    /// Adding stock would overflow `u32`.
    #[error("stock quantity would overflow u32")]
    QuantityOverflow,
}

/// A validated stock-keeping unit. Construct once; the rest of the code trusts it.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[allow(dead_code)]
pub struct Sku {
    raw: String,
}

impl Sku {
    /// Minimum length in bytes.
    pub const MIN_LEN: usize = 4;
    /// Maximum length in bytes.
    pub const MAX_LEN: usize = 16;

    /// Validates `raw` as a SKU. No trimming is performed.
    pub fn new(_raw: &str) -> Result<Sku, ProductError> {
        todo!()
    }

    /// The SKU text.
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl FromStr for Sku {
    type Err = ProductError;

    fn from_str(_raw: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl fmt::Display for Sku {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl AsRef<str> for Sku {
    fn as_ref(&self) -> &str {
        todo!()
    }
}

/// A sellable product with a validated SKU, a trimmed name, a price and stock.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct Product {
    sku: Sku,
    name: String,
    price: Money,
    quantity: u32,
}

impl Product {
    /// Maximum name length in characters.
    pub const MAX_NAME_CHARS: usize = 100;

    /// Creates a product with zero stock. The name is trimmed.
    pub fn new(_sku: Sku, _name: &str, _price: Money) -> Result<Product, ProductError> {
        todo!()
    }

    /// Builder: seeds the stock quantity.
    pub fn with_quantity(self, _quantity: u32) -> Product {
        todo!()
    }

    /// The SKU.
    pub fn sku(&self) -> &Sku {
        todo!()
    }

    /// The trimmed name.
    pub fn name(&self) -> &str {
        todo!()
    }

    /// The unit price.
    pub fn price(&self) -> Money {
        todo!()
    }

    /// The units in stock.
    pub fn quantity(&self) -> u32 {
        todo!()
    }

    /// `true` when at least one unit is in stock.
    pub fn in_stock(&self) -> bool {
        todo!()
    }

    /// Replaces the name. On error the old name is kept.
    pub fn set_name(&mut self, _name: &str) -> Result<(), ProductError> {
        todo!()
    }

    /// Replaces the unit price.
    pub fn set_price(&mut self, _price: Money) {
        todo!()
    }

    /// Replaces the stock quantity.
    pub fn set_quantity(&mut self, _quantity: u32) {
        todo!()
    }

    /// Adds `count` units. Fails without changing stock on zero or overflow.
    pub fn restock(&mut self, _count: u32) -> Result<(), ProductError> {
        todo!()
    }
}
