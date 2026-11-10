//! Part 4: `bail!` and `ensure!`, early returns from `Result`-returning functions.

use std::cell::Cell;
use std::num::ParseIntError;

use ex20_macros::{bail, ensure};

#[derive(Debug, PartialEq, Eq)]
enum TransferError {
    NonPositive,
    Frozen,
    Insufficient { needed: u64, available: u64 },
}

fn transfer(amount: i64, balance: u64, frozen: bool) -> Result<u64, TransferError> {
    ensure!(amount > 0, TransferError::NonPositive);
    ensure!(!frozen, TransferError::Frozen);
    let needed = amount as u64;
    ensure!(
        needed <= balance,
        TransferError::Insufficient {
            needed,
            available: balance,
        }
    );
    Ok(balance - needed)
}

#[test]
fn part4_ensure_checks_in_order_and_reports_fields() {
    assert_eq!(transfer(30, 100, false), Ok(70));
    assert_eq!(transfer(0, 100, false), Err(TransferError::NonPositive));
    assert_eq!(
        transfer(0, 100, true),
        Err(TransferError::NonPositive),
        "the first failing check should win"
    );
    assert_eq!(transfer(5, 100, true), Err(TransferError::Frozen));
    assert_eq!(
        transfer(150, 100, false),
        Err(TransferError::Insufficient {
            needed: 150,
            available: 100,
        })
    );
}

fn check_once(calls: &Cell<u32>) -> Result<(), &'static str> {
    ensure!(
        {
            calls.set(calls.get() + 1);
            calls.get() > 0
        },
        "never"
    );
    Ok(())
}

#[test]
fn part4_ensure_evaluates_condition_exactly_once() {
    let calls = Cell::new(0);
    assert_eq!(check_once(&calls), Ok(()));
    assert_eq!(calls.get(), 1, "the condition must be evaluated once");
}

fn lazy_error(ok: bool, built: &Cell<u32>) -> Result<(), String> {
    ensure!(ok, {
        built.set(built.get() + 1);
        "failed".to_string()
    });
    Ok(())
}

#[test]
fn part4_ensure_builds_error_only_on_failure() {
    let built = Cell::new(0);
    assert_eq!(lazy_error(true, &built), Ok(()));
    assert_eq!(built.get(), 0, "the error must not be built when the check passes");

    assert_eq!(lazy_error(false, &built), Err("failed".to_string()));
    assert_eq!(built.get(), 1);
}

fn either(a: bool, b: bool) -> Result<(), &'static str> {
    ensure!(a || b, "neither");
    Ok(())
}

#[test]
fn part4_ensure_negation_respects_precedence() {
    assert_eq!(either(false, true), Ok(()), "the condition is `a || b`, not `!a || b`");
    assert_eq!(either(true, false), Ok(()));
    assert_eq!(either(false, false), Err("neither"));
}

fn parse_port(s: &str) -> Result<u16, String> {
    let Ok(n) = s.parse::<u32>() else {
        bail!(format!("not a number: {s}"));
    };
    ensure!(n > 0 && n <= 65_535, format!("port out of range: {n}"));
    Ok(n as u16)
}

#[test]
fn part4_bail_returns_converted_error() {
    assert_eq!(parse_port("8080"), Ok(8080));
    assert_eq!(parse_port("abc"), Err("not a number: abc".to_string()));
    assert_eq!(parse_port("70000"), Err("port out of range: 70000".to_string()));
    assert_eq!(parse_port("0"), Err("port out of range: 0".to_string()));
}

fn fail_fast() -> Result<(), String> {
    bail!("nope")
}

fn trailing(flag: bool) -> Result<(), &'static str> {
    ensure!(flag, "off",);
    Ok(())
}

#[test]
fn part4_bail_as_tail_and_trailing_commas() {
    assert_eq!(fail_fast(), Err("nope".to_string()), "bail! works as a tail expression");
    assert_eq!(trailing(true), Ok(()));
    assert_eq!(trailing(false), Err("off"), "trailing comma is accepted");
}

#[derive(Debug, PartialEq)]
enum AppError {
    Parse(ParseIntError),
    Limit(u64),
}

impl From<ParseIntError> for AppError {
    fn from(err: ParseIntError) -> Self {
        AppError::Parse(err)
    }
}

#[allow(clippy::question_mark, reason = "bail! is the behaviour under test")]
fn read_limit(s: &str) -> Result<u64, AppError> {
    let n = match s.parse::<u64>() {
        Ok(n) => n,
        Err(e) => bail!(e),
    };
    ensure!(n <= 100, AppError::Limit(n));
    Ok(n)
}

#[test]
fn part4_bail_uses_from_conversion() {
    assert_eq!(read_limit("50"), Ok(50));
    assert!(
        matches!(read_limit("x"), Err(AppError::Parse(_))),
        "bail!(e) should convert via From<ParseIntError>"
    );
    assert_eq!(read_limit("101"), Err(AppError::Limit(101)));
}

#[test]
fn part4_ensure_works_inside_closures() {
    let check = |n: i32| -> Result<i32, String> {
        ensure!(n >= 0, "negative".to_string());
        Ok(n * 2)
    };
    assert_eq!(check(2), Ok(4));
    assert_eq!(check(-1), Err("negative".to_string()), "return exits the closure");
}
