//! Part 3 · event-sourced account (patterns) and Part 4 · folds over the log.

use thiserror::Error;

/// Facts recorded in the account log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Opened { account_id: String, owner: String },
    Deposited { amount_cents: u64 },
    Withdrawn { amount_cents: u64 },
    Frozen,
    Unfrozen,
}

/// Account lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    Frozen,
}

/// Projection of the log at one point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub account_id: String,
    pub owner: String,
    pub balance_cents: u64,
    pub status: Status,
}

/// Rule violations and log-level failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LedgerError {
    #[error("account already opened")]
    AlreadyOpened,
    #[error("account not opened")]
    NotOpened,
    #[error("amount must be positive")]
    NonPositiveAmount,
    #[error("account is frozen")]
    AccountFrozen,
    #[error("insufficient funds: balance {balance_cents}, requested {requested_cents}")]
    InsufficientFunds {
        balance_cents: u64,
        requested_cents: u64,
    },
    #[error("balance overflow")]
    BalanceOverflow,
    #[error("event log is empty")]
    EmptyLog,
}

/// Fold step. `state` is `None` before the account is opened.
///
/// See `README.md` Part 3 for the exact check order.
pub fn apply(_state: Option<&Account>, _event: &Event) -> Result<Account, LedgerError> {
    todo!()
}

/// Command handler: returns the `Withdrawn` event if the withdrawal is allowed.
/// Uses the same rules as `apply`. `account` is not changed.
pub fn decide_withdraw(_account: &Account, _amount_cents: u64) -> Result<Event, LedgerError> {
    todo!()
}

/// Folds the whole log from `None`. Stops at the first invalid event.
pub fn replay(_events: &[Event]) -> Result<Account, LedgerError> {
    todo!()
}

/// State after each event. Stops at the first invalid event.
pub fn snapshots(_events: &[Event]) -> Result<Vec<Account>, LedgerError> {
    todo!()
}

/// Resumes from `start` (a previous snapshot) and applies `events`.
pub fn replay_from(_start: &Account, _events: &[Event]) -> Result<Account, LedgerError> {
    todo!()
}

/// Sum of `Deposited` amounts. `None` on overflow; other events are ignored.
pub fn total_deposited(_events: &[Event]) -> Option<u64> {
    todo!()
}
