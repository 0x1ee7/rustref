//! Part 6: `assert_matches!`, a pattern-based assertion that can also extract bindings.

use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

use ex20_macros::assert_matches;

#[derive(Debug, PartialEq)]
enum Event {
    Login { user: String },
    Logout,
    Error(u16),
}

/// Runs `f`, which must panic, and returns the panic message.
fn panic_message(f: impl FnOnce()) -> String {
    let payload = panic::catch_unwind(AssertUnwindSafe(f))
        .expect_err("expected the assertion to panic");
    if let Some(s) = payload.downcast_ref::<String>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn next_event(calls: &Cell<u32>) -> Event {
    calls.set(calls.get() + 1);
    Event::Logout
}

#[test]
fn part6_arrow_form_returns_bindings() {
    let user: String = assert_matches!(
        Event::Login { user: "ada".into() },
        Event::Login { user } => user,
    );
    assert_eq!(user, "ada", "the arrow form returns the pattern's binding");

    let code: u16 = assert_matches!(Event::Error(503), Event::Error(c) if c >= 500 => c);
    assert_eq!(code, 503, "guards filter, then the output is returned");
}

#[test]
fn part6_unit_form_passes_with_guard_and_trailing_comma() {
    assert_matches!(Event::Logout, Event::Logout);
    assert_matches!(Event::Error(500), Event::Error(c) if c >= 500);
    assert_matches!(Event::Logout, Event::Logout,);
}

#[test]
fn part6_or_patterns_and_single_evaluation() {
    assert_matches!(Event::Error(404), Event::Error(400 | 404));

    let calls = Cell::new(0);
    assert_matches!(next_event(&calls), Event::Logout);
    assert_eq!(calls.get(), 1, "the value expression must be evaluated once");
}

#[test]
fn part6_failure_message_shows_value_and_pattern() {
    let msg = panic_message(|| assert_matches!(Event::Logout, Event::Error(_)));
    assert!(msg.contains("Logout"), "message should show the value, got {msg:?}");
    assert!(
        msg.contains("Error(_)"),
        "message should show the pattern text, got {msg:?}"
    );
}

#[test]
fn part6_custom_message_prefixes_the_failure() {
    let msg = panic_message(|| {
        assert_matches!(Event::Logout, Event::Error(_), "expected an error for {}", "login")
    });
    assert!(
        msg.contains("expected an error for login"),
        "the custom message should prefix the failure, got {msg:?}"
    );
    assert!(msg.contains("Logout"), "the value should still appear, got {msg:?}");
}

#[test]
fn part6_guard_and_arrow_failures_panic() {
    let guard_msg = panic_message(|| assert_matches!(Event::Error(200), Event::Error(c) if c >= 500));
    assert!(
        guard_msg.contains("Error(200)"),
        "a failing guard should panic with the value, got {guard_msg:?}"
    );

    let arrow_msg = panic_message(|| {
        let _: u16 = assert_matches!(Event::Logout, Event::Error(c) => c);
    });
    assert!(arrow_msg.contains("Logout"), "got {arrow_msg:?}");
}
