//! Reference test suite for `billing/billing.rs`.
//!
//! Black-box: only the public API of `ex16_testing::billing` is used. Fakes
//! implement the ports with `RefCell` state; `&T` implements each port so the
//! test keeps ownership of the fakes and can inspect them afterwards.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ex16_testing::billing::*;
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use tempfile::TempDir;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

// ---------------------------------------------------------------- helpers

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(7),
        ..ProptestConfig::default()
    }
}

fn date(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd(year, month, day).expect("valid calendar date")
}

/// Days since 1970-01-01 by the civil-calendar formula, written out here so the
/// fixtures do not depend on the code under test.
fn unix_days(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(if month <= 2 { year - 1 } else { year });
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The instant `hour:00 UTC` on the given date.
fn at(year: i32, month: u32, day: u32, hour: i64) -> Timestamp {
    Timestamp::from_unix_seconds(unix_days(year, month, day) * 86_400 + hour * 3_600)
}

fn line(sku: &str, unit_price_mills: i64, quantity: i64) -> LineItem {
    LineItem::new(sku, unit_price_mills, quantity).expect("valid line item")
}

fn invoice(lines: Vec<LineItem>) -> Invoice {
    Invoice::new("INV-1", "ada@example.com", lines, date(2026, 3, 1)).expect("has lines")
}

/// Subtotal 1000.00, 10% discount, 8% tax: taxable 900.00, tax 72.00, total 972.00.
fn sample_invoice() -> Invoice {
    invoice(vec![line("BOOK", 1_000_000, 1)])
        .with_tax_rate_bp(800)
        .expect("valid rate")
}

fn policy(max_attempts: u32) -> RetryPolicy {
    RetryPolicy::new(max_attempts, Duration::from_millis(100), Duration::from_secs(10))
        .expect("valid policy")
}

fn ok(txn: &str) -> Result<String, GatewayError> {
    Ok(txn.to_owned())
}

fn transient(msg: &str) -> Result<String, GatewayError> {
    Err(GatewayError::Transient(msg.to_owned()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Call {
    amount: Money,
    currency: String,
    token: String,
    key: String,
}

struct ScriptedGateway {
    script: RefCell<VecDeque<Result<String, GatewayError>>>,
    calls: RefCell<Vec<Call>>,
}

impl ScriptedGateway {
    fn respond(&self, request: &ChargeRequest<'_>) -> Result<String, GatewayError> {
        self.calls.borrow_mut().push(Call {
            amount: request.amount,
            currency: request.currency.to_owned(),
            token: request.token.to_owned(),
            key: request.idempotency_key.to_owned(),
        });
        self.script
            .borrow_mut()
            .pop_front()
            .expect("gateway called more times than scripted")
    }
}

impl Gateway for &ScriptedGateway {
    fn charge(&self, request: &ChargeRequest<'_>) -> Result<String, GatewayError> {
        self.respond(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Email {
    to: String,
    subject: String,
    body: String,
}

struct RecordingNotifier {
    sent: RefCell<Vec<Email>>,
    fail: bool,
}

impl Notifier for &RecordingNotifier {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), NotifyError> {
        self.sent.borrow_mut().push(Email {
            to: to.to_owned(),
            subject: subject.to_owned(),
            body: body.to_owned(),
        });
        if self.fail {
            Err(NotifyError("smtp unreachable".to_owned()))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct RecordingSleeper {
    sleeps: RefCell<Vec<Duration>>,
}

impl Sleeper for &RecordingSleeper {
    fn sleep(&self, duration: Duration) {
        self.sleeps.borrow_mut().push(duration);
    }
}

#[derive(Debug, Clone, Copy)]
struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

/// Everything a charge touched, so tests can assert on it afterwards.
struct Fakes {
    gateway: ScriptedGateway,
    notifier: RecordingNotifier,
    sleeper: RecordingSleeper,
}

impl Fakes {
    fn new(script: Vec<Result<String, GatewayError>>, notify_fails: bool) -> Self {
        Self {
            gateway: ScriptedGateway {
                script: RefCell::new(script.into()),
                calls: RefCell::new(Vec::new()),
            },
            notifier: RecordingNotifier {
                sent: RefCell::new(Vec::new()),
                fail: notify_fails,
            },
            sleeper: RecordingSleeper::default(),
        }
    }

    fn calls(&self) -> Vec<Call> {
        self.gateway.calls.borrow().clone()
    }

    fn sleeps(&self) -> Vec<Duration> {
        self.sleeper.sleeps.borrow().clone()
    }

    fn emails(&self) -> Vec<Email> {
        self.notifier.sent.borrow().clone()
    }
}

fn charge_at(
    now: Timestamp,
    invoice: &Invoice,
    script: Vec<Result<String, GatewayError>>,
    policy: RetryPolicy,
    notify_fails: bool,
) -> (Result<String, BillingError>, Fakes) {
    let fakes = Fakes::new(script, notify_fails);
    let service = PaymentService::new(
        &fakes.gateway,
        &fakes.notifier,
        FixedClock(now),
        &fakes.sleeper,
        policy,
    );
    let result = service.charge(invoice, "tok_visa");
    (result, fakes)
}

fn charge(
    invoice: &Invoice,
    script: Vec<Result<String, GatewayError>>,
    policy: RetryPolicy,
) -> (Result<String, BillingError>, Fakes) {
    charge_at(at(2026, 3, 1, 12), invoice, script, policy, false)
}

/// Captures tracing output as lines, e.g. `ERROR payment declined ...`.
#[derive(Clone, Default)]
struct LogSink(Arc<Mutex<Vec<u8>>>);

impl Write for LogSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogSink {
    type Writer = LogSink;

    fn make_writer(&'a self) -> LogSink {
        self.clone()
    }
}

fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    let sink = LogSink::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_max_level(Level::INFO)
        .finish();
    let out = tracing::subscriber::with_default(subscriber, f);
    let bytes = sink.0.lock().expect("log lock").clone();
    let text = String::from_utf8(bytes).expect("utf-8 logs");
    (out, text.lines().map(str::to_owned).collect())
}

/// The level word at the start of a captured line.
fn level(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

fn has_log(lines: &[String], lvl: &str, needle: &str) -> bool {
    lines
        .iter()
        .any(|l| level(l) == lvl && l.contains(needle))
}

fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read dir")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------- Part 1: line items, money, totals

#[test]
fn part1_line_total_multiplies_unit_price_by_quantity() {
    // 12.50 each, quantity 3 -> 37.50
    let total = line("A", 12_500, 3).line_total();
    assert_eq!(total, Money::from_cents(3_750), "12.50 x 3 must be 37.50");
}

#[test]
fn part1_line_total_rounds_half_cent_up() {
    // 1002.5 cents rounds to 1003 (ties away from zero).
    let total = line("A", 10_025, 1).line_total();
    assert_eq!(total, Money::from_cents(1_003), "half a cent must round up");
}

#[test]
fn part1_line_total_keeps_sub_cent_precision_until_the_end() {
    // Unit price 0.015 (15 mills), quantity 200 -> 3.00 exactly.
    let total = line("A", 15, 200).line_total();
    assert_eq!(total, Money::from_cents(300), "per-unit rounding would drift");
}

#[test]
fn part1_zero_priced_item_is_allowed_and_free() {
    let free = line("GIFT", 0, 5);
    assert_eq!(free.line_total(), Money::ZERO);
}

#[test]
fn part1_line_item_rejects_empty_sku() {
    let err = LineItem::new("", 100, 1).expect_err("empty sku must be rejected");
    assert!(matches!(err, BillingError::InvalidInvoice(_)), "got {err:?}");
}

#[test]
fn part1_line_item_rejects_negative_unit_price() {
    assert!(LineItem::new("A", -1, 1).is_err(), "negative price must be rejected");
}

#[test]
fn part1_line_item_rejects_zero_quantity() {
    let err = LineItem::new("A", 100, 0).expect_err("quantity 0 must be rejected");
    assert!(matches!(err, BillingError::InvalidInvoice(_)), "got {err:?}");
}

#[test]
fn part1_invoice_requires_at_least_one_line() {
    let err = Invoice::new("INV-9", "x@example.com", vec![], date(2026, 1, 1))
        .expect_err("no lines must be rejected");
    assert!(matches!(err, BillingError::InvalidInvoice(_)), "got {err:?}");
}

#[test]
fn part1_subtotal_is_sum_of_line_totals_without_discount_below_tier() {
    let inv = invoice(vec![line("A", 10_000, 2), line("B", 25_500, 1)]);
    let totals = invoice_totals(&inv);
    // 20.00 + 25.50 = 45.50, below the 100.00 discount tier.
    assert_eq!(totals.subtotal, Money::from_cents(4_550));
    assert_eq!(totals.discount, Money::ZERO);
    assert_eq!(totals.tax, Money::ZERO);
    assert_eq!(totals.total, Money::from_cents(4_550));
}

#[test]
fn part1_tax_rate_must_be_below_one_hundred_percent() {
    let inv = invoice(vec![line("A", 100, 1)]);
    assert!(inv.clone().with_tax_rate_bp(10_000).is_err(), "100% must be rejected");
    assert!(inv.with_tax_rate_bp(9_999).is_ok(), "99.99% is allowed");
}

#[test]
fn part1_money_displays_with_two_decimals() {
    assert_eq!(Money::from_cents(1_205).to_string(), "12.05");
    assert_eq!(Money::from_cents(-5).to_string(), "-0.05");
    assert_eq!(Money::from_cents(0).to_string(), "0.00");
    assert_eq!(Money::from_cents(100_000).to_string(), "1000.00");
}

#[test]
fn part1_money_sums_and_subtracts() {
    let total: Money = [100, 250, -50].into_iter().map(Money::from_cents).sum();
    assert_eq!(total, Money::from_cents(300));
    assert_eq!(Money::from_cents(300) - Money::from_cents(500), Money::from_cents(-200));
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn part1_total_identity_holds_for_any_invoice(
        lines in prop::collection::vec((0i64..=1_000_000, 1i64..=20), 1..6),
        rate in 0u32..10_000,
    ) {
        let lines: Vec<LineItem> = lines
            .into_iter()
            .enumerate()
            .map(|(i, (mills, qty))| line(&format!("SKU{i}"), mills, qty))
            .collect();
        let inv = Invoice::new("INV-P", "p@example.com", lines, date(2026, 1, 1))
            .expect("has lines")
            .with_tax_rate_bp(rate)
            .expect("rate below 100%");
        let t = invoice_totals(&inv);
        prop_assert_eq!(t.total, t.subtotal - t.discount + t.tax);
        prop_assert!(t.discount <= t.subtotal);
        prop_assert!(t.tax >= Money::ZERO);
        prop_assert!(t.total >= Money::ZERO);
    }
}

// ---------------------------------------------------------------- Part 2: discounts and tax

#[test]
fn part2_discount_tier_table() {
    let cases = [
        (0, 0),
        (9_999, 0),
        (10_000, 200),
        (49_999, 200),
        (50_000, 500),
        (99_999, 500),
        (100_000, 1_000),
        (5_000_000, 1_000),
    ];
    for (cents, expected) in cases {
        assert_eq!(
            discount_rate_bp(Money::from_cents(cents)),
            expected,
            "discount for {cents} cents"
        );
    }
}

#[test]
fn part2_tier_threshold_is_inclusive_at_500() {
    let inv = invoice(vec![line("A", 500_000, 1)]);
    let totals = invoice_totals(&inv);
    assert_eq!(totals.subtotal, Money::from_cents(50_000));
    assert_eq!(totals.discount, Money::from_cents(2_500), "exactly 500.00 earns 5%");
}

#[test]
fn part2_just_below_threshold_gets_the_lower_tier() {
    let inv = invoice(vec![line("A", 499_990, 1)]);
    let totals = invoice_totals(&inv);
    assert_eq!(totals.subtotal, Money::from_cents(49_999));
    assert_eq!(totals.discount, Money::from_cents(1_000), "499.99 earns 2%, not 5%");
}

#[test]
fn part2_discount_half_cent_rounds_up() {
    // Subtotal 100.25, 2% discount = 2.005 -> 2.01.
    let totals = invoice_totals(&invoice(vec![line("A", 100_250, 1)]));
    assert_eq!(totals.discount, Money::from_cents(201));
}

#[test]
fn part2_tax_is_charged_on_the_discounted_amount() {
    let totals = invoice_totals(&sample_invoice());
    assert_eq!(totals.subtotal, Money::from_cents(100_000));
    assert_eq!(totals.discount, Money::from_cents(10_000), "10% of 1000.00");
    assert_eq!(totals.tax, Money::from_cents(7_200), "8% of the 900.00 taxable amount");
    assert_eq!(totals.total, Money::from_cents(97_200));
}

#[test]
fn part2_second_discount_and_tax_example() {
    // 500.00 -> 5% discount 25.00 -> taxable 475.00 -> 10% tax 47.50 -> total 522.50.
    let inv = invoice(vec![line("A", 500_000, 1)])
        .with_tax_rate_bp(1_000)
        .expect("valid rate");
    let totals = invoice_totals(&inv);
    assert_eq!(totals.discount, Money::from_cents(2_500));
    assert_eq!(totals.tax, Money::from_cents(4_750));
    assert_eq!(totals.total, Money::from_cents(52_250));
}

#[test]
fn part2_without_tax_total_equals_taxable_amount() {
    let totals = invoice_totals(&invoice(vec![line("A", 2_000_000, 1)]));
    assert_eq!(totals.tax, Money::ZERO);
    assert_eq!(totals.total, totals.subtotal - totals.discount);
}

#[test]
fn part2_invoices_default_to_usd_and_can_change_currency() {
    let inv = invoice(vec![line("A", 100, 1)]);
    assert_eq!(inv.currency(), "USD");
    assert_eq!(inv.clone().with_currency("EUR").currency(), "EUR");
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn part2_discount_rate_never_decreases_as_subtotal_grows(
        a in 0i64..20_000_000,
        b in 0i64..20_000_000,
    ) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        prop_assert!(
            discount_rate_bp(Money::from_cents(lo)) <= discount_rate_bp(Money::from_cents(hi))
        );
    }
}

// ---------------------------------------------------------------- Part 3: dates, timestamps, due dates

#[test]
fn part3_from_ymd_rejects_impossible_dates() {
    assert!(Date::from_ymd(2026, 2, 29).is_none(), "2026 is not a leap year");
    assert!(Date::from_ymd(2026, 13, 1).is_none(), "no month 13");
    assert!(Date::from_ymd(2026, 4, 31).is_none(), "April has 30 days");
    assert!(Date::from_ymd(2026, 1, 0).is_none(), "no day 0");
    assert!(Date::from_ymd(2026, 0, 1).is_none(), "no month 0");
}

#[test]
fn part3_leap_years_follow_the_gregorian_rule() {
    assert!(Date::from_ymd(2024, 2, 29).is_some(), "2024 is a leap year");
    assert!(Date::from_ymd(2000, 2, 29).is_some(), "2000 is divisible by 400");
    assert!(Date::from_ymd(1900, 2, 29).is_none(), "1900 is not (divisible by 100)");
    assert!(Date::from_ymd(2100, 2, 29).is_none(), "2100 is not a leap year");
}

#[test]
fn part3_date_displays_as_iso_8601() {
    assert_eq!(date(2026, 3, 1).to_string(), "2026-03-01");
    assert_eq!(date(999, 12, 31).to_string(), "0999-12-31");
}

#[test]
fn part3_add_days_crosses_month_year_and_leap_boundaries() {
    assert_eq!(date(2025, 12, 31).add_days(1).ymd(), (2026, 1, 1));
    assert_eq!(date(2024, 2, 28).add_days(1).ymd(), (2024, 2, 29));
    assert_eq!(date(2025, 2, 28).add_days(1).ymd(), (2025, 3, 1));
    assert_eq!(date(2026, 1, 1).add_days(-1).ymd(), (2025, 12, 31));
}

#[test]
fn part3_dates_order_chronologically() {
    assert!(date(2026, 3, 1) < date(2026, 3, 2));
    assert!(date(2025, 12, 31) < date(2026, 1, 1));
}

#[test]
fn part3_timestamp_maps_to_its_utc_day() {
    // 2026-03-01T00:00:00Z
    let midnight = Timestamp::from_unix_seconds(1_772_323_200);
    assert_eq!(midnight.utc_date(), date(2026, 3, 1));
    assert_eq!(Timestamp::from_unix_seconds(1_772_323_199).utc_date(), date(2026, 2, 28));
}

#[test]
fn part3_negative_timestamps_map_to_the_previous_day() {
    assert_eq!(Timestamp::from_unix_seconds(-1).utc_date(), date(1969, 12, 31));
    assert_eq!(Timestamp::from_unix_seconds(0).utc_date(), date(1970, 1, 1));
}

#[test]
fn part3_due_date_adds_net_days() {
    assert_eq!(due_date(date(2026, 3, 1), 30).expect("valid"), date(2026, 3, 31));
    assert_eq!(due_date(date(2026, 3, 1), 0).expect("valid"), date(2026, 3, 1));
}

#[test]
fn part3_due_date_rejects_negative_terms() {
    let err = due_date(date(2026, 3, 1), -1).expect_err("negative terms must be rejected");
    assert!(matches!(err, BillingError::InvalidTerms(_)), "got {err:?}");
}

#[test]
fn part3_issue_due_date_counts_from_the_clock_utc_day() {
    // 02:00Z on 2026-03-31 is still 31 March in UTC.
    let clock = FixedClock(at(2026, 3, 31, 2));
    assert_eq!(issue_due_date(&clock, 30).expect("valid"), date(2026, 4, 30));
}

#[test]
fn part3_overdue_only_after_the_due_day_has_ended() {
    let due = date(2026, 3, 31);
    let late_on_due_day = FixedClock(at(2026, 3, 31, 23));
    assert!(!is_overdue(&late_on_due_day, due), "still due during the due day");
    let next_midnight = FixedClock(at(2026, 4, 1, 0));
    assert!(is_overdue(&next_midnight, due), "overdue once the day has ended");
}

#[test]
fn part3_overdue_uses_the_utc_day_not_a_local_offset() {
    // 02:00Z on 31 March is already overdue for a 30 March due date.
    let clock = FixedClock(at(2026, 3, 31, 2));
    assert!(is_overdue(&clock, date(2026, 3, 30)), "UTC day 31 is after due day 30");
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn part3_ymd_round_trips_through_days(days in -200_000i64..200_000) {
        let d = date(1970, 1, 1).add_days(days);
        let (y, m, dd) = d.ymd();
        prop_assert_eq!(Date::from_ymd(y, m, dd), Some(d));
    }
}

// ---------------------------------------------------------------- Part 4: charging and retries

#[test]
fn part4_successful_charge_returns_txn_and_sends_the_total() {
    let (result, fakes) = charge(&sample_invoice(), vec![ok("txn_1")], policy(4));
    assert_eq!(result.expect("charge succeeds"), "txn_1");
    let calls = fakes.calls();
    assert_eq!(calls.len(), 1, "one successful call, no retries");
    assert_eq!(calls[0].amount, Money::from_cents(97_200), "charge the total, not the subtotal");
    assert_eq!(calls[0].currency, "USD");
    assert_eq!(calls[0].token, "tok_visa");
}

#[test]
fn part4_idempotency_key_is_the_invoice_id() {
    let (_, fakes) = charge(&sample_invoice(), vec![ok("txn_1")], policy(4));
    assert_eq!(fakes.calls()[0].key, "INV-1", "key must be the invoice id, not the token");
}

#[test]
fn part4_zero_total_is_rejected_before_any_gateway_call() {
    let free = invoice(vec![line("FREE", 0, 1)]);
    let (result, fakes) = charge(&free, vec![], policy(4));
    assert!(matches!(result, Err(BillingError::InvalidInvoice(_))), "got {result:?}");
    assert!(fakes.calls().is_empty(), "no gateway call for a zero total");
}

#[test]
fn part4_declined_charge_is_never_retried() {
    let (result, fakes) = charge(
        &sample_invoice(),
        vec![Err(GatewayError::Declined("card refused".to_owned()))],
        policy(4),
    );
    match result {
        Err(BillingError::Declined(reason)) => assert_eq!(reason, "card refused"),
        other => panic!("expected Declined, got {other:?}"),
    }
    assert_eq!(fakes.calls().len(), 1, "declines are not retried");
    assert!(fakes.sleeps().is_empty(), "no backoff for a decline");
}

#[test]
fn part4_fatal_gateway_error_is_surfaced_without_retry() {
    let (result, fakes) = charge(
        &sample_invoice(),
        vec![Err(GatewayError::Fatal("bad api key".to_owned()))],
        policy(4),
    );
    match result {
        Err(BillingError::Gateway(GatewayError::Fatal(msg))) => assert_eq!(msg, "bad api key"),
        other => panic!("expected Gateway(Fatal), got {other:?}"),
    }
    assert_eq!(fakes.calls().len(), 1, "fatal errors are not retried");
}

#[test]
fn part4_transient_failures_are_retried_until_success() {
    let (result, fakes) = charge(
        &sample_invoice(),
        vec![transient("t1"), transient("t2"), ok("txn_9")],
        policy(4),
    );
    assert_eq!(result.expect("third attempt succeeds"), "txn_9");
    assert_eq!(fakes.calls().len(), 3);
    assert_eq!(
        fakes.sleeps(),
        vec![Duration::from_millis(100), Duration::from_millis(200)],
        "backoff before each retry"
    );
}

#[test]
fn part4_exhausted_retries_report_attempts_and_last_error() {
    let (result, fakes) = charge(
        &sample_invoice(),
        vec![transient("t1"), transient("t2"), transient("t3"), transient("t4")],
        policy(4),
    );
    match result {
        Err(BillingError::PaymentFailed {
            invoice_id,
            attempts,
            last_error,
        }) => {
            assert_eq!(invoice_id, "INV-1");
            assert_eq!(attempts, 4, "exactly max_attempts attempts");
            assert_eq!(last_error, GatewayError::Transient("t4".to_owned()));
        }
        other => panic!("expected PaymentFailed, got {other:?}"),
    }
    assert_eq!(fakes.calls().len(), 4);
}

#[test]
fn part4_no_sleep_after_the_final_attempt() {
    let (_, fakes) = charge(
        &sample_invoice(),
        vec![transient("t1"), transient("t2"), transient("t3"), transient("t4")],
        policy(4),
    );
    assert_eq!(fakes.sleeps().len(), 3, "sleep between attempts only");
}

#[test]
fn part4_backoff_doubles_from_the_base_delay() {
    let (_, fakes) = charge(
        &sample_invoice(),
        vec![transient("a"), transient("b"), transient("c"), transient("d")],
        policy(4),
    );
    assert_eq!(
        fakes.sleeps(),
        vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(400),
        ]
    );
}

#[test]
fn part4_backoff_is_capped_at_max_delay() {
    let capped = RetryPolicy::new(5, Duration::from_secs(1), Duration::from_secs(3))
        .expect("valid policy");
    let (_, fakes) = charge(
        &sample_invoice(),
        vec![transient("a"), transient("b"), transient("c"), transient("d"), transient("e")],
        capped,
    );
    assert_eq!(
        fakes.sleeps(),
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_secs(3),
        ],
        "1s, 2s, then capped at 3s"
    );
}

#[test]
fn part4_backoff_table_for_default_policy() {
    let p = RetryPolicy::default();
    let delays: Vec<Duration> = (1..=5).map(|attempt| p.backoff_delay(attempt)).collect();
    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
            Duration::from_secs(8),
        ]
    );
}

#[test]
fn part4_backoff_saturates_for_huge_attempt_numbers() {
    let p = RetryPolicy::default();
    assert_eq!(p.backoff_delay(1_000), Duration::from_secs(8));
    assert_eq!(p.backoff_delay(u32::MAX), Duration::from_secs(8));
}

#[test]
fn part4_retry_policy_rejects_zero_attempts() {
    let err = RetryPolicy::new(0, Duration::from_millis(1), Duration::from_millis(2))
        .expect_err("zero attempts must be rejected");
    assert!(matches!(err, BillingError::InvalidPolicy(_)), "got {err:?}");
}

#[test]
fn part4_single_attempt_policy_gives_up_on_first_transient() {
    let (result, fakes) = charge(&sample_invoice(), vec![transient("t1")], policy(1));
    match result {
        Err(BillingError::PaymentFailed { attempts, .. }) => assert_eq!(attempts, 1),
        other => panic!("expected PaymentFailed, got {other:?}"),
    }
    assert!(fakes.sleeps().is_empty(), "nothing to wait for with one attempt");
}

#[test]
fn part4_transient_classification_matches_the_retry_rule() {
    assert!(GatewayError::Transient("timeout".to_owned()).is_transient());
    assert!(!GatewayError::Declined("no funds".to_owned()).is_transient());
    assert!(!GatewayError::Fatal("bad key".to_owned()).is_transient());
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn part4_calls_never_exceed_max_attempts(
        max in 1u32..=6,
        outcomes in prop::collection::vec(prop::bool::ANY, 0..10),
    ) {
        // `true` = transient failure, `false` = success. Padded so the fake is never over-called.
        let mut script: Vec<Result<String, GatewayError>> = outcomes
            .iter()
            .map(|&fails| if fails { transient("t") } else { ok("txn") })
            .collect();
        script.resize_with(10, || transient("t"));
        let (_, fakes) = charge(&sample_invoice(), script, policy(max));
        let calls = fakes.calls().len();
        prop_assert!(calls <= max as usize, "{calls} calls exceed max_attempts {max}");
        prop_assert_eq!(fakes.sleeps().len() + 1, calls, "one sleep between each pair of attempts");
    }
}

// ---------------------------------------------------------------- Part 5: receipts and logs

#[test]
fn part5_receipt_is_sent_after_a_successful_charge() {
    let (result, fakes) = charge(&sample_invoice(), vec![ok("txn_9")], policy(4));
    assert_eq!(result.expect("charge succeeds"), "txn_9");
    let emails = fakes.emails();
    assert_eq!(emails.len(), 1, "one receipt per successful charge");
    assert_eq!(emails[0].to, "ada@example.com");
    assert_eq!(emails[0].subject, "Receipt for invoice INV-1");
    assert!(emails[0].body.contains("Total: 972.00 USD"), "body: {}", emails[0].body);
    assert!(emails[0].body.contains("Transaction: txn_9"), "body: {}", emails[0].body);
}

#[test]
fn part5_receipt_states_the_payment_date_in_utc() {
    let (_, fakes) = charge_at(
        at(2026, 3, 31, 2),
        &sample_invoice(),
        vec![ok("txn_9")],
        policy(4),
        false,
    );
    assert!(
        fakes.emails()[0].body.contains("Paid on: 2026-03-31"),
        "body: {}",
        fakes.emails()[0].body
    );
}

#[test]
fn part5_no_receipt_when_the_charge_is_declined() {
    let (_, fakes) = charge(
        &sample_invoice(),
        vec![Err(GatewayError::Declined("no".to_owned()))],
        policy(4),
    );
    assert!(fakes.emails().is_empty(), "no receipt without a payment");
}

#[test]
fn part5_receipt_failure_does_not_fail_the_charge() {
    let (result, fakes) = charge_at(
        at(2026, 3, 1, 12),
        &sample_invoice(),
        vec![ok("txn_9")],
        policy(4),
        true,
    );
    assert_eq!(result.expect("money moved, so the charge is Ok"), "txn_9");
    assert_eq!(fakes.emails().len(), 1, "delivery was attempted");
}

#[test]
fn part5_receipt_failure_is_logged_at_error_level() {
    let (_, logs) = capture(|| {
        charge_at(
            at(2026, 3, 1, 12),
            &sample_invoice(),
            vec![ok("txn_9")],
            policy(4),
            true,
        )
    });
    assert!(
        has_log(&logs, "ERROR", "receipt delivery failed for invoice INV-1 (txn txn_9)"),
        "logs: {logs:?}"
    );
}

#[test]
fn part5_successful_charge_is_logged_at_info_level() {
    let (_, logs) = capture(|| charge(&sample_invoice(), vec![ok("txn_9")], policy(4)));
    assert!(has_log(&logs, "INFO", "charged invoice INV-1 (txn txn_9)"), "logs: {logs:?}");
}

#[test]
fn part5_declines_are_logged_at_error_level() {
    let (_, logs) = capture(|| {
        charge(
            &sample_invoice(),
            vec![Err(GatewayError::Declined("card refused".to_owned()))],
            policy(4),
        )
    });
    assert!(
        has_log(&logs, "ERROR", "payment declined for invoice INV-1: card refused"),
        "logs: {logs:?}"
    );
}

#[test]
fn part5_each_transient_retry_is_logged_as_a_warning_with_attempt_number() {
    let (_, logs) = capture(|| {
        charge(
            &sample_invoice(),
            vec![transient("t1"), ok("txn_9")],
            policy(3),
        )
    });
    assert!(has_log(&logs, "WARN", "attempt 1 of 3"), "logs: {logs:?}");
}

#[test]
fn part5_giving_up_is_logged_at_error_level() {
    let (_, logs) = capture(|| charge(&sample_invoice(), vec![transient("t1")], policy(1)));
    assert!(
        has_log(&logs, "ERROR", "giving up on invoice INV-1 after 1 attempts"),
        "logs: {logs:?}"
    );
}

// ---------------------------------------------------------------- Part 6: CSV export

const HEADER: &str = "invoice_id,customer_email,issued_on,currency,subtotal,discount,tax,total\n";

fn export(
    dir: &Path,
    name: &str,
    invoices: Vec<Result<Invoice, BillingError>>,
) -> (Result<usize, BillingError>, PathBuf) {
    let path = dir.join(name);
    let result = export_invoices_csv(invoices, &path);
    (result, path)
}

#[test]
fn part6_export_writes_header_and_one_row_per_invoice() {
    let dir = TempDir::new().expect("tempdir");
    let second = Invoice::new(
        "INV-2",
        "bob@example.com",
        vec![line("A", 50_000, 2)],
        date(2026, 3, 2),
    );
    let (result, path) = export(
        dir.path(),
        "out.csv",
        vec![Ok(sample_invoice()), second],
    );
    assert_eq!(result.expect("export succeeds"), 2);
    let text = fs::read_to_string(&path).expect("read export");
    assert_eq!(
        text,
        format!(
            "{HEADER}INV-1,ada@example.com,2026-03-01,USD,1000.00,100.00,72.00,972.00\n\
             INV-2,bob@example.com,2026-03-02,USD,100.00,2.00,0.00,98.00\n"
        )
    );
}

#[test]
fn part6_empty_export_writes_only_the_header() {
    let dir = TempDir::new().expect("tempdir");
    let (result, path) = export(dir.path(), "empty.csv", vec![]);
    assert_eq!(result.expect("export succeeds"), 0);
    assert_eq!(fs::read_to_string(&path).expect("read"), HEADER);
}

#[test]
fn part6_fields_with_commas_and_quotes_are_rfc4180_quoted() {
    let dir = TempDir::new().expect("tempdir");
    let tricky = Invoice::new(
        r#"INV "7", x"#,
        "ada@example.com",
        vec![line("A", 1_000_000, 1)],
        date(2026, 3, 1),
    )
    .expect("has lines");
    let (result, path) = export(dir.path(), "quoted.csv", vec![Ok(tricky)]);
    result.expect("export succeeds");
    let text = fs::read_to_string(&path).expect("read");
    let row = text.lines().nth(1).expect("one data row");
    assert_eq!(
        row,
        r#""INV ""7"", x",ada@example.com,2026-03-01,USD,1000.00,100.00,0.00,900.00"#
    );
}

#[test]
fn part6_export_replaces_an_existing_file() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("report.csv");
    fs::write(&path, "stale contents\n").expect("seed file");
    let (result, path) = export(dir.path(), "report.csv", vec![Ok(sample_invoice())]);
    result.expect("export succeeds");
    let text = fs::read_to_string(&path).expect("read");
    assert!(!text.contains("stale"), "old contents must be replaced");
    assert!(text.starts_with(HEADER));
}

#[test]
fn part6_mid_stream_error_keeps_the_previous_file() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("report.csv");
    fs::write(&path, "keep me\n").expect("seed file");
    let items = vec![
        Ok(sample_invoice()),
        Err(BillingError::InvalidInvoice("boom".to_owned())),
    ];
    let (result, path) = export(dir.path(), "report.csv", items);
    assert!(matches!(result, Err(BillingError::InvalidInvoice(_))), "got {result:?}");
    assert_eq!(fs::read_to_string(&path).expect("read"), "keep me\n", "target untouched");
}

#[test]
fn part6_failed_export_leaves_no_temporary_files() {
    let dir = TempDir::new().expect("tempdir");
    let items = vec![Err(BillingError::InvalidInvoice("boom".to_owned()))];
    let (result, _) = export(dir.path(), "report.csv", items);
    assert!(result.is_err());
    assert!(dir_entries(dir.path()).is_empty(), "temp file must be removed on error");
}

#[test]
fn part6_successful_export_leaves_only_the_target_file() {
    let dir = TempDir::new().expect("tempdir");
    let (result, _) = export(dir.path(), "report.csv", vec![Ok(sample_invoice())]);
    result.expect("export succeeds");
    assert_eq!(dir_entries(dir.path()), vec!["report.csv".to_owned()]);
}

#[test]
fn part6_missing_directory_is_an_io_error() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("no-such-dir").join("report.csv");
    let result = export_invoices_csv(vec![Ok(sample_invoice())], &path);
    assert!(matches!(result, Err(BillingError::Io(_))), "got {result:?}");
}
