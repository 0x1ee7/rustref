mod common;

use common::Recorder;
use ex10_logging_tracing::filter::{build_filter, FilterError};
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn emit_all() {
    debug!(target: "shop::db", "db debug");
    info!(target: "shop::db", "db info");
    debug!(target: "shop::db::pool", "pool debug");
    trace!(target: "shop::db::pool", "pool trace");
    debug!(target: "shop::dbx", "sibling debug");
    info!(target: "shop::http", "http info");
    warn!(target: "shop::http", "http warn");
    error!(target: "shop", "root error");
}

/// Installs the filter from `build_filter` and returns the messages that got through.
fn run(default: &str, overrides: &[(&str, &str)]) -> Vec<String> {
    let filter = build_filter(default, overrides).expect("filter should be valid");
    let rec = Recorder::new();
    let sub = Registry::default().with(filter).with(rec.clone());
    tracing::subscriber::with_default(sub, emit_all);
    rec.snapshot().events.iter().map(|e| e.message().to_owned()).collect()
}

fn has(messages: &[String], msg: &str) -> bool {
    messages.iter().any(|m| m == msg)
}

#[test]
fn part3_default_level_hides_info_for_unlisted_targets() {
    let got = run("warn", &[("shop::db", "debug")]);
    assert!(has(&got, "http warn"), "WARN should pass the default warn level, got {got:?}");
    assert!(!has(&got, "http info"), "INFO must be hidden by the default warn level, got {got:?}");
}

#[test]
fn part3_override_enables_debug_and_info_for_listed_target() {
    let got = run("warn", &[("shop::db", "debug")]);
    assert!(has(&got, "db debug"), "override debug should enable db DEBUG, got {got:?}");
    assert!(has(&got, "db info"), "override debug should enable db INFO, got {got:?}");
}

#[test]
fn part3_override_applies_to_child_module_paths() {
    let got = run("warn", &[("shop::db", "debug")]);
    assert!(has(&got, "pool debug"), "shop::db::pool should inherit the shop::db override, got {got:?}");
    assert!(!has(&got, "pool trace"), "override level is debug, TRACE must stay hidden, got {got:?}");
}

#[test]
fn part3_override_also_matches_sibling_sharing_a_string_prefix() {
    // EnvFilter matches targets by string prefix, not by path segment.
    let got = run("warn", &[("shop::db", "debug")]);
    assert!(has(&got, "sibling debug"), "EnvFilter matches by string prefix, so shop::dbx also gets the shop::db override, got {got:?}");
}

#[test]
fn part3_default_still_passes_root_errors() {
    let got = run("warn", &[("shop::db", "debug")]);
    assert!(has(&got, "root error"), "ERROR from shop must pass the warn default, got {got:?}");
}

#[test]
fn part3_most_specific_directive_wins() {
    let got = run("info", &[("shop", "error"), ("shop::db", "debug")]);
    assert!(has(&got, "db debug"), "shop::db=debug should beat shop=error, got {got:?}");
    assert!(!has(&got, "http info"), "shop=error should silence shop::http INFO, got {got:?}");
    assert!(!has(&got, "http warn"), "shop=error should silence shop::http WARN, got {got:?}");
}

#[test]
fn part3_off_silences_one_target() {
    let got = run("debug", &[("shop::http", "off")]);
    assert!(!has(&got, "http warn"), "off should silence shop::http, got {got:?}");
    assert!(!has(&got, "http info"), "off should silence shop::http, got {got:?}");
    assert!(has(&got, "db debug"), "default debug should still reach shop::db, got {got:?}");
}

#[test]
fn part3_level_names_are_case_insensitive() {
    let got = run("WARN", &[("shop::db", "Debug")]);
    assert!(has(&got, "db debug"), "'Debug' should parse as debug, got {got:?}");
    assert!(!has(&got, "http info"), "'WARN' default should hide INFO, got {got:?}");
}

#[test]
fn part3_empty_overrides_leave_only_default_level() {
    let got = run("error", &[]);
    assert_eq!(got, vec!["root error".to_owned()], "only ERROR passes an error default");
}

#[test]
fn part3_unknown_override_level_is_rejected() {
    let err = build_filter("info", &[("shop::db", "loud")]).unwrap_err();
    assert_eq!(
        err,
        FilterError::UnknownLevel { target: "shop::db".to_owned(), level: "loud".to_owned() },
        "bad override level should name the target and level"
    );
}

#[test]
fn part3_unknown_default_level_is_rejected_with_empty_target() {
    let err = build_filter("loud", &[]).unwrap_err();
    assert_eq!(
        err,
        FilterError::UnknownLevel { target: String::new(), level: "loud".to_owned() },
        "bad default level should use an empty target"
    );
}

#[test]
fn part3_empty_target_is_rejected() {
    let err = build_filter("info", &[("", "debug")]).unwrap_err();
    assert_eq!(err, FilterError::EmptyTarget, "empty override target must be rejected");
}

#[test]
fn part3_unknown_level_message_names_the_level() {
    let err = build_filter("info", &[("shop::db", "loud")]).unwrap_err();
    assert!(err.to_string().contains("loud"), "error message should mention the bad level, got {err}");
}

#[test]
fn part3_empty_target_message_is_readable() {
    let err = build_filter("info", &[("", "debug")]).unwrap_err();
    assert_eq!(err, FilterError::EmptyTarget, "an empty override target must be EmptyTarget, got {err:?}");
    assert!(err.to_string().contains("target"), "message should mention the target, got {err}");
}
