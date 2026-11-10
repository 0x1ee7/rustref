//! A ledger whose balance can only change through checked operations.

/// Errors from [`Ledger`] operations. Failed operations leave the balance
/// unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LedgerError {
    /// The opening balance was negative.
    #[error("opening balance {0} must not be negative")]
    NegativeOpening(i64),
    /// Deposit or withdrawal amount was zero or negative.
    #[error("amount {0} must be positive")]
    NonPositiveAmount(i64),
    /// Withdrawal exceeds the available balance.
    #[error("insufficient funds: balance {balance}, requested {requested}")]
    InsufficientFunds {
        /// Balance at the time of the request.
        balance: i64,
        /// Amount requested.
        requested: i64,
    },
    /// The arithmetic would overflow `i64`.
    #[error("balance arithmetic overflowed")]
    Overflow,
}

/// A ledger balance. The field is private so that only checked methods
/// can change it.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    balance: i64,
}

impl Ledger {
    /// Creates a ledger with `opening` as its starting balance.
    ///
    /// # Errors
    ///
    /// [`LedgerError::NegativeOpening`] when `opening < 0`.
    pub fn new(opening: i64) -> Result<Self, LedgerError> {
        let _ = opening;
        todo!()
    }

    /// Current balance.
    #[must_use]
    pub fn balance(&self) -> i64 {
        todo!()
    }

    /// Adds `amount` and returns the new balance.
    ///
    /// # Errors
    ///
    /// [`LedgerError::NonPositiveAmount`] or [`LedgerError::Overflow`].
    pub fn deposit(&mut self, amount: i64) -> Result<i64, LedgerError> {
        let _ = amount;
        todo!()
    }

    /// Removes `amount` and returns the new balance.
    ///
    /// # Errors
    ///
    /// [`LedgerError::NonPositiveAmount`] or [`LedgerError::InsufficientFunds`].
    pub fn withdraw(&mut self, amount: i64) -> Result<i64, LedgerError> {
        let _ = amount;
        todo!()
    }
}
