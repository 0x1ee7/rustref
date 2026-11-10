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
        match &self.code {
            b"JPY" | b"KRW" => 0,
            b"KWD" => 3,
            _ => 2,
        }
    }

    /// The three-letter code.
    pub fn as_str(&self) -> &str {
        // Validated ASCII on construction, so this cannot fail.
        std::str::from_utf8(&self.code).expect("currency codes are ASCII")
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Currency({:?})", self.as_str())
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for Currency {
    type Error = MoneyError;

    fn try_from(code: &str) -> Result<Self, Self::Error> {
        match <[u8; 3]>::try_from(code.as_bytes()) {
            Ok(bytes) if bytes.iter().all(u8::is_ascii_uppercase) => Ok(Currency { code: bytes }),
            _ => Err(MoneyError::InvalidCurrency(code.to_owned())),
        }
    }
}

impl FromStr for Currency {
    type Err = MoneyError;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        Currency::try_from(code)
    }
}

/// An exact amount of money: a count of minor units in one currency.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    /// Creates an amount from minor units.
    pub fn new(minor: i64, currency: Currency) -> Money {
        Money { minor, currency }
    }

    /// The zero amount in `currency`.
    pub fn zero(currency: Currency) -> Money {
        Money::new(0, currency)
    }

    /// The count of minor units.
    pub fn minor(self) -> i64 {
        self.minor
    }

    /// The currency.
    pub fn currency(self) -> Currency {
        self.currency
    }

    /// Fails unless both operands share a currency.
    fn same_currency(self, rhs: Money) -> Result<(), MoneyError> {
        if self.currency == rhs.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch {
                left: self.currency,
                right: rhs.currency,
            })
        }
    }

    /// Adds two amounts in the same currency.
    pub fn checked_add(self, rhs: Money) -> Result<Money, MoneyError> {
        self.same_currency(rhs)?;
        let minor = self.minor.checked_add(rhs.minor).ok_or(MoneyError::Overflow)?;
        Ok(Money::new(minor, self.currency))
    }

    /// Subtracts `rhs` from `self` in the same currency.
    pub fn checked_sub(self, rhs: Money) -> Result<Money, MoneyError> {
        self.same_currency(rhs)?;
        let minor = self.minor.checked_sub(rhs.minor).ok_or(MoneyError::Overflow)?;
        Ok(Money::new(minor, self.currency))
    }

    /// Multiplies by an integer factor.
    pub fn checked_mul(self, factor: i64) -> Result<Money, MoneyError> {
        let minor = self.minor.checked_mul(factor).ok_or(MoneyError::Overflow)?;
        Ok(Money::new(minor, self.currency))
    }

    /// Orders two amounts in the same currency.
    pub fn checked_cmp(self, other: Money) -> Result<Ordering, MoneyError> {
        self.same_currency(other)?;
        Ok(self.minor.cmp(&other.minor))
    }

    /// Computes `minor * numerator / denominator`, rounded half to even.
    pub fn scaled(self, numerator: i64, denominator: i64) -> Result<Money, MoneyError> {
        if denominator == 0 {
            return Err(MoneyError::ZeroDenominator);
        }
        // i64 * i64 always fits in i128, so the intermediate product is exact.
        let product = i128::from(self.minor) * i128::from(numerator);
        let divisor = i128::from(denominator);
        let sign = if (product < 0) != (divisor < 0) { -1 } else { 1 };
        let (dividend_abs, divisor_abs) = (product.abs(), divisor.abs());
        let mut quotient = dividend_abs / divisor_abs;
        let remainder = dividend_abs % divisor_abs;
        let twice_remainder = remainder * 2;
        if twice_remainder > divisor_abs || (twice_remainder == divisor_abs && quotient % 2 == 1) {
            quotient += 1;
        }
        let minor = i64::try_from(sign * quotient).map_err(|_| MoneyError::Overflow)?;
        Ok(Money::new(minor, self.currency))
    }

    /// Splits into `parts` shares whose minor units sum exactly to `self`.
    pub fn allocate(self, parts: usize) -> Result<Vec<Money>, MoneyError> {
        if parts == 0 {
            return Err(MoneyError::ZeroParts);
        }
        let count = i128::try_from(parts).map_err(|_| MoneyError::Overflow)?;
        let magnitude = i128::from(self.minor).abs();
        let base = magnitude / count;
        let remainder = magnitude % count;
        let sign: i128 = if self.minor < 0 { -1 } else { 1 };
        (0..count)
            .map(|index| -> Result<Money, MoneyError> {
                // The first `remainder` shares absorb the leftover units.
                let share = base + i128::from(index < remainder);
                let minor = i64::try_from(sign * share).map_err(|_| MoneyError::Overflow)?;
                Ok(Money::new(minor, self.currency))
            })
            .collect()
    }

    /// Sums `items` in `currency`, starting from zero.
    pub fn try_sum<I: IntoIterator<Item = Money>>(
        currency: Currency,
        items: I,
    ) -> Result<Money, MoneyError> {
        items
            .into_iter()
            .try_fold(Money::zero(currency), Money::checked_add)
    }
}

/// Parses `-?digits(.digits)?` into minor units for a currency with `minor_digits`.
fn parse_minor_units(text: &str, minor_digits: u32) -> Result<i64, MoneyError> {
    let invalid = || MoneyError::InvalidAmount(text.to_owned());
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) if is_digits(fraction) => (whole, fraction),
        Some(_) => return Err(invalid()),
        None => (unsigned, ""),
    };
    if !is_digits(whole) {
        return Err(invalid());
    }
    let digits = minor_digits as usize;
    let (kept, extra) = fraction.split_at(digits.min(fraction.len()));
    // Trailing zeros beyond the minor unit are harmless; anything else is lost precision.
    if extra.bytes().any(|byte| byte != b'0') {
        return Err(MoneyError::TooPrecise(text.to_owned()));
    }
    let whole_units: i128 = whole.parse().map_err(|_| MoneyError::Overflow)?;
    let mut fraction_units: i128 = 0;
    for byte in kept.bytes() {
        fraction_units = fraction_units * 10 + i128::from(byte - b'0');
    }
    for _ in kept.len()..digits {
        fraction_units *= 10;
    }
    let magnitude = whole_units
        .checked_mul(10_i128.pow(minor_digits))
        .and_then(|scaled| scaled.checked_add(fraction_units))
        .ok_or(MoneyError::Overflow)?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).map_err(|_| MoneyError::Overflow)
}

/// `true` for a non-empty run of ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

impl FromStr for Money {
    type Err = MoneyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split_whitespace();
        let (Some(amount), Some(code), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(MoneyError::InvalidAmount(text.to_owned()));
        };
        let currency = Currency::try_from(code)?;
        let minor = parse_minor_units(amount, currency.minor_digits())?;
        Ok(Money::new(minor, currency))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.minor < 0 { "-" } else { "" };
        let magnitude = self.minor.unsigned_abs();
        let digits = self.currency.minor_digits();
        if digits == 0 {
            write!(f, "{sign}{magnitude} {}", self.currency)
        } else {
            let scale = 10_u64.pow(digits);
            let width = digits as usize;
            write!(
                f,
                "{sign}{}.{:0width$} {}",
                magnitude / scale,
                magnitude % scale,
                self.currency
            )
        }
    }
}

impl Add for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money addition failed" on currency mismatch or overflow.
    fn add(self, rhs: Money) -> Money {
        self.checked_add(rhs).expect("Money addition failed")
    }
}

impl Sub for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money subtraction failed" on currency mismatch or overflow.
    fn sub(self, rhs: Money) -> Money {
        self.checked_sub(rhs).expect("Money subtraction failed")
    }
}

impl Mul<i64> for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money multiplication overflowed" on overflow.
    fn mul(self, factor: i64) -> Money {
        self.checked_mul(factor)
            .expect("Money multiplication overflowed")
    }
}

impl Mul<Money> for i64 {
    type Output = Money;

    /// # Panics
    /// Panics with "Money multiplication overflowed" on overflow.
    fn mul(self, rhs: Money) -> Money {
        rhs * self
    }
}

impl Neg for Money {
    type Output = Money;

    /// # Panics
    /// Panics with "Money negation overflowed" when negating `i64::MIN`.
    fn neg(self) -> Money {
        let minor = self.minor.checked_neg().expect("Money negation overflowed");
        Money::new(minor, self.currency)
    }
}

impl PartialOrd for Money {
    /// Returns `None` across currencies.
    fn partial_cmp(&self, other: &Money) -> Option<Ordering> {
        self.checked_cmp(*other).ok()
    }
}
