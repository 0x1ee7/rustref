//! Part 1 — events, structured fields and `#[instrument]`.

use std::fmt;

use tracing::{debug, info, instrument};

/// Why a card charge failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChargeError {
    /// The amount must be strictly positive (cents). Carries the rejected value.
    InvalidAmount(i64),
    /// The issuer refused the card; `reason` is the gateway's code.
    Declined { reason: String },
    /// The gateway did not answer.
    GatewayUnavailable,
}

impl fmt::Display for ChargeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChargeError::InvalidAmount(amount) => {
                write!(f, "amount must be positive, got {amount}")
            }
            ChargeError::Declined { reason } => write!(f, "card declined: {reason}"),
            ChargeError::GatewayUnavailable => write!(f, "gateway unavailable"),
        }
    }
}

impl std::error::Error for ChargeError {}

/// Charges `amount_cents` to `card_token` for `order_id`.
///
/// The token is skipped in the span: it is a secret and must never be logged.
#[instrument(skip(card_token), fields(provider = "sim-gateway"), err)]
pub fn charge_card(
    order_id: &str,
    amount_cents: i64,
    card_token: &str,
) -> Result<String, ChargeError> {
    if amount_cents <= 0 {
        return Err(ChargeError::InvalidAmount(amount_cents));
    }
    debug!(order_id = %order_id, amount_cents, "authorising");
    if card_token.starts_with("decline") {
        return Err(ChargeError::Declined {
            reason: "do_not_honor".to_owned(),
        });
    }
    if card_token.starts_with("down") {
        return Err(ChargeError::GatewayUnavailable);
    }
    let txn = format!("txn_{order_id}_{amount_cents}");
    info!(txn_id = %txn, "charged");
    Ok(txn)
}

/// Sums item prices in cents.
#[instrument(ret)]
pub fn quote_total(items_cents: &[i64]) -> i64 {
    items_cents.iter().sum()
}
