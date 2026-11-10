//! Part 4: shared mutable state with `Mutex`, lock ordering and poisoning.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

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
        let _ = f;
        todo!()
    }
}

impl Error for BankError {}

/// An account whose balance is protected by a mutex.
///
/// Requirements:
/// - `deposit` and `withdraw` reject zero amounts with `ZeroAmount`;
/// - `deposit` returns `Overflow` rather than wrapping;
/// - `withdraw` returns `InsufficientFunds` with the current balance;
/// - both return the new balance on success;
/// - after a panic inside `update` poisons the lock, the account stays usable.
#[derive(Debug)]
pub struct Account {
    // Private: the id and the balance cell.
}

impl Account {
    /// Opens an account with `opening` units.
    pub fn new(id: impl Into<String>, opening: u64) -> Self {
        let _: String = id.into();
        let _ = opening;
        todo!()
    }

    /// The account identifier.
    pub fn id(&self) -> &str {
        todo!()
    }

    /// Current balance.
    pub fn balance(&self) -> u64 {
        todo!()
    }

    /// Adds `amount` and returns the new balance.
    pub fn deposit(&self, amount: u64) -> Result<u64, BankError> {
        let _ = amount;
        todo!()
    }

    /// Removes `amount` and returns the new balance.
    pub fn withdraw(&self, amount: u64) -> Result<u64, BankError> {
        let _ = amount;
        todo!()
    }

    /// Runs `f` on the balance while holding the lock.
    pub fn update<R>(&self, f: impl FnOnce(&mut u64) -> R) -> R {
        let _ = f;
        todo!()
    }
}

/// Moves `amount` from `src` to `dst`.
///
/// Requirements:
/// - `src` and `dst` being the same account is `SameAccount` (checked first);
/// - zero amount is `ZeroAmount`;
/// - insufficient source funds is `InsufficientFunds`; destination overflow is `Overflow`;
/// - any error leaves both balances unchanged;
/// - locks are taken in a global order so opposing transfers cannot deadlock.
pub fn transfer(src: &Account, dst: &Account, amount: u64) -> Result<(), BankError> {
    let _ = (src, dst, amount);
    todo!()
}
