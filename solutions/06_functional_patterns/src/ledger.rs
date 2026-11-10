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
pub fn apply(state: Option<&Account>, event: &Event) -> Result<Account, LedgerError> {
    match (state, event) {
        (Some(_), Event::Opened { .. }) => Err(LedgerError::AlreadyOpened),
        (None, Event::Opened { account_id, owner }) => Ok(Account {
            account_id: account_id.clone(),
            owner: owner.clone(),
            balance_cents: 0,
            status: Status::Open,
        }),
        (None, _) => Err(LedgerError::NotOpened),
        (Some(_), Event::Deposited { amount_cents: 0 } | Event::Withdrawn { amount_cents: 0 }) => {
            Err(LedgerError::NonPositiveAmount)
        }
        (Some(account), Event::Deposited { amount_cents }) => {
            ensure_open(account)?;
            let balance = account
                .balance_cents
                .checked_add(*amount_cents)
                .ok_or(LedgerError::BalanceOverflow)?;
            Ok(with_balance(account, balance))
        }
        (Some(account), Event::Withdrawn { amount_cents }) => {
            ensure_open(account)?;
            let balance = account.balance_cents.checked_sub(*amount_cents).ok_or(
                LedgerError::InsufficientFunds {
                    balance_cents: account.balance_cents,
                    requested_cents: *amount_cents,
                },
            )?;
            Ok(with_balance(account, balance))
        }
        (Some(account), Event::Frozen) => Ok(Account {
            status: Status::Frozen,
            ..account.clone()
        }),
        (Some(account), Event::Unfrozen) => Ok(Account {
            status: Status::Open,
            ..account.clone()
        }),
    }
}

fn ensure_open(account: &Account) -> Result<(), LedgerError> {
    match account.status {
        Status::Open => Ok(()),
        Status::Frozen => Err(LedgerError::AccountFrozen),
    }
}

fn with_balance(account: &Account, balance_cents: u64) -> Account {
    Account {
        balance_cents,
        ..account.clone()
    }
}

/// Command handler: returns the `Withdrawn` event if the withdrawal is allowed.
/// Uses the same rules as `apply`. `account` is not changed.
pub fn decide_withdraw(account: &Account, amount_cents: u64) -> Result<Event, LedgerError> {
    let event = Event::Withdrawn { amount_cents };
    apply(Some(account), &event)?;
    Ok(event)
}

/// Folds the whole log from `None`. Stops at the first invalid event.
pub fn replay(events: &[Event]) -> Result<Account, LedgerError> {
    let state = events.iter().try_fold(None, |state: Option<Account>, event| {
        apply(state.as_ref(), event).map(Some)
    })?;
    state.ok_or(LedgerError::EmptyLog)
}

/// State after each event. Stops at the first invalid event.
pub fn snapshots(events: &[Event]) -> Result<Vec<Account>, LedgerError> {
    events
        .iter()
        .scan(None::<Account>, |state: &mut Option<Account>, event| {
            Some(match apply(state.as_ref(), event) {
                Ok(account) => {
                    *state = Some(account.clone());
                    Ok(account)
                }
                Err(error) => Err(error),
            })
        })
        .collect()
}

/// Resumes from `start` (a previous snapshot) and applies `events`.
pub fn replay_from(start: &Account, events: &[Event]) -> Result<Account, LedgerError> {
    events
        .iter()
        .try_fold(start.clone(), |account, event| apply(Some(&account), event))
}

/// Sum of `Deposited` amounts. `None` on overflow; other events are ignored.
pub fn total_deposited(events: &[Event]) -> Option<u64> {
    events.iter().try_fold(0_u64, |total, event| match event {
        Event::Deposited { amount_cents } => total.checked_add(*amount_cents),
        _ => Some(total),
    })
}
