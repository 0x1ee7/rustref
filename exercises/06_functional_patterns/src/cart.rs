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
pub fn coupon_percent(_code: &str) -> Option<u8> {
    todo!()
}

/// Pure reducer: returns the next cart. Neither `cart` nor `action` is modified.
///
/// Matches on `Action` exhaustively (no wildcard arm). See `README.md` Part 1.
pub fn reduce(_cart: &Cart, _action: &Action) -> Result<Cart, CartError> {
    todo!()
}

/// Folds the reducer over `actions`, stopping at the first error.
pub fn run_actions(
    _cart: Cart,
    _actions: impl IntoIterator<Item = Action>,
) -> Result<Cart, CartError> {
    todo!()
}

/// Sum of `unit_price_cents * quantity` over all lines.
pub fn subtotal_cents(_cart: &Cart) -> Result<u64, CartError> {
    todo!()
}

/// Subtotal minus the coupon discount `subtotal * percent / 100` (floored).
///
/// A cart whose coupon is absent or unknown gets no discount.
pub fn total_cents(_cart: &Cart) -> Result<u64, CartError> {
    todo!()
}
