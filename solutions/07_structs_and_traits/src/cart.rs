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
pub struct CartLine {
    sku: Sku,
    unit_price: Money,
    quantity: u32,
}

impl CartLine {
    /// The SKU.
    pub fn sku(&self) -> &Sku {
        &self.sku
    }

    /// The unit price snapshot.
    pub fn unit_price(&self) -> Money {
        self.unit_price
    }

    /// Units on this line.
    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    /// `unit_price * quantity`.
    pub fn subtotal(&self) -> Result<Money, MoneyError> {
        self.unit_price.checked_mul(i64::from(self.quantity))
    }
}

/// A cart in one currency. Lines keep insertion order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppingCart {
    currency: Currency,
    lines: Vec<CartLine>,
}

impl ShoppingCart {
    /// An empty cart priced in `currency`.
    pub fn new(currency: Currency) -> ShoppingCart {
        ShoppingCart {
            currency,
            lines: Vec::new(),
        }
    }

    /// The cart currency.
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Index of the line for `sku`, if any.
    fn position(&self, sku: &Sku) -> Option<usize> {
        self.lines.iter().position(|line| line.sku == *sku)
    }

    /// Adds `quantity` units of `product`. See the README for the rules.
    pub fn add(&mut self, product: &Product, quantity: u32) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::ZeroQuantity);
        }
        let price = product.price();
        if price.currency() != self.currency {
            return Err(CartError::CurrencyMismatch {
                expected: self.currency,
                found: price.currency(),
            });
        }
        match self.position(product.sku()) {
            Some(index) => {
                let line = &mut self.lines[index];
                if line.unit_price != price {
                    return Err(CartError::PriceChanged {
                        sku: product.sku().clone(),
                    });
                }
                line.quantity = line
                    .quantity
                    .checked_add(quantity)
                    .ok_or(CartError::QuantityOverflow)?;
            }
            None => self.lines.push(CartLine {
                sku: product.sku().clone(),
                unit_price: price,
                quantity,
            }),
        }
        Ok(())
    }

    /// Removes `quantity` units of `sku`. Removing every unit deletes the line.
    pub fn remove(&mut self, sku: &Sku, quantity: u32) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::ZeroQuantity);
        }
        let index = self.position(sku).ok_or_else(|| CartError::UnknownSku {
            sku: sku.clone(),
        })?;
        let line = &mut self.lines[index];
        if quantity > line.quantity {
            return Err(CartError::InsufficientQuantity {
                sku: sku.clone(),
                requested: quantity,
                available: line.quantity,
            });
        }
        line.quantity -= quantity;
        if line.quantity == 0 {
            self.lines.remove(index);
        }
        Ok(())
    }

    /// Units of `sku` in the cart; zero when absent.
    pub fn quantity_of(&self, sku: &Sku) -> u32 {
        self.position(sku).map_or(0, |index| self.lines[index].quantity)
    }

    /// `true` when the cart has a line for `sku`.
    pub fn contains(&self, sku: &Sku) -> bool {
        self.position(sku).is_some()
    }

    /// Total units across all lines.
    pub fn units(&self) -> u64 {
        self.lines.iter().map(|line| u64::from(line.quantity)).sum()
    }

    /// `true` when the cart has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Sum of all line subtotals.
    pub fn total(&self) -> Result<Money, CartError> {
        let mut total = Money::zero(self.currency);
        for line in &self.lines {
            total = total.checked_add(line.subtotal()?)?;
        }
        Ok(total)
    }

    /// Applies every `(product, quantity)` pair, or none of them.
    pub fn try_extend<'a, I>(&mut self, items: I) -> Result<(), CartError>
    where
        I: IntoIterator<Item = (&'a Product, u32)>,
    {
        // Stage on a copy so a failing item leaves `self` untouched.
        let mut staged = self.clone();
        for (product, quantity) in items {
            staged.add(product, quantity)?;
        }
        *self = staged;
        Ok(())
    }
}

impl Index<&Sku> for ShoppingCart {
    type Output = u32;

    /// # Panics
    /// Panics with a message containing "no line" when `sku` is absent.
    fn index(&self, sku: &Sku) -> &u32 {
        match self.position(sku) {
            Some(index) => &self.lines[index].quantity,
            None => panic!("SKU {sku} has no line in the cart"),
        }
    }
}

impl<'a> IntoIterator for &'a ShoppingCart {
    type Item = &'a CartLine;
    type IntoIter = std::slice::Iter<'a, CartLine>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.iter()
    }
}

impl IntoIterator for ShoppingCart {
    type Item = CartLine;
    type IntoIter = std::vec::IntoIter<CartLine>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}
