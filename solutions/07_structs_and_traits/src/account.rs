//! Part 3: the `Account` trait, composition via `Ledger`, and overrides.

use std::cmp::Ordering;

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
pub struct Ledger {
    owner: String,
    balance: Money,
}

impl Ledger {
    /// Creates a ledger. The owner is trimmed and must not be blank.
    pub fn new(owner: &str, opening: Money) -> Result<Ledger, AccountError> {
        let owner = owner.trim();
        if owner.is_empty() {
            return Err(AccountError::BlankOwner);
        }
        Ok(Ledger {
            owner: owner.to_owned(),
            balance: opening,
        })
    }

    /// The trimmed owner.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The current balance.
    pub fn balance(&self) -> Money {
        self.balance
    }
}

/// Rejects zero and negative amounts.
fn ensure_positive(amount: Money) -> Result<(), AccountError> {
    if amount.minor() > 0 {
        Ok(())
    } else {
        Err(AccountError::NonPositiveAmount)
    }
}

/// The one withdrawal rule, shared by the default method and overrides.
fn apply_withdrawal<A: Account + ?Sized>(
    account: &mut A,
    amount: Money,
) -> Result<Money, AccountError> {
    ensure_positive(amount)?;
    let available = account.available_funds()?;
    if amount.checked_cmp(available)? == Ordering::Greater {
        return Err(AccountError::InsufficientFunds {
            requested: amount,
            available,
        });
    }
    let ledger = account.ledger_mut();
    ledger.balance = ledger.balance.checked_sub(amount)?;
    Ok(ledger.balance)
}

/// Shared behaviour of all accounts. Implementors provide the ledger; the rest has defaults.
pub trait Account {
    /// Shared read access to the ledger.
    fn ledger(&self) -> &Ledger;

    /// Mutable access to the ledger.
    fn ledger_mut(&mut self) -> &mut Ledger;

    /// The owner, from the ledger.
    fn owner(&self) -> &str {
        self.ledger().owner()
    }

    /// The balance, from the ledger.
    fn balance(&self) -> Money {
        self.ledger().balance()
    }

    /// Funds that may be withdrawn right now. Defaults to the balance.
    fn available_funds(&self) -> Result<Money, AccountError> {
        Ok(self.balance())
    }

    /// Adds a strictly positive amount and returns the new balance.
    fn deposit(&mut self, amount: Money) -> Result<Money, AccountError> {
        ensure_positive(amount)?;
        let ledger = self.ledger_mut();
        ledger.balance = ledger.balance.checked_add(amount)?;
        Ok(ledger.balance)
    }

    /// Removes a strictly positive amount and returns the new balance.
    fn withdraw(&mut self, amount: Money) -> Result<Money, AccountError> {
        apply_withdrawal(self, amount)
    }
}

/// A current account that may overdraw up to a limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckingAccount {
    ledger: Ledger,
    overdraft: Money,
}

impl CheckingAccount {
    /// Opens an account with no overdraft.
    pub fn open(owner: &str, opening: Money) -> Result<CheckingAccount, AccountError> {
        Ok(CheckingAccount {
            ledger: Ledger::new(owner, opening)?,
            overdraft: Money::zero(opening.currency()),
        })
    }

    /// Sets the overdraft limit. Rejects negative or foreign-currency limits.
    pub fn with_overdraft(mut self, limit: Money) -> Result<CheckingAccount, AccountError> {
        let currency = self.ledger.balance.currency();
        if limit.currency() != currency {
            return Err(MoneyError::CurrencyMismatch {
                left: currency,
                right: limit.currency(),
            }
            .into());
        }
        if limit.minor() < 0 {
            return Err(AccountError::NegativeOverdraft);
        }
        self.overdraft = limit;
        Ok(self)
    }

    /// The overdraft limit; zero in the balance's currency by default.
    pub fn overdraft_limit(&self) -> Money {
        self.overdraft
    }
}

impl Account for CheckingAccount {
    fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.ledger
    }

    fn available_funds(&self) -> Result<Money, AccountError> {
        Ok(self.ledger.balance.checked_add(self.overdraft)?)
    }
}

/// A savings account with a limit on withdrawals per period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavingsAccount {
    ledger: Ledger,
    max_withdrawals: u32,
    withdrawals: u32,
}

impl SavingsAccount {
    /// Opens an account with the default limit of 3 withdrawals per period.
    pub fn open(owner: &str, opening: Money) -> Result<SavingsAccount, AccountError> {
        Ok(SavingsAccount {
            ledger: Ledger::new(owner, opening)?,
            max_withdrawals: 3,
            withdrawals: 0,
        })
    }

    /// Sets the withdrawal limit per period.
    pub fn with_max_withdrawals(mut self, max: u32) -> SavingsAccount {
        self.max_withdrawals = max;
        self
    }

    /// The withdrawal limit per period.
    pub fn max_withdrawals(&self) -> u32 {
        self.max_withdrawals
    }

    /// Successful withdrawals in the current period.
    pub fn withdrawals_this_period(&self) -> u32 {
        self.withdrawals
    }

    /// Starts a new period: the withdrawal counter goes back to zero.
    pub fn reset_period(&mut self) {
        self.withdrawals = 0;
    }
}

impl Account for SavingsAccount {
    fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.ledger
    }

    /// Checks the period limit first, then applies the shared withdrawal rule.
    fn withdraw(&mut self, amount: Money) -> Result<Money, AccountError> {
        if self.withdrawals >= self.max_withdrawals {
            return Err(AccountError::WithdrawalLimitReached {
                limit: self.max_withdrawals,
            });
        }
        let balance = apply_withdrawal(self, amount)?;
        // Only successful withdrawals count; the limit check guarantees no overflow here.
        self.withdrawals += 1;
        Ok(balance)
    }
}
