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
pub struct Sku {
    raw: String,
}

impl Sku {
    /// Minimum length in bytes.
    pub const MIN_LEN: usize = 4;
    /// Maximum length in bytes.
    pub const MAX_LEN: usize = 16;

    /// Validates `raw` as a SKU. No trimming is performed.
    pub fn new(raw: &str) -> Result<Sku, ProductError> {
        let well_formed = (Self::MIN_LEN..=Self::MAX_LEN).contains(&raw.len())
            && raw
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-');
        if well_formed {
            Ok(Sku {
                raw: raw.to_owned(),
            })
        } else {
            Err(ProductError::InvalidSku(raw.to_owned()))
        }
    }

    /// The SKU text.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl FromStr for Sku {
    type Err = ProductError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Sku::new(raw)
    }
}

impl fmt::Display for Sku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl AsRef<str> for Sku {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

/// Trims a product name and checks its bounds.
fn normalise_name(name: &str) -> Result<String, ProductError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ProductError::BlankName);
    }
    if trimmed.chars().count() > Product::MAX_NAME_CHARS {
        return Err(ProductError::NameTooLong {
            max: Product::MAX_NAME_CHARS,
        });
    }
    Ok(trimmed.to_owned())
}

/// A sellable product with a validated SKU, a trimmed name, a price and stock.
#[derive(Clone, Debug, PartialEq)]
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
    pub fn new(sku: Sku, name: &str, price: Money) -> Result<Product, ProductError> {
        Ok(Product {
            sku,
            name: normalise_name(name)?,
            price,
            quantity: 0,
        })
    }

    /// Builder: seeds the stock quantity.
    pub fn with_quantity(mut self, quantity: u32) -> Product {
        self.quantity = quantity;
        self
    }

    /// The SKU.
    pub fn sku(&self) -> &Sku {
        &self.sku
    }

    /// The trimmed name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The unit price.
    pub fn price(&self) -> Money {
        self.price
    }

    /// The units in stock.
    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    /// `true` when at least one unit is in stock.
    pub fn in_stock(&self) -> bool {
        self.quantity > 0
    }

    /// Replaces the name. On error the old name is kept.
    pub fn set_name(&mut self, name: &str) -> Result<(), ProductError> {
        self.name = normalise_name(name)?;
        Ok(())
    }

    /// Replaces the unit price.
    pub fn set_price(&mut self, price: Money) {
        self.price = price;
    }

    /// Replaces the stock quantity.
    pub fn set_quantity(&mut self, quantity: u32) {
        self.quantity = quantity;
    }

    /// Adds `count` units. Fails without changing stock on zero or overflow.
    pub fn restock(&mut self, count: u32) -> Result<(), ProductError> {
        if count == 0 {
            return Err(ProductError::ZeroRestock);
        }
        self.quantity = self
            .quantity
            .checked_add(count)
            .ok_or(ProductError::QuantityOverflow)?;
        Ok(())
    }
}
