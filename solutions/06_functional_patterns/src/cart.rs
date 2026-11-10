//! Part 1 · Redux-style cart reducer (enums as ADTs) and Part 4 fold over actions.

use thiserror::Error;

/// Read-only coupon table: `(code, percent off)`.
pub const COUPONS: &[(&str, u8)] = &[("SAVE10", 10), ("HALF", 50)];

/// One line of the cart. Price is in cents, never floating point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItem {
    pub sku: String,
    pub name: String,
    pub unit_price_cents: u64,
    pub quantity: u32,
}

/// Immutable cart state. `Default` is the empty cart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cart {
    pub items: Vec<LineItem>,
    pub coupon: Option<String>,
}

/// Every state transition the cart understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    AddItem {
        sku: String,
        name: String,
        unit_price_cents: u64,
        quantity: u32,
    },
    RemoveItem {
        sku: String,
    },
    SetQuantity {
        sku: String,
        quantity: u32,
    },
    /// `None` clears the coupon.
    ApplyCoupon(Option<String>),
    ClearCart,
}

/// Reducer and arithmetic failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CartError {
    /// `AddItem` with `quantity == 0`.
    #[error("quantity must be at least 1")]
    ZeroQuantity,
    /// `SetQuantity` for a sku that is not in the cart.
    #[error("unknown sku '{0}'")]
    UnknownSku(String),
    /// `ApplyCoupon` with a code not in `COUPONS`.
    #[error("unknown coupon '{0}'")]
    UnknownCoupon(String),
    /// Quantity or money arithmetic overflowed.
    #[error("arithmetic overflow")]
    Overflow,
}

/// Percent off for a known coupon code (case-sensitive), `None` otherwise.
pub fn coupon_percent(code: &str) -> Option<u8> {
    COUPONS
        .iter()
        .find(|(known, _)| *known == code)
        .map(|&(_, percent)| percent)
}

/// Pure reducer: returns the next cart. Neither `cart` nor `action` is modified.
///
/// Matches on `Action` exhaustively (no wildcard arm). See `README.md` Part 1.
pub fn reduce(cart: &Cart, action: &Action) -> Result<Cart, CartError> {
    let mut next = cart.clone();
    match action {
        Action::AddItem {
            sku,
            name,
            unit_price_cents,
            quantity,
        } => {
            if *quantity == 0 {
                return Err(CartError::ZeroQuantity);
            }
            if let Some(existing) = next.items.iter_mut().find(|line| line.sku == *sku) {
                existing.quantity = existing
                    .quantity
                    .checked_add(*quantity)
                    .ok_or(CartError::Overflow)?;
            } else {
                next.items.push(LineItem {
                    sku: sku.clone(),
                    name: name.clone(),
                    unit_price_cents: *unit_price_cents,
                    quantity: *quantity,
                });
            }
        }
        Action::RemoveItem { sku } => next.items.retain(|line| line.sku != *sku),
        Action::SetQuantity { sku, quantity } => {
            let index = next
                .items
                .iter()
                .position(|line| line.sku == *sku)
                .ok_or_else(|| CartError::UnknownSku(sku.clone()))?;
            if *quantity == 0 {
                next.items.remove(index);
            } else {
                next.items[index].quantity = *quantity;
            }
        }
        Action::ApplyCoupon(None) => next.coupon = None,
        Action::ApplyCoupon(Some(code)) => {
            if coupon_percent(code).is_none() {
                return Err(CartError::UnknownCoupon(code.clone()));
            }
            next.coupon = Some(code.clone());
        }
        Action::ClearCart => next = Cart::default(),
    }
    Ok(next)
}

/// Folds the reducer over `actions`, stopping at the first error.
pub fn run_actions(
    cart: Cart,
    actions: impl IntoIterator<Item = Action>,
) -> Result<Cart, CartError> {
    actions
        .into_iter()
        .try_fold(cart, |state, action| reduce(&state, &action))
}

/// Sum of `unit_price_cents * quantity` over all lines.
pub fn subtotal_cents(cart: &Cart) -> Result<u64, CartError> {
    cart.items.iter().try_fold(0_u64, |total, line| {
        let line_total = line
            .unit_price_cents
            .checked_mul(u64::from(line.quantity))
            .ok_or(CartError::Overflow)?;
        total.checked_add(line_total).ok_or(CartError::Overflow)
    })
}

/// Subtotal minus the coupon discount `subtotal * percent / 100` (floored).
///
/// A cart whose coupon is absent or unknown gets no discount.
pub fn total_cents(cart: &Cart) -> Result<u64, CartError> {
    let subtotal = subtotal_cents(cart)?;
    let percent = cart
        .coupon
        .as_deref()
        .and_then(coupon_percent)
        .unwrap_or(0);
    let discount = subtotal
        .checked_mul(u64::from(percent))
        .ok_or(CartError::Overflow)?
        / 100;
    Ok(subtotal - discount)
}
