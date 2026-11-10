//! Mutation grader for your billing test suite. Do not edit.
//!
//! The grader builds a copy of `billing/billing.rs` with one deliberate bug
//! injected per mutant, compiles your suite against it, and checks that the
//! suite fails. A mutant that passes the suite "survives" and counts against you.
//!
//! The suite is `suite/billing_suite.rs` in the crate you run this from. The
//! reference `billing.rs` must pass it with at least `MIN_TESTS` tests in under
//! `TIME_BUDGET`.
//!
//! Every test here fails until your suite passes against the reference code.
//! Run `cargo test -p ex16_testing` to see which checks remain.

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MIN_TESTS: usize = 30;
const TIME_BUDGET: Duration = Duration::from_secs(5);
const RUN_TIMEOUT: Duration = Duration::from_secs(60);
const BILLING_SRC: &str = include_str!("../billing/billing.rs");
const LIB_SRC: &str = "pub mod billing;\n";
const CARGO_TOML: &str = r#"[package]
name = "ex16_testing"
version = "0.1.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
tracing = "0.1"

[dev-dependencies]
proptest = "1"
serde_json = "1"
tempfile = "3"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "registry", "fmt"] }
"#;

/// `(name, old, new)`: `old` must occur exactly once in `billing.rs`.
const MUTANTS: &[(&str, &str, &str)] = &[
    (
        "rounding_ties_down",
        r#"(numerator + denominator / 2) / denominator"#,
        r#"(numerator + denominator / 2 - 1) / denominator"#,
    ),
    (
        "tier_boundary_exclusive",
        r#"subtotal >= Money::from_cents(50_000)"#,
        r#"subtotal > Money::from_cents(50_000)"#,
    ),
    (
        "discount_not_removed_from_taxable",
        r#"let taxable = subtotal - discount;"#,
        r#"let taxable = subtotal;"#,
    ),
    (
        "tax_on_gross_subtotal",
        r#"let tax = apply_rate_bp(taxable, invoice.tax_rate_bp);"#,
        r#"let tax = apply_rate_bp(subtotal, invoice.tax_rate_bp);"#,
    ),
    (
        "zero_quantity_allowed",
        r#"if quantity < 1 {"#,
        r#"if quantity < 0 {"#,
    ),
    (
        "overdue_on_due_day",
        r#"clock.now().utc_date() > invoice_due"#,
        r#"clock.now().utc_date() >= invoice_due"#,
    ),
    (
        "clock_timezone_shift",
        r#"self.unix_seconds.div_euclid(SECONDS_PER_DAY)"#,
        r#"(self.unix_seconds - 5 * 3600).div_euclid(SECONDS_PER_DAY)"#,
    ),
    (
        "backoff_linear",
        r#"saturating_mul(1u128 << shift)"#,
        r#"saturating_mul(u128::from(shift) + 1)"#,
    ),
    (
        "backoff_uncapped",
        r#"let capped = nanos.min(self.max_delay.as_nanos());"#,
        r#"let capped = nanos;"#,
    ),
    (
        "one_attempt_short",
        r#"if attempt == max_attempts {"#,
        r#"if attempt + 1 == max_attempts {"#,
    ),
    (
        "sleep_ignores_backoff",
        r#"self.sleeper.sleep(self.policy.backoff_delay(attempt));"#,
        r#"self.sleeper.sleep(Duration::ZERO);"#,
    ),
    (
        "declined_reason_ignored",
        r#"Err(GatewayError::Declined(reason)) => {"#,
        r#"Err(GatewayError::Declined(reason)) if reason.is_empty() => {"#,
    ),
    (
        "transient_not_retried",
        r#"matches!(self, GatewayError::Transient(_))"#,
        r#"matches!(self, GatewayError::Transient(_) if false)"#,
    ),
    (
        "idempotency_key_is_token",
        r#"idempotency_key: &invoice.invoice_id,"#,
        r#"idempotency_key: token,"#,
    ),
    (
        "charge_amount_is_subtotal",
        r#"amount: totals.total,"#,
        r#"amount: totals.subtotal,"#,
    ),
    (
        "receipt_never_sent",
        r#"self.send_receipt(invoice, &totals, &txn_id);"#,
        r#"let _ = (invoice, &totals, &txn_id);"#,
    ),
    (
        "receipt_error_escapes",
        r#"error!("receipt delivery failed"#,
        r#"panic!("receipt delivery failed"#,
    ),
    (
        "receipt_failure_logged_as_info",
        r#"error!("receipt delivery failed"#,
        r#"info!("receipt delivery failed"#,
    ),
    (
        "export_leaks_temp_file",
        r#"let _ = fs::remove_file(&tmp_path);"#,
        r#"let _ = &tmp_path;"#,
    ),
    (
        "export_copy_instead_of_rename",
        r#"fs::rename(&tmp_path, path)?;"#,
        r#"fs::copy(&tmp_path, path)?;"#,
    ),
];

/// Result of compiling and running the suite against one billing variant.
#[derive(Clone, Debug)]
struct Outcome {
    compiled: bool,
    finished: bool,
    success: bool,
    passed: usize,
    failed: usize,
    elapsed: Duration,
    diagnostic: String,
}

static CACHE: Mutex<Option<HashMap<String, Outcome>>> = Mutex::new(None);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<repo>/` (the crate lives at `<repo>/exercises/16_testing` or `<repo>/solutions/16_testing`).
fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the repo root")
        .to_path_buf()
}

/// `exercises` or `solutions`.
fn mode_name() -> String {
    manifest_dir()
        .parent()
        .and_then(Path::file_name)
        .expect("crate is inside a mode directory")
        .to_string_lossy()
        .into_owned()
}

fn suite_source() -> String {
    let path = manifest_dir().join("suite/billing_suite.rs");
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

fn tail(text: &str) -> String {
    const MAX: usize = 1500;
    let bytes = text.as_bytes();
    let start = bytes.len().saturating_sub(MAX);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

/// Sums the `N passed` / `N failed` counts over every `test result:` line.
fn parse_counts(stdout: &str) -> (usize, usize) {
    let (mut passed, mut failed) = (0, 0);
    for line in stdout.lines().filter(|l| l.starts_with("test result:")) {
        for segment in line.split(';') {
            let Some((head, word)) = segment.trim().rsplit_once(' ') else {
                continue;
            };
            let count = head.rsplit(' ').next().and_then(|n| n.parse::<usize>().ok());
            match (word, count) {
                ("passed", Some(n)) => passed += n,
                ("failed", Some(n)) => failed += n,
                _ => {}
            }
        }
    }
    (passed, failed)
}

/// Finds the path of the compiled `suite` test executable in cargo's JSON output.
fn find_test_exe(stdout: &[u8]) -> Option<PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|msg| {
            let is_suite = msg["reason"] == "compiler-artifact"
                && msg["target"]["name"] == "suite"
                && msg["profile"]["test"] == true;
            if is_suite {
                msg["executable"].as_str().map(PathBuf::from)
            } else {
                None
            }
        })
}

/// Compiles and runs the suite against `billing_src` in a scratch crate.
fn evaluate(key: &str, billing_src: &str) -> Outcome {
    let root = repo_root();
    let work = root
        .join("target")
        .join("ex16-mutants")
        .join(mode_name())
        .join(key);
    let failed_setup = |diagnostic: String| Outcome {
        compiled: false,
        finished: false,
        success: false,
        passed: 0,
        failed: 0,
        elapsed: Duration::ZERO,
        diagnostic,
    };

    fs::create_dir_all(work.join("src")).expect("create scratch src dir");
    fs::create_dir_all(work.join("tests")).expect("create scratch tests dir");
    let manifest = format!(
        "{}\n[lib]\nname = \"ex16_testing\"\n",
        CARGO_TOML.replace("name = \"ex16_testing\"", &format!("name = \"ex16_{}_{key}\"", mode_name()))
    );
    fs::write(work.join("Cargo.toml"), manifest).expect("write scratch Cargo.toml");
    fs::copy(root.join("Cargo.lock"), work.join("Cargo.lock")).expect("copy Cargo.lock");
    fs::write(work.join("src/lib.rs"), LIB_SRC).expect("write scratch lib.rs");
    fs::write(work.join("src/billing.rs"), billing_src).expect("write scratch billing.rs");
    fs::write(work.join("tests/suite.rs"), suite_source()).expect("write scratch suite");

    let build = Command::new(env!("CARGO"))
        .current_dir(&work)
        .env("CARGO_TARGET_DIR", root.join("target/ex16-mutants"))
        .args([
            "test",
            "--offline",
            "--test",
            "suite",
            "--no-run",
            "--message-format=json",
        ])
        .output()
        .expect("spawn cargo");
    if !build.status.success() {
        return failed_setup(tail(&String::from_utf8_lossy(&build.stderr)));
    }
    let Some(exe) = find_test_exe(&build.stdout) else {
        return failed_setup("cargo did not report a suite test executable".to_owned());
    };

    let stdout_file = File::create(work.join("run-stdout.txt")).expect("create stdout file");
    let stderr_file = File::create(work.join("run-stderr.txt")).expect("create stderr file");
    let started = Instant::now();
    let mut child = Command::new(&exe)
        .current_dir(&work)
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()
        .expect("spawn suite executable");

    let (finished, status_success) = loop {
        match child.try_wait().expect("poll suite executable") {
            Some(status) => break (true, status.success()),
            None if started.elapsed() > RUN_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                break (false, false);
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let elapsed = started.elapsed();

    let stdout = fs::read_to_string(work.join("run-stdout.txt")).unwrap_or_default();
    let (passed, failed) = parse_counts(&stdout);
    let diagnostic = if finished {
        tail(&stdout)
    } else {
        format!("timed out after {RUN_TIMEOUT:?}")
    };
    Outcome {
        compiled: true,
        finished,
        success: finished && status_success,
        passed,
        failed,
        elapsed,
        diagnostic,
    }
}

/// Memoised per key. The lock is held while evaluating so cargo runs one at a time.
fn cached(key: &str, make: impl FnOnce() -> Outcome) -> Outcome {
    let mut guard = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    if let Some(outcome) = map.get(key) {
        return outcome.clone();
    }
    let outcome = make();
    map.insert(key.to_owned(), outcome.clone());
    outcome
}

fn baseline() -> Outcome {
    cached("baseline", || evaluate("baseline", BILLING_SRC))
}

fn mutant(name: &str) -> Outcome {
    let (_, old, new) = MUTANTS
        .iter()
        .find(|(n, _, _)| *n == name)
        .unwrap_or_else(|| panic!("unknown mutant {name}"));
    let occurrences = BILLING_SRC.matches(old).count();
    assert_eq!(
        occurrences, 1,
        "grader bug: mutant {name} pattern must occur exactly once in billing.rs"
    );
    let source = BILLING_SRC.replacen(old, new, 1);
    cached(name, || evaluate(name, &source))
}

fn assert_baseline_passes(base: &Outcome) {
    assert!(
        base.compiled,
        "suite does not compile against billing.rs:\n{}",
        base.diagnostic
    );
    assert!(
        base.finished,
        "suite did not finish within {RUN_TIMEOUT:?}"
    );
    assert!(
        base.success,
        "suite has failing tests against the reference billing.rs:\n{}",
        base.diagnostic
    );
    assert!(
        base.passed >= MIN_TESTS,
        "suite has {} tests, need at least {MIN_TESTS}",
        base.passed
    );
}

fn assert_killed(name: &str) {
    assert_baseline_passes(&baseline());
    let outcome = mutant(name);
    assert!(
        outcome.compiled && outcome.finished,
        "mutant {name} did not produce a runnable suite:\n{}",
        outcome.diagnostic
    );
    assert!(
        !outcome.success && outcome.failed >= 1,
        "mutant {name} survived: the suite still passes against the injected bug"
    );
}

#[test]
fn part1_suite_passes_with_at_least_min_tests() {
    assert_baseline_passes(&baseline());
}

macro_rules! killed_tests {
    ($($fn_name:ident => $mutant:literal),* $(,)?) => {
        $(
            #[test]
            fn $fn_name() {
                assert_killed($mutant);
            }
        )*
    };
}

killed_tests! {
    part2_rounding_ties_down_is_killed => "rounding_ties_down",
    part2_tier_boundary_exclusive_is_killed => "tier_boundary_exclusive",
    part2_discount_not_removed_from_taxable_is_killed => "discount_not_removed_from_taxable",
    part2_tax_on_gross_subtotal_is_killed => "tax_on_gross_subtotal",
    part2_zero_quantity_allowed_is_killed => "zero_quantity_allowed",
    part2_overdue_on_due_day_is_killed => "overdue_on_due_day",
    part2_clock_timezone_shift_is_killed => "clock_timezone_shift",
    part2_backoff_linear_is_killed => "backoff_linear",
    part2_backoff_uncapped_is_killed => "backoff_uncapped",
    part2_one_attempt_short_is_killed => "one_attempt_short",
    part2_sleep_ignores_backoff_is_killed => "sleep_ignores_backoff",
    part2_declined_reason_ignored_is_killed => "declined_reason_ignored",
    part2_transient_not_retried_is_killed => "transient_not_retried",
    part2_idempotency_key_is_token_is_killed => "idempotency_key_is_token",
    part2_charge_amount_is_subtotal_is_killed => "charge_amount_is_subtotal",
    part2_receipt_never_sent_is_killed => "receipt_never_sent",
    part2_receipt_error_escapes_is_killed => "receipt_error_escapes",
    part2_receipt_failure_logged_as_info_is_killed => "receipt_failure_logged_as_info",
    part2_export_leaks_temp_file_is_killed => "export_leaks_temp_file",
    part2_export_copy_instead_of_rename_is_killed => "export_copy_instead_of_rename",
}

#[test]
fn part3_suite_finishes_within_budget() {
    let base = baseline();
    assert_baseline_passes(&base);
    assert!(
        base.elapsed <= TIME_BUDGET,
        "suite took {:?}, budget is {TIME_BUDGET:?}",
        base.elapsed
    );
}
