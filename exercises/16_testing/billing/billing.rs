//! Billing core: invoice totals, payment charging with retries, receipts,
//! due dates and atomic CSV export.
//!
//! This is production code under test. Do not edit it: the exercise is the
//! test suite you write in `suite/billing_suite.rs`.
//!
//! Money is kept in integer cents and rates in basis points (1 bp = 0.01%), so
//! every calculation is exact. The only rounding happens in `round_div`.

use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::iter::Sum;
use std::ops::{Add, Sub};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::{error, info, warn};

/// Basis points in 100%.
const FULL_RATE_BP: u32 = 10_000;
/// Mills (thousandths of a currency unit) per cent.
const MILLS_PER_CENT: i64 = 10;
const SECONDS_PER_DAY: i64 = 86_400;

/// Columns of the CSV export, in order.
pub const EXPORT_FIELDS: [&str; 8] = [
    "invoice_id",
    "customer_email",
    "issued_on",
    "currency",
    "subtotal",
    "discount",
    "tax",
    "total",
];

// ---------------------------------------------------------------- errors

/// Everything that can go wrong in this module.
#[derive(Debug)]
pub enum BillingError {
    /// A line item or invoice violates a business rule.
    InvalidInvoice(String),
    /// Payment terms are invalid (negative `net_days`).
    InvalidTerms(String),
    /// A retry policy is invalid (zero attempts).
    InvalidPolicy(String),
    /// The issuer refused the charge. Never retried.
    Declined(String),
    /// A non-retryable gateway failure other than a decline. Never retried.
    Gateway(GatewayError),
    /// Every attempt failed transiently. `source()` is the last gateway error.
    PaymentFailed {
        invoice_id: String,
        attempts: u32,
        last_error: GatewayError,
    },
    /// Export I/O failure.
    Io(io::Error),
}

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInvoice(msg)
            | Self::InvalidTerms(msg)
            | Self::InvalidPolicy(msg)
            | Self::Declined(msg) => f.write_str(msg),
            Self::Gateway(err) => write!(f, "gateway error: {err}"),
            Self::PaymentFailed {
                invoice_id,
                attempts,
                ..
            } => write!(
                f,
                "invoice {invoice_id}: gateway unavailable after {attempts} attempts"
            ),
            Self::Io(err) => write!(f, "export I/O failed: {err}"),
        }
    }
}

impl Error for BillingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Gateway(err) => Some(err),
            Self::PaymentFailed { last_error, .. } => Some(last_error),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for BillingError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

// ---------------------------------------------------------------- money

/// Signed amount in whole cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);

    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Self {
        iter.fold(Self::ZERO, Add::add)
    }
}

/// `12.34`, `-0.05`: always two decimals.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

/// Divides and rounds to the nearest integer, ties away from zero.
/// Callers only pass a non-negative numerator and a positive denominator.
fn round_div(numerator: i64, denominator: i64) -> i64 {
    debug_assert!(
        numerator >= 0 && denominator > 0,
        "round_div expects a non-negative numerator and a positive denominator"
    );
    (numerator + denominator / 2) / denominator
}

/// `amount * rate_bp / 10_000`, rounded half-up to the cent.
fn apply_rate_bp(amount: Money, rate_bp: u32) -> Money {
    Money::from_cents(round_div(
        amount.cents() * i64::from(rate_bp),
        i64::from(FULL_RATE_BP),
    ))
}

// ---------------------------------------------------------------- invoices

/// One priced line. The unit price is in mills (thousandths of the currency
/// unit), so a line can carry a sub-cent price; its total is rounded to cents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItem {
    sku: String,
    unit_price_mills: i64,
    quantity: i64,
}

impl LineItem {
    /// Fails when the SKU is empty, the unit price is negative, or the
    /// quantity is below one. A zero unit price is allowed (free item).
    pub fn new(
        sku: impl Into<String>,
        unit_price_mills: i64,
        quantity: i64,
    ) -> Result<Self, BillingError> {
        let sku = sku.into();
        if sku.is_empty() {
            return Err(BillingError::InvalidInvoice(
                "sku must be non-empty".to_owned(),
            ));
        }
        if unit_price_mills < 0 {
            return Err(BillingError::InvalidInvoice(format!(
                "unit price for {sku} must not be negative"
            )));
        }
        if quantity < 1 {
            return Err(BillingError::InvalidInvoice(format!(
                "quantity for {sku} must be >= 1, got {quantity}"
            )));
        }
        Ok(Self {
            sku,
            unit_price_mills,
            quantity,
        })
    }

    pub fn sku(&self) -> &str {
        &self.sku
    }

    pub fn unit_price_mills(&self) -> i64 {
        self.unit_price_mills
    }

    pub fn quantity(&self) -> i64 {
        self.quantity
    }

    /// `unit_price * quantity`, rounded half-up to the cent.
    pub fn line_total(&self) -> Money {
        Money::from_cents(round_div(
            self.unit_price_mills * self.quantity,
            MILLS_PER_CENT,
        ))
    }
}

/// A customer invoice: lines plus tax rate and currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    invoice_id: String,
    customer_email: String,
    lines: Vec<LineItem>,
    issued_on: Date,
    tax_rate_bp: u32,
    currency: String,
}

impl Invoice {
    /// Creates a USD invoice with no tax. Fails when there are no lines.
    pub fn new(
        invoice_id: impl Into<String>,
        customer_email: impl Into<String>,
        lines: Vec<LineItem>,
        issued_on: Date,
    ) -> Result<Self, BillingError> {
        let invoice_id = invoice_id.into();
        if lines.is_empty() {
            return Err(BillingError::InvalidInvoice(format!(
                "invoice {invoice_id} has no line items"
            )));
        }
        Ok(Self {
            invoice_id,
            customer_email: customer_email.into(),
            lines,
            issued_on,
            tax_rate_bp: 0,
            currency: "USD".to_owned(),
        })
    }

    /// Sets the tax rate in basis points. Must be in `[0, 10_000)`.
    pub fn with_tax_rate_bp(mut self, tax_rate_bp: u32) -> Result<Self, BillingError> {
        if tax_rate_bp >= FULL_RATE_BP {
            return Err(BillingError::InvalidInvoice(format!(
                "tax rate must be below {FULL_RATE_BP} bp, got {tax_rate_bp}"
            )));
        }
        self.tax_rate_bp = tax_rate_bp;
        Ok(self)
    }

    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    pub fn invoice_id(&self) -> &str {
        &self.invoice_id
    }

    pub fn customer_email(&self) -> &str {
        &self.customer_email
    }

    pub fn lines(&self) -> &[LineItem] {
        &self.lines
    }

    pub fn issued_on(&self) -> Date {
        self.issued_on
    }

    pub fn tax_rate_bp(&self) -> u32 {
        self.tax_rate_bp
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}

/// Volume discount in basis points for a subtotal: 10% from 1 000.00, 5% from
/// 500.00, 2% from 100.00. Thresholds are inclusive.
pub fn discount_rate_bp(subtotal: Money) -> u32 {
    if subtotal >= Money::from_cents(100_000) {
        1_000
    } else if subtotal >= Money::from_cents(50_000) {
        500
    } else if subtotal >= Money::from_cents(10_000) {
        200
    } else {
        0
    }
}

/// The money breakdown of an invoice.
///
/// Invariant: `total == subtotal - discount + tax`. Tax is charged on the
/// discounted amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub subtotal: Money,
    pub discount: Money,
    pub tax: Money,
    pub total: Money,
}

/// Computes the totals of an invoice. Pure: no I/O, no clock.
pub fn invoice_totals(invoice: &Invoice) -> Totals {
    let subtotal: Money = invoice.lines.iter().map(LineItem::line_total).sum();
    let discount = apply_rate_bp(subtotal, discount_rate_bp(subtotal));
    let taxable = subtotal - discount;
    let tax = apply_rate_bp(taxable, invoice.tax_rate_bp);
    Totals {
        subtotal,
        discount,
        tax,
        total: taxable + tax,
    }
}

// ---------------------------------------------------------------- dates & clocks

/// Calendar date in the proleptic Gregorian calendar. No time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    days_since_epoch: i64,
}

impl Date {
    /// `None` when the month or day does not exist (including Feb 29 in
    /// non-leap years).
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self {
            days_since_epoch: days_from_civil(year, month, day),
        })
    }

    /// `(year, month, day)`.
    pub fn ymd(self) -> (i32, u32, u32) {
        civil_from_days(self.days_since_epoch)
    }

    /// Adds (or subtracts, for negative `days`) whole days.
    pub fn add_days(self, days: i64) -> Self {
        Self {
            days_since_epoch: self.days_since_epoch + days,
        }
    }
}

/// ISO 8601: `2026-11-10`.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(if month <= 2 { year - 1 } else { year });
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12; // March = 0
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

/// An instant: seconds since the Unix epoch, UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    unix_seconds: i64,
}

impl Timestamp {
    pub const fn from_unix_seconds(unix_seconds: i64) -> Self {
        Self { unix_seconds }
    }

    pub const fn unix_seconds(self) -> i64 {
        self.unix_seconds
    }

    /// The UTC calendar day this instant falls on.
    pub fn utc_date(self) -> Date {
        Date {
            days_since_epoch: self.unix_seconds.div_euclid(SECONDS_PER_DAY),
        }
    }
}

/// Source of the current time. Injected so that code is testable.
pub trait Clock {
    fn now(&self) -> Timestamp;
}

/// The wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
            Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(i64::MAX),
        };
        Timestamp::from_unix_seconds(secs)
    }
}

/// Due date: `issued_on` plus `net_days`. Fails on negative terms.
pub fn due_date(issued_on: Date, net_days: i64) -> Result<Date, BillingError> {
    if net_days < 0 {
        return Err(BillingError::InvalidTerms(format!(
            "net_days must be >= 0, got {net_days}"
        )));
    }
    Ok(issued_on.add_days(net_days))
}

/// Due date counted from the clock's UTC day.
pub fn issue_due_date(clock: &impl Clock, net_days: i64) -> Result<Date, BillingError> {
    due_date(clock.now().utc_date(), net_days)
}

/// An invoice is overdue only after its due day has ended (UTC).
pub fn is_overdue(clock: &impl Clock, invoice_due: Date) -> bool {
    clock.now().utc_date() > invoice_due
}

// ---------------------------------------------------------------- ports

/// Why a charge attempt failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayError {
    /// Timeout or 503. Safe to retry.
    Transient(String),
    /// The issuer refused the card. Never retried.
    Declined(String),
    /// Anything else (bad credentials, malformed request). Never retried.
    Fatal(String),
}

impl GatewayError {
    /// Only transient failures are safe to retry.
    pub fn is_transient(&self) -> bool {
        matches!(self, GatewayError::Transient(_))
    }
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transient(msg) | Self::Declined(msg) | Self::Fatal(msg) => f.write_str(msg),
        }
    }
}

impl Error for GatewayError {}

/// One charge request, borrowed from the invoice.
#[derive(Debug, Clone, Copy)]
pub struct ChargeRequest<'a> {
    pub amount: Money,
    pub currency: &'a str,
    pub token: &'a str,
    pub idempotency_key: &'a str,
}

/// Payment gateway port. Returns the transaction id on success.
pub trait Gateway {
    fn charge(&self, request: &ChargeRequest<'_>) -> Result<String, GatewayError>;
}

/// A receipt could not be delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyError(pub String);

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for NotifyError {}

/// Outbound email port.
pub trait Notifier {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), NotifyError>;
}

/// Blocking delay port, so tests never really sleep.
pub trait Sleeper {
    fn sleep(&self, duration: Duration);
}

/// Sleeps the current thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

// ---------------------------------------------------------------- retries

/// Attempts and backoff for transient gateway failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl RetryPolicy {
    /// Fails when `max_attempts` is zero.
    pub fn new(
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
    ) -> Result<Self, BillingError> {
        if max_attempts == 0 {
            return Err(BillingError::InvalidPolicy(
                "max_attempts must be >= 1, got 0".to_owned(),
            ));
        }
        Ok(Self {
            max_attempts,
            base_delay,
            max_delay,
        })
    }

    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    pub fn base_delay(&self) -> Duration {
        self.base_delay
    }

    pub fn max_delay(&self) -> Duration {
        self.max_delay
    }

    /// Delay slept after failed attempt `attempt` (1-based):
    /// `base_delay * 2^(attempt - 1)`, capped at `max_delay`.
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(64);
        let nanos = self.base_delay.as_nanos().saturating_mul(1u128 << shift);
        let capped = nanos.min(self.max_delay.as_nanos());
        Duration::new(
            (capped / 1_000_000_000) as u64,
            (capped % 1_000_000_000) as u32,
        )
    }
}

impl Default for RetryPolicy {
    /// 4 attempts, 1 s base delay, 8 s cap.
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(8),
        }
    }
}

// ---------------------------------------------------------------- payments

/// Charges invoices through an injected gateway, retrying transient failures
/// with exponential backoff, and sends a receipt after each success.
pub struct PaymentService<G, N, C, S> {
    gateway: G,
    notifier: N,
    clock: C,
    sleeper: S,
    policy: RetryPolicy,
}

impl<G: Gateway, N: Notifier, C: Clock, S: Sleeper> PaymentService<G, N, C, S> {
    pub fn new(gateway: G, notifier: N, clock: C, sleeper: S, policy: RetryPolicy) -> Self {
        Self {
            gateway,
            notifier,
            clock,
            sleeper,
            policy,
        }
    }

    /// Charges the invoice total and returns the gateway transaction id.
    ///
    /// - A total at or below zero fails with `InvalidInvoice` before any call.
    /// - `Declined` and `Fatal` failures are returned at once, never retried.
    /// - Transient failures are retried after `backoff_delay`, with no sleep
    ///   after the last attempt. Exhaustion yields `PaymentFailed`.
    /// - The gateway uses the invoice id as the idempotency key.
    /// - A receipt failure is logged and does not fail the charge.
    pub fn charge(&self, invoice: &Invoice, token: &str) -> Result<String, BillingError> {
        let totals = invoice_totals(invoice);
        if totals.total <= Money::ZERO {
            return Err(BillingError::InvalidInvoice(format!(
                "invoice {} total must be positive, got {}",
                invoice.invoice_id, totals.total
            )));
        }

        let max_attempts = self.policy.max_attempts();
        let mut attempt = 1_u32;
        loop {
            let request = ChargeRequest {
                amount: totals.total,
                currency: &invoice.currency,
                token,
                idempotency_key: &invoice.invoice_id,
            };
            match self.gateway.charge(&request) {
                Ok(txn_id) => {
                    info!("charged invoice {} (txn {txn_id})", invoice.invoice_id);
                    self.send_receipt(invoice, &totals, &txn_id);
                    return Ok(txn_id);
                }
                Err(GatewayError::Declined(reason)) => {
                    error!("payment declined for invoice {}: {reason}", invoice.invoice_id);
                    return Err(BillingError::Declined(reason));
                }
                Err(err) if err.is_transient() => {
                    warn!(
                        "transient gateway error for invoice {} (attempt {attempt} of {max_attempts}): {err}",
                        invoice.invoice_id
                    );
                    if attempt == max_attempts {
                        error!(
                            "giving up on invoice {} after {attempt} attempts",
                            invoice.invoice_id
                        );
                        return Err(BillingError::PaymentFailed {
                            invoice_id: invoice.invoice_id.clone(),
                            attempts: attempt,
                            last_error: err,
                        });
                    }
                    self.sleeper.sleep(self.policy.backoff_delay(attempt));
                    attempt += 1;
                }
                Err(err) => return Err(BillingError::Gateway(err)),
            }
        }
    }

    fn send_receipt(&self, invoice: &Invoice, totals: &Totals, txn_id: &str) {
        let paid_on = self.clock.now().utc_date();
        let subject = format!("Receipt for invoice {}", invoice.invoice_id);
        let body = format!(
            "Invoice {}\nTotal: {} {}\nPaid on: {paid_on}\nTransaction: {txn_id}\n",
            invoice.invoice_id, totals.total, invoice.currency
        );
        if let Err(err) = self
            .notifier
            .send(&invoice.customer_email, &subject, &body)
        {
            error!("receipt delivery failed for invoice {} (txn {txn_id}): {err}", invoice.invoice_id);
        }
    }
}

// ---------------------------------------------------------------- CSV export

/// Writes one row per invoice to `path`, with a header row.
///
/// The file is written to a temporary file in the same directory and renamed
/// into place, so readers see either the old file or the complete new one.
/// On any error the temporary file is removed and `path` is left unchanged.
/// An `Err` item from `invoices` aborts the export.
///
/// Returns the number of invoice rows written.
pub fn export_invoices_csv<I>(invoices: I, path: &Path) -> Result<usize, BillingError>
where
    I: IntoIterator<Item = Result<Invoice, BillingError>>,
{
    let dir = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let (tmp_path, file) = create_temp_file(dir, path)?;
    let result = write_csv(file, invoices).and_then(|rows| {
        fs::rename(&tmp_path, path)?;
        Ok(rows)
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Creates a new, uniquely named file in `dir` for a write destined for `target`.
fn create_temp_file(dir: &Path, target: &Path) -> io::Result<(PathBuf, File)> {
    let name = target.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "export path has no file name")
    })?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let tmp_path = dir.join(format!(
        ".{}.{}.{sequence}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)?;
    Ok((tmp_path, file))
}

fn write_csv<I>(file: File, invoices: I) -> Result<usize, BillingError>
where
    I: IntoIterator<Item = Result<Invoice, BillingError>>,
{
    let mut out = BufWriter::new(file);
    write_record(&mut out, EXPORT_FIELDS)?;
    let mut rows = 0;
    for invoice in invoices {
        let invoice = invoice?;
        let totals = invoice_totals(&invoice);
        let issued_on = invoice.issued_on.to_string();
        let subtotal = totals.subtotal.to_string();
        let discount = totals.discount.to_string();
        let tax = totals.tax.to_string();
        let total = totals.total.to_string();
        write_record(
            &mut out,
            [
                invoice.invoice_id.as_str(),
                invoice.customer_email.as_str(),
                issued_on.as_str(),
                invoice.currency.as_str(),
                subtotal.as_str(),
                discount.as_str(),
                tax.as_str(),
                total.as_str(),
            ],
        )?;
        rows += 1;
    }
    let file = out.into_inner().map_err(|err| err.into_error())?;
    file.sync_all()?;
    Ok(rows)
}

/// RFC 4180 record: quote fields containing `,`, `"`, or line breaks.
fn write_record<'a>(
    out: &mut impl Write,
    fields: impl IntoIterator<Item = &'a str>,
) -> io::Result<()> {
    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            out.write_all(b",")?;
        }
        if field.contains([',', '"', '\n', '\r']) {
            write!(out, "\"{}\"", field.replace('"', "\"\""))?;
        } else {
            out.write_all(field.as_bytes())?;
        }
    }
    out.write_all(b"\n")
}
