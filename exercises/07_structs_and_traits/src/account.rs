//! Part 3: the `Account` trait, composition via `Ledger`, and overrides.

use crate::money::{Money, MoneyError};

/// Errors from account construction and transactions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AccountError {
    /// The owner name is empty after trimming.
    #[error("owner name is blank")]
    BlankOwner,
    /// Deposits and withdrawals must be strictly positive.
    #[error("amount must be strictly positive")]
    NonPositiveAmount,
    /// A money operation failed (for example a currency mismatch).
    #[error(transparent)]
    Money(#[from] MoneyError),
    /// The withdrawal exceeds the available funds.
    #[error("insufficient funds: requested {requested}, available {available}")]
    InsufficientFunds { requested: Money, available: Money },
    /// Overdraft limits cannot be negative.
    #[error("overdraft limit must not be negative")]
    NegativeOverdraft,
    /// The savings withdrawal limit for this period has been reached.
    #[error("withdrawal limit of {limit} reached for this period")]
    WithdrawalLimitReached { limit: u32 },
}

/// Owner and balance. Accounts compose a `Ledger` rather than inheriting one.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Ledger {
    owner: String,
    balance: Money,
}

impl Ledger {
    /// Creates a ledger. The owner is trimmed and must not be blank.
    pub fn new(_owner: &str, _opening: Money) -> Result<Ledger, AccountError> {
        todo!()
    }

    /// The trimmed owner.
    pub fn owner(&self) -> &str {
        todo!()
    }

    /// The current balance.
    pub fn balance(&self) -> Money {
        todo!()
    }
}

/// Shared behaviour of all accounts. Implementors provide the ledger; the rest has defaults.
pub trait Account {
    /// Shared read access to the ledger.
    fn ledger(&self) -> &Ledger;

    /// Mutable access to the ledger.
    fn ledger_mut(&mut self) -> &mut Ledger;

    /// The owner, from the ledger.
    fn owner(&self) -> &str {
        todo!()
    }

    /// The balance, from the ledger.
    fn balance(&self) -> Money {
        todo!()
    }

    /// Funds that may be withdrawn right now. Defaults to the balance.
    fn available_funds(&self) -> Result<Money, AccountError> {
        todo!()
    }

    /// Adds a strictly positive amount and returns the new balance.
    fn deposit(&mut self, _amount: Money) -> Result<Money, AccountError> {
        todo!()
    }

    /// Removes a strictly positive amount and returns the new balance.
    fn withdraw(&mut self, _amount: Money) -> Result<Money, AccountError> {
        todo!()
    }
}

/// A current account that may overdraw up to a limit.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct CheckingAccount {
    ledger: Ledger,
    overdraft: Money,
}

impl CheckingAccount {
    /// Opens an account with no overdraft.
    pub fn open(_owner: &str, _opening: Money) -> Result<CheckingAccount, AccountError> {
        todo!()
    }

    /// Sets the overdraft limit. Rejects negative or foreign-currency limits.
    pub fn with_overdraft(self, _limit: Money) -> Result<CheckingAccount, AccountError> {
        todo!()
    }

    /// The overdraft limit; zero in the balance's currency by default.
    pub fn overdraft_limit(&self) -> Money {
        todo!()
    }
}

impl Account for CheckingAccount {
    fn ledger(&self) -> &Ledger {
        todo!()
    }

    fn ledger_mut(&mut self) -> &mut Ledger {
        todo!()
    }

    fn available_funds(&self) -> Result<Money, AccountError> {
        todo!()
    }
}

/// A savings account with a limit on withdrawals per period.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct SavingsAccount {
    ledger: Ledger,
    max_withdrawals: u32,
    withdrawals: u32,
}

impl SavingsAccount {
    /// Opens an account with the default limit of 3 withdrawals per period.
    pub fn open(_owner: &str, _opening: Money) -> Result<SavingsAccount, AccountError> {
        todo!()
    }

    /// Sets the withdrawal limit per period.
    pub fn with_max_withdrawals(self, _max: u32) -> SavingsAccount {
        todo!()
    }

    /// The withdrawal limit per period.
    pub fn max_withdrawals(&self) -> u32 {
        todo!()
    }

    /// Successful withdrawals in the current period.
    pub fn withdrawals_this_period(&self) -> u32 {
        todo!()
    }

    /// Starts a new period: the withdrawal counter goes back to zero.
    pub fn reset_period(&mut self) {
        todo!()
    }
}

impl Account for SavingsAccount {
    fn ledger(&self) -> &Ledger {
        todo!()
    }

    fn ledger_mut(&mut self) -> &mut Ledger {
        todo!()
    }

    /// Checks the period limit first, then applies the shared withdrawal rule.
    fn withdraw(&mut self, _amount: Money) -> Result<Money, AccountError> {
        todo!()
    }
}
