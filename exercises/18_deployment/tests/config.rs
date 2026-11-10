//! Part 1: 12-factor configuration: typed parsing, aggregated errors, secret redaction.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::time::Duration;

use ex18_deployment::config::{
    Config, LogLevel, Secret, parse_bool, parse_duration, parse_list,
};

const TOKEN: &str = "0123456789abcdef"; // exactly 16 characters
const DB: &str = "postgres://app:hunter2@db:5432/orders";

/// Builds a lookup closure over a fixed set of variables.
fn env_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |key| map.get(key).cloned()
}

/// Minimal valid environment: both required variables set, nothing else.
fn required() -> Vec<(&'static str, &'static str)> {
    vec![("API_TOKEN", TOKEN), ("DATABASE_URL", DB)]
}

fn with(mut base: Vec<(&'static str, &'static str)>, extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
    base.extend_from_slice(extra);
    base
}

fn vars_of(err: &ex18_deployment::config::ConfigError) -> Vec<&'static str> {
    err.issues().iter().map(|i| i.var).collect()
}

// --- secrets ---------------------------------------------------------------

#[test]
fn part1_secret_debug_is_redacted() {
    let s = Secret::new(TOKEN.to_string());
    let shown = format!("{s:?}");
    assert!(!shown.contains(TOKEN), "Debug must not reveal the secret, got {shown:?}");
    assert!(shown.contains("REDACTED"), "Debug must say REDACTED, got {shown:?}");
}

#[test]
fn part1_secret_display_is_redacted() {
    let s = Secret::new(TOKEN.to_string());
    let shown = format!("{s}");
    assert!(!shown.contains(TOKEN), "Display must not reveal the secret, got {shown:?}");
    assert!(shown.contains("REDACTED"), "Display must say REDACTED, got {shown:?}");
}

#[test]
fn part1_secret_expose_returns_the_value() {
    let s = Secret::new(TOKEN.to_string());
    assert_eq!(s.expose(), TOKEN, "expose() must return the wrapped value");
}

#[test]
fn part1_config_debug_never_shows_secrets() {
    let cfg = Config::from_env(env_from(&required())).expect("valid config");
    let shown = format!("{cfg:?}");
    assert!(!shown.contains(TOKEN), "Config Debug leaked API_TOKEN: {shown}");
    assert!(!shown.contains("hunter2"), "Config Debug leaked the database password: {shown}");
    assert!(shown.contains("0.0.0.0:8080"), "non-secret fields must stay visible: {shown}");
}

// --- value parsers ---------------------------------------------------------

#[test]
fn part1_parse_bool_accepts_truthy_and_falsy_spellings() {
    for t in ["1", "true", "TRUE", "yes", "On", "  off  ", "no", "0", "False"] {
        assert!(parse_bool(t).is_ok(), "parse_bool({t:?}) must be accepted");
    }
    assert_eq!(parse_bool("yes"), Ok(true), "yes is true");
    assert_eq!(parse_bool(" On "), Ok(true), "surrounding whitespace is ignored");
    assert_eq!(parse_bool("off"), Ok(false), "off is false");
    assert_eq!(parse_bool("0"), Ok(false), "0 is false");
}

#[test]
fn part1_parse_bool_rejects_other_text() {
    for t in ["", "maybe", "2", "truthy", "y"] {
        assert!(parse_bool(t).is_err(), "parse_bool({t:?}) must be rejected");
    }
}

#[test]
fn part1_parse_duration_understands_units_and_bare_seconds() {
    assert_eq!(parse_duration("500ms"), Ok(Duration::from_millis(500)));
    assert_eq!(parse_duration("30s"), Ok(Duration::from_secs(30)));
    assert_eq!(parse_duration("5m"), Ok(Duration::from_secs(300)));
    assert_eq!(parse_duration("1h"), Ok(Duration::from_secs(3600)));
    assert_eq!(parse_duration("2"), Ok(Duration::from_secs(2)), "bare number means seconds");
    assert_eq!(parse_duration("1.5s"), Ok(Duration::from_millis(1500)), "fractional seconds");
}

#[test]
fn part1_parse_duration_rejects_bad_input() {
    for t in ["", "   ", "-1s", "5 minutes", "10d", "abc", "s"] {
        assert!(parse_duration(t).is_err(), "parse_duration({t:?}) must be rejected");
    }
}

#[test]
fn part1_parse_list_trims_and_drops_empty_items() {
    assert_eq!(
        parse_list(" a.example , ,b.example,, "),
        vec!["a.example".to_string(), "b.example".to_string()],
        "items are trimmed and empty items dropped"
    );
    assert!(parse_list("").is_empty(), "empty input gives an empty list");
    assert!(parse_list(" , ,").is_empty(), "only separators gives an empty list");
}

#[test]
fn part1_log_level_parses_case_insensitively() {
    assert_eq!("INFO".parse::<LogLevel>(), Ok(LogLevel::Info));
    assert_eq!("warn".parse::<LogLevel>(), Ok(LogLevel::Warn));
    assert_eq!("Trace".parse::<LogLevel>(), Ok(LogLevel::Trace));
    assert_eq!(LogLevel::Error.as_str(), "error", "as_str is lowercase");
}

#[test]
fn part1_log_level_rejects_unknown_names() {
    assert!("verbose".parse::<LogLevel>().is_err(), "unknown level must be rejected");
    assert!("".parse::<LogLevel>().is_err(), "empty level must be rejected");
}

// --- from_env: defaults and typed values ----------------------------------

#[test]
fn part1_from_env_minimal_environment_uses_defaults() {
    let cfg = Config::from_env(env_from(&required())).expect("required vars are enough");
    assert_eq!(cfg.listen_addr, "0.0.0.0:8080".parse::<SocketAddr>().unwrap(), "default LISTEN_ADDR");
    assert_eq!(cfg.app_name, "rustref-service", "default APP_NAME");
    assert_eq!(cfg.request_timeout, Duration::from_secs(30), "default REQUEST_TIMEOUT");
    assert_eq!(cfg.max_connections, NonZeroU32::new(256).unwrap(), "default MAX_CONNECTIONS");
    assert!(cfg.allowed_hosts.is_empty(), "default ALLOWED_HOSTS is empty");
    assert!(!cfg.debug, "default DEBUG is false");
    assert_eq!(cfg.log_level, LogLevel::Info, "default LOG_LEVEL is info");
    assert_eq!(cfg.api_token.expose(), TOKEN, "API_TOKEN is kept verbatim");
    assert_eq!(cfg.database_url.expose(), DB, "DATABASE_URL is kept verbatim");
}

#[test]
fn part1_from_env_reads_every_optional_value() {
    let vars = with(
        required(),
        &[
            ("LISTEN_ADDR", "127.0.0.1:9000"),
            ("APP_NAME", "orders"),
            ("REQUEST_TIMEOUT", "2m"),
            ("MAX_CONNECTIONS", "64"),
            ("ALLOWED_HOSTS", "a.example, b.example"),
            ("DEBUG", "yes"),
            ("LOG_LEVEL", "DEBUG"),
        ],
    );
    let cfg = Config::from_env(env_from(&vars)).expect("all values are valid");
    assert_eq!(cfg.listen_addr, "127.0.0.1:9000".parse::<SocketAddr>().unwrap());
    assert_eq!(cfg.app_name, "orders");
    assert_eq!(cfg.request_timeout, Duration::from_secs(120));
    assert_eq!(cfg.max_connections.get(), 64);
    assert_eq!(cfg.allowed_hosts, vec!["a.example".to_string(), "b.example".to_string()]);
    assert!(cfg.debug, "DEBUG=yes is true");
    assert_eq!(cfg.log_level, LogLevel::Debug);
}

#[test]
fn part1_from_env_blank_values_count_as_unset() {
    let vars = with(required(), &[("LISTEN_ADDR", "   "), ("APP_NAME", ""), ("DEBUG", "\t")]);
    let cfg = Config::from_env(env_from(&vars)).expect("blank optionals fall back to defaults");
    assert_eq!(cfg.listen_addr, "0.0.0.0:8080".parse::<SocketAddr>().unwrap());
    assert_eq!(cfg.app_name, "rustref-service");
    assert!(!cfg.debug);
}

#[test]
fn part1_from_env_blank_required_value_is_reported_missing() {
    let vars = vec![("API_TOKEN", "   "), ("DATABASE_URL", DB)];
    let err = Config::from_env(env_from(&vars)).expect_err("blank API_TOKEN is unset");
    assert_eq!(vars_of(&err), vec!["API_TOKEN"], "exactly the blank required var is reported");
}

#[test]
fn part1_from_env_reports_every_missing_required_variable() {
    let err = Config::from_env(env_from(&[])).expect_err("nothing set");
    assert_eq!(
        vars_of(&err),
        vec!["API_TOKEN", "DATABASE_URL"],
        "both missing required variables must be reported in one error"
    );
}

#[test]
fn part1_from_env_collects_every_problem_not_just_the_first() {
    let vars = with(
        required(),
        &[
            ("LISTEN_ADDR", "nope"),
            ("REQUEST_TIMEOUT", "0s"),
            ("MAX_CONNECTIONS", "0"),
            ("DEBUG", "maybe"),
            ("LOG_LEVEL", "verbose"),
        ],
    );
    let err = Config::from_env(env_from(&vars)).expect_err("five bad values");
    let mut got = vars_of(&err);
    got.sort_unstable();
    assert_eq!(
        got,
        vec!["DEBUG", "LISTEN_ADDR", "LOG_LEVEL", "MAX_CONNECTIONS", "REQUEST_TIMEOUT"],
        "each bad variable gets exactly one issue"
    );
}

#[test]
fn part1_from_env_error_display_lists_all_issues() {
    let err = Config::from_env(env_from(&[("PORT_HINT", "x")])).expect_err("required vars missing");
    let msg = err.to_string();
    assert!(
        msg.starts_with("invalid configuration: "),
        "Display must start with `invalid configuration: `, got {msg:?}"
    );
    assert!(msg.contains("API_TOKEN: "), "message must name API_TOKEN, got {msg:?}");
    assert!(msg.contains("DATABASE_URL: "), "message must name DATABASE_URL, got {msg:?}");
    assert!(msg.contains("; "), "issues are joined with `; `, got {msg:?}");
}

#[test]
fn part1_from_env_issue_renders_as_var_colon_problem() {
    let vars = with(required(), &[("REQUEST_TIMEOUT", "soon")]);
    let err = Config::from_env(env_from(&vars)).expect_err("bad duration");
    let issue = err.issues().first().expect("one issue");
    assert_eq!(issue.var, "REQUEST_TIMEOUT");
    assert!(
        format!("{issue}").starts_with("REQUEST_TIMEOUT: "),
        "Issue Display must be `VAR: problem`, got {issue}"
    );
}

// --- validation rules ------------------------------------------------------

#[test]
fn part1_short_api_token_is_rejected_without_echoing_it() {
    let vars = vec![("API_TOKEN", "tiny-secret"), ("DATABASE_URL", DB)];
    let err = Config::from_env(env_from(&vars)).expect_err("token under 16 chars");
    assert_eq!(vars_of(&err), vec!["API_TOKEN"]);
    assert!(
        !err.to_string().contains("tiny-secret"),
        "error message must never echo the token: {err}"
    );
}

#[test]
fn part1_database_url_scheme_is_checked_and_password_not_echoed() {
    let vars = vec![("API_TOKEN", TOKEN), ("DATABASE_URL", "mysql://app:pw-very-secret@db/x")];
    let err = Config::from_env(env_from(&vars)).expect_err("mysql is not supported");
    assert_eq!(vars_of(&err), vec!["DATABASE_URL"]);
    assert!(
        !err.to_string().contains("pw-very-secret"),
        "error message must never echo the database password: {err}"
    );
}

#[test]
fn part1_database_url_accepts_postgres_postgresql_and_sqlite() {
    for url in ["postgres://h/db", "postgresql://h/db", "sqlite:///tmp/app.db"] {
        let vars = vec![("API_TOKEN", TOKEN), ("DATABASE_URL", url)];
        assert!(Config::from_env(env_from(&vars)).is_ok(), "{url} must be accepted");
    }
}

#[test]
fn part1_zero_request_timeout_is_rejected() {
    let vars = with(required(), &[("REQUEST_TIMEOUT", "0")]);
    let err = Config::from_env(env_from(&vars)).expect_err("zero timeout");
    assert_eq!(vars_of(&err), vec!["REQUEST_TIMEOUT"]);
}

#[test]
fn part1_listen_addr_port_out_of_range_is_rejected() {
    let vars = with(required(), &[("LISTEN_ADDR", "0.0.0.0:70000")]);
    let err = Config::from_env(env_from(&vars)).expect_err("port 70000 does not fit in u16");
    assert_eq!(vars_of(&err), vec!["LISTEN_ADDR"]);
}

#[test]
fn part1_config_derives_value_equality() {
    let a = Config::from_env(env_from(&required())).unwrap();
    let b = Config::from_env(env_from(&required())).unwrap();
    assert_eq!(a, b, "identical environments produce equal configs");
}
