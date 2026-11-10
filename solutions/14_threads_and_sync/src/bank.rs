//! Accounts guarded by a `Mutex`, with deadlock-free two-account transfers.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ptr;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Why a balance change was refused. A refused operation changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankError {
    /// The account holds less than the requested amount.
    InsufficientFunds {
        /// Amount that was asked for.
        requested: u64,
        /// Balance at the time of the request.
        available: u64,
    },
    /// Source and destination are the same account.
    SameAccount,
    /// Amounts must be positive.
    ZeroAmount,
    /// The destination balance would exceed `u64::MAX`.
    Overflow,
}

impl Display for BankError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientFunds {
                requested,
                available,
            } => write!(f, "insufficient funds: requested {requested}, available {available}"),
            Self::SameAccount => write!(f, "source and destination are the same account"),
            Self::ZeroAmount => write!(f, "amount must be positive"),
            Self::Overflow => write!(f, "balance would overflow"),
        }
    }
}

impl Error for BankError {}

/// An account whose balance is protected by its own mutex.
#[derive(Debug)]
pub struct Account {
    id: String,
    balance: Mutex<u64>,
}

impl Account {
    /// Opens an account with `opening` units.
    pub fn new(id: impl Into<String>, opening: u64) -> Self {
        Self {
            id: id.into(),
            balance: Mutex::new(opening),
        }
    }

    /// The account identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Current balance.
    pub fn balance(&self) -> u64 {
        *self.cell()
    }

    /// Adds `amount` and returns the new balance.
    pub fn deposit(&self, amount: u64) -> Result<u64, BankError> {
        if amount == 0 {
            return Err(BankError::ZeroAmount);
        }
        let mut balance = self.cell();
        let new = balance.checked_add(amount).ok_or(BankError::Overflow)?;
        *balance = new;
        Ok(new)
    }

    /// Removes `amount` and returns the new balance.
    pub fn withdraw(&self, amount: u64) -> Result<u64, BankError> {
        if amount == 0 {
            return Err(BankError::ZeroAmount);
        }
        let mut balance = self.cell();
        if amount > *balance {
            return Err(BankError::InsufficientFunds {
                requested: amount,
                available: *balance,
            });
        }
        *balance -= amount;
        Ok(*balance)
    }

    /// Runs `f` on the balance while holding the lock.
    ///
    /// If `f` panics the mutex is poisoned. The balance is still readable and
    /// writable afterwards: the value is a plain `u64`, so there is no broken
    /// invariant for poisoning to protect.
    pub fn update<R>(&self, f: impl FnOnce(&mut u64) -> R) -> R {
        let mut balance = self.cell();
        f(&mut balance)
    }

    fn cell(&self) -> MutexGuard<'_, u64> {
        self.balance.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Moves `amount` from `src` to `dst` atomically with respect to other transfers.
///
/// Both locks are held for the duration. They are always taken in address
/// order, so two opposing transfers (A→B and B→A) cannot deadlock. Validation
/// happens before any balance changes, so a failed transfer changes nothing.
pub fn transfer(src: &Account, dst: &Account, amount: u64) -> Result<(), BankError> {
    if ptr::eq(src, dst) {
        return Err(BankError::SameAccount);
    }
    if amount == 0 {
        return Err(BankError::ZeroAmount);
    }

    let (mut from, mut to) = if ptr::from_ref(src).addr() < ptr::from_ref(dst).addr() {
        let from = src.cell();
        (from, dst.cell())
    } else {
        let to = dst.cell();
        (src.cell(), to)
    };

    if amount > *from {
        return Err(BankError::InsufficientFunds {
            requested: amount,
            available: *from,
        });
    }
    let new_to = to.checked_add(amount).ok_or(BankError::Overflow)?;
    *from -= amount;
    *to = new_to;
    Ok(())
}
