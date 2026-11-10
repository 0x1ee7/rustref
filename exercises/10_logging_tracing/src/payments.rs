//! Part 1 — events, structured fields and `#[instrument]`.

use std::fmt;

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

/// Charges `amount_cents` to `card_token` for `order_id` and returns the
/// transaction id `txn_<order_id>_<amount_cents>`.
///
/// Behaviour (Part 1):
/// - `amount_cents <= 0` → `Err(InvalidAmount)`, checked before anything else.
/// - a `card_token` starting with `"decline"` → `Err(Declined { reason: "do_not_honor" })`;
///   one starting with `"down"` → `Err(GatewayUnavailable)`.
/// - otherwise `Ok(txn id)`.
///
/// Observability requirements:
/// - a DEBUG event `"authorising"` with `order_id` (Display) and `amount_cents`
///   before the gateway call;
/// - an INFO event `"charged"` with `txn_id` (Display) on success;
/// - the function runs inside a span named `charge_card`, with its arguments
///   recorded as fields **except** `card_token` (a secret), plus a static
///   field `provider = "sim-gateway"`;
/// - a failure emits an ERROR event carrying the error (`err` attribute).
pub fn charge_card(
    order_id: &str,
    amount_cents: i64,
    card_token: &str,
) -> Result<String, ChargeError> {
    let _ = (order_id, amount_cents, card_token);
    todo!()
}

/// Sums item prices in cents.
///
/// Part 1: the function runs inside a span named `quote_total`, and its return
/// value is recorded on an event (`ret` attribute).
pub fn quote_total(items_cents: &[i64]) -> i64 {
    let _ = items_cents;
    todo!()
}
