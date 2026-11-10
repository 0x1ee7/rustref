# 16 — Testing a billing module

## Scenario

A billing module already exists and ships to production. It turns line items into
invoices, applies volume discounts and tax, charges a payment gateway with
retries, sends receipts, and exports invoices to CSV. Nobody wrote its tests.
Your job is to write the test suite that a reviewer would sign off on.

The production code is in `billing/billing.rs`. You do **not** change it. You
write `suite/billing_suite.rs` against its public API. Your suite is then graded
by mutation testing: the grader plants 20 realistic bugs in a copy of the
production code and checks that your suite catches each one.

## The API under test

Everything is in `ex16_testing::billing` (the `src/lib.rs` re-export).

| Area | Items |
|---|---|
| Money and lines | `Money` (signed cents), `LineItem::new`, `line_total`, `Invoice::new`, `with_tax_rate_bp`, `with_currency`, `invoice_totals` → `Totals` |
| Discounts | `discount_rate_bp` (0 / 2% / 5% / 10% tiers at 100.00, 500.00 and 1000.00 thresholds, inclusive) |
| Dates and time | `Date::from_ymd`, `add_days`, `ymd`, `Timestamp::from_unix_seconds`, `utc_date`, `Clock`, `due_date`, `issue_due_date`, `is_overdue` |
| Charging | `Gateway`, `GatewayError`, `Notifier`, `NotifyError`, `Sleeper`, `RetryPolicy`, `PaymentService::charge` |
| Export | `export_invoices_csv`, `BillingError` |

Read the doc comments in `billing.rs`; they are part of the contract.

## Parts

### Part 1 — Line items, money and totals
Line totals (including half-cent rounding and sub-cent unit prices), validation
of lines and invoices, the subtotal, `Money` display and arithmetic, and a
property test for the total identity `total = subtotal − discount + tax`.

### Part 2 — Discounts and tax
The discount tier table and its inclusive thresholds, half-cent rounding of the
discount, the order of discount then tax, and monotonicity of the tier function
(property test).

### Part 3 — Dates, timestamps and due dates
Calendar validity (leap years included), ISO display, day arithmetic across
month, year and leap boundaries, UTC day mapping for timestamps (including
negative ones), due dates from the clock, and the overdue rule at the UTC day
boundary (property test for the civil-date round trip).

### Part 4 — Charging and retries
Happy path (amount, currency, token, idempotency key), zero-total rejection,
declines and fatal errors that are never retried, transient retries with
backoff, exhaustion, the backoff table and cap, saturation, policy validation,
and a property test bounding the number of gateway calls.

### Part 5 — Receipts and logs
Receipt content and delivery, the rule that a receipt failure does not fail the
charge, and the log levels and messages (`INFO`, `WARN`, `ERROR`) captured with a
scoped `tracing` subscriber.

### Part 6 — CSV export
Exact output, RFC 4180 quoting, replacing an existing file, leaving the old file
intact when the stream fails mid-way, leaving no temporary files behind on
success or failure, and reporting I/O errors.

## Concepts exercised

- Black-box unit and integration tests against a public API, with the
  `tests/` vs `#[cfg(test)]` distinction
- Hand-written fakes for ports (`Gateway`, `Notifier`, `Sleeper`, `Clock`) using
  `RefCell` state, and `impl Trait for &Fake` so the test keeps ownership and can
  inspect calls afterwards
- Asserting on call sequences, arguments and sleeps, not only return values
- Pinning exact error variants with `matches!` and clear assertion messages
- Table-driven tests and boundary values (inclusive thresholds, the first
  instant after a day ends, the exact cap)
- Property-based tests with `proptest` and a fixed seed (determinism)
- Capturing `tracing` output with a scoped subscriber (`with_default`, never a
  global one)
- Temporary directories with `tempfile` and filesystem assertions (atomic
  replace, no leftovers)
- Killing mutants: thinking in terms of "which bug would this test catch?"

## Grading model

`tests/grader.rs` is the grader. It is not part of your suite. It runs your suite
against the unmodified billing code (it must pass), then against 20 mutants of
the billing code. Each mutant is a small, plausible bug. A mutant is "killed" when
your suite compiles against it and at least one of your tests fails. The grader
requires at least 30 tests and must finish within its time budget.

The categories of the 20 mutants are: rounding direction, threshold
inclusivity, discount ordering, tax base, input validation, boundary off-by-one,
timezone shift, backoff shape and cap, attempt counting, sleep usage, error
classification, idempotency key, charged amount, receipt delivery, error
propagation, log level, and temporary-file handling. The grader does not reveal
the exact mutations.

## Run

```
cargo test -p ex16_testing                  # your suite (grader runs too; it should fail until the suite is written)
cargo test -p ex16_testing part2            # only Part 2 tests (grader's part2_* included)
cargo solutions -p ex16_testing             # reference solution: all pass
```

## Stretch

- Add a test for `Clock` injection via a real clock wrapper and show why it
  cannot be tested deterministically.
- Write a `Gateway` fake that returns `Transient` forever and check the retry
  loop terminates.
- Add a snapshot test for the CSV export using `insta` (not a workspace
  dependency; discuss the trade-off instead).
- Mutate the production code yourself (copy `billing.rs` to a scratch crate),
  find a mutant your suite misses, and fix the suite.
