//! Part 1: exact money in minor units.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

/// Errors from [`Currency`] and [`Money`] parsing and arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// The code is not exactly three ASCII uppercase letters.
    #[error("invalid currency code {0:?}: expected three ASCII uppercase letters")]
    InvalidCurrency(String),
    /// The amount text does not match `-?digits(.digits)?`.
    #[error("invalid amount {0:?}")]
    InvalidAmount(String),
    /// The amount has more fraction digits than the currency's minor unit allows.
    #[error("amount {0:?} has more precision than the currency allows")]
    TooPrecise(String),
    /// Integer overflow while parsing or computing.
    #[error("money value overflows i64")]
    Overflow,
    /// Two operands are in different currencies.
    #[error("currency mismatch: {left} vs {right}")]
    CurrencyMismatch { left: Currency, right: Currency },
    /// `scaled` was called with a zero denominator.
    #[error("denominator must not be zero")]
    ZeroDenominator,
    /// `allocate` was called with zero parts.
    #[error("cannot allocate into zero parts")]
    ZeroParts,
}

/// A validated three-letter ISO-style currency code.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Currency {
    code: [u8; 3],
}

impl Currency {
    /// Number of fraction digits in the minor unit (`JPY`/`KRW`: 0, `KWD`: 3, else 2).
    pub fn minor_digits(self) -> u32 {
        todo!()
    }

    /// The three-letter code.
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl TryFrom<&str> for Currency {
    type Error = MoneyError;

    fn try_from(_code: &str) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(_code: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

/// An exact amount of money: a count of minor units in one currency.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[allow(dead_code)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    /// Creates an amount from minor units.
    pub fn new(_minor: i64, _currency: Currency) -> Money {
        todo!()
    }

    /// The zero amount in `currency`.
    pub fn zero(_currency: Currency) -> Money {
        todo!()
    }

    /// The count of minor units.
    pub fn minor(self) -> i64 {
        todo!()
    }

    /// The currency.
    pub fn currency(self) -> Currency {
        todo!()
    }

    /// Adds two amounts in the same currency.
    pub fn checked_add(self, _rhs: Money) -> Result<Money, MoneyError> {
        todo!()
    }

    /// Subtracts `rhs` from `self` in the same currency.
    pub fn checked_sub(self, _rhs: Money) -> Result<Money, MoneyError> {
        todo!()
    }

    /// Multiplies by an integer factor.
    pub fn checked_mul(self, _factor: i64) -> Result<Money, MoneyError> {
        todo!()
    }

    /// Orders two amounts in the same currency.
    pub fn checked_cmp(self, _other: Money) -> Result<Ordering, MoneyError> {
        todo!()
    }

    /// Computes `minor * numerator / denominator`, rounded half to even.
    pub fn scaled(self, _numerator: i64, _denominator: i64) -> Result<Money, MoneyError> {
        todo!()
    }

    /// Splits into `parts` shares whose minor units sum exactly to `self`.
    pub fn allocate(self, _parts: usize) -> Result<Vec<Money>, MoneyError> {
        todo!()
    }

    /// Sums `items` in `currency`, starting from zero.
    pub fn try_sum<I: IntoIterator<Item = Money>>(
        _currency: Currency,
        _items: I,
    ) -> Result<Money, MoneyError> {
        todo!()
    }
}

impl FromStr for Money {
    type Err = MoneyError;

    fn from_str(_text: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl fmt::Display for Money {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Add for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money addition failed" on currency mismatch or overflow.
    fn add(self, _rhs: Money) -> Money {
        todo!()
    }
}

impl Sub for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money subtraction failed" on currency mismatch or overflow.
    fn sub(self, _rhs: Money) -> Money {
        todo!()
    }
}

impl Mul<i64> for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money multiplication overflowed" on overflow.
    fn mul(self, _factor: i64) -> Money {
        todo!()
    }
}

impl Mul<Money> for i64 {
    type Output = Money;

    /// # Panics
    /// Panics with "Money multiplication overflowed" on overflow.
    fn mul(self, _rhs: Money) -> Money {
        todo!()
    }
}

impl Neg for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money negation overflowed" when negating `i64::MIN`.
    fn neg(self) -> Money {
        todo!()
    }
}

impl PartialOrd for Money {
    /// Returns `None` across currencies.
    fn partial_cmp(&self, _other: &Money) -> Option<Ordering> {
        todo!()
    }
}
