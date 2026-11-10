//! Part 4: `ShoppingCart` with collection traits and atomic batches.

use std::ops::Index;

use crate::money::{Currency, Money, MoneyError};
use crate::product::{Product, Sku};

/// Errors from cart operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CartError {
    /// A quantity of zero was given.
    #[error("quantity must be greater than zero")]
    ZeroQuantity,
    /// The product is priced in a different currency than the cart.
    #[error("product currency {found} does not match cart currency {expected}")]
    CurrencyMismatch { expected: Currency, found: Currency },
    /// The quantity would overflow `u32`.
    #[error("quantity overflows u32")]
    QuantityOverflow,
    /// The same SKU was added at a different unit price.
    #[error("SKU {sku} is already in the cart at a different price")]
    PriceChanged { sku: Sku },
    /// The SKU is not in the cart.
    #[error("SKU {sku} is not in the cart")]
    UnknownSku { sku: Sku },
    /// More units were requested than the cart holds.
    #[error("cannot remove {requested} of {sku}: only {available} in cart")]
    InsufficientQuantity { sku: Sku, requested: u32, available: u32 },
    /// A money operation failed.
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// One cart line: a SKU, the unit price it was added at, and a quantity.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct CartLine {
    sku: Sku,
    unit_price: Money,
    quantity: u32,
}

impl CartLine {
    /// The SKU.
    pub fn sku(&self) -> &Sku {
        todo!()
    }

    /// The unit price snapshot.
    pub fn unit_price(&self) -> Money {
        todo!()
    }

    /// Units on this line.
    pub fn quantity(&self) -> u32 {
        todo!()
    }

    /// `unit_price * quantity`.
    pub fn subtotal(&self) -> Result<Money, MoneyError> {
        todo!()
    }
}

/// A cart in one currency. Lines keep insertion order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ShoppingCart {
    currency: Currency,
    lines: Vec<CartLine>,
}

impl ShoppingCart {
    /// An empty cart priced in `currency`.
    pub fn new(_currency: Currency) -> ShoppingCart {
        todo!()
    }

    /// The cart currency.
    pub fn currency(&self) -> Currency {
        todo!()
    }

    /// Adds `quantity` units of `product`. See the README for the rules.
    pub fn add(&mut self, _product: &Product, _quantity: u32) -> Result<(), CartError> {
        todo!()
    }

    /// Removes `quantity` units of `sku`. Removing every unit deletes the line.
    pub fn remove(&mut self, _sku: &Sku, _quantity: u32) -> Result<(), CartError> {
        todo!()
    }

    /// Units of `sku` in the cart; zero when absent.
    pub fn quantity_of(&self, _sku: &Sku) -> u32 {
        todo!()
    }

    /// `true` when the cart has a line for `sku`.
    pub fn contains(&self, _sku: &Sku) -> bool {
        todo!()
    }

    /// Total units across all lines.
    pub fn units(&self) -> u64 {
        todo!()
    }

    /// `true` when the cart has no lines.
    pub fn is_empty(&self) -> bool {
        todo!()
    }

    /// Sum of all line subtotals.
    pub fn total(&self) -> Result<Money, CartError> {
        todo!()
    }

    /// Applies every `(product, quantity)` pair, or none of them.
    pub fn try_extend<'a, I>(&mut self, _items: I) -> Result<(), CartError>
    where
        I: IntoIterator<Item = (&'a Product, u32)>,
    {
        todo!()
    }
}

impl Index<&Sku> for ShoppingCart {
    type Output = u32;

    /// # Panics
    /// Panics with a message containing "no line" when `sku` is absent.
    fn index(&self, _sku: &Sku) -> &u32 {
        todo!()
    }
}

impl<'a> IntoIterator for &'a ShoppingCart {
    type Item = &'a CartLine;
    type IntoIter = std::slice::Iter<'a, CartLine>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

impl IntoIterator for ShoppingCart {
    type Item = CartLine;
    type IntoIter = std::vec::IntoIter<CartLine>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}
