//! Part 4: CLI: clap derive surface, testable `run` entry point, exit codes.

use std::process::ExitCode;

use ex18_deployment::cli::run;

const TOKEN: &str = "0123456789abcdef";
const DB: &str = "postgres://app:hunter2@db:5432/orders";

/// Runs the CLI with an environment map. Returns (exit code, stdout, stderr).
fn invoke(args: &[&str], env: &[(&str, &str)]) -> (ExitCode, String, String) {
    let lookup = {
        let map: std::collections::HashMap<String, String> =
            env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |key: &str| map.get(key).cloned()
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(args.iter().copied(), lookup, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).expect("stdout is UTF-8"),
        String::from_utf8(err).expect("stderr is UTF-8"),
    )
}

fn valid_env() -> Vec<(&'static str, &'static str)> {
    vec![("API_TOKEN", TOKEN), ("DATABASE_URL", DB)]
}

// --- clap surface, observed through `run` ----------------------------------

#[test]
fn part4_parses_serve_dry_run() {
    let (code, out, _) = invoke(&["rustref-service", "serve", "--dry-run"], &valid_env());
    assert_eq!(code, ExitCode::SUCCESS, "serve --dry-run is a valid invocation");
    assert!(out.starts_with("dry run:"), "--dry-run selects the dry-run plan, got {out:?}");
}

#[test]
fn part4_parses_check_config_and_version() {
    let (code, out, _) = invoke(&["rustref-service", "check-config"], &valid_env());
    assert_eq!(code, ExitCode::SUCCESS, "check-config with valid config exits 0");
    assert_eq!(out, "configuration ok\n");
    let (code, out, _) = invoke(&["rustref-service", "version"], &[]);
    assert_eq!(code, ExitCode::SUCCESS, "version exits 0");
    assert!(out.starts_with("rustref-service "), "version prints the app name, got {out:?}");
}

#[test]
fn part4_unknown_flag_is_a_usage_error() {
    let (code, out, err) = invoke(&["rustref-service", "serve", "--bogus"], &[]);
    assert_eq!(code, ExitCode::from(2), "unknown flags exit 2");
    assert!(out.is_empty() && !err.is_empty(), "usage text goes to stderr");
}

#[test]
fn part4_missing_subcommand_is_an_error() {
    let (code, _, _) = invoke(&["rustref-service"], &[]);
    assert_eq!(code, ExitCode::from(2), "running without a subcommand exits 2");
}

#[test]
fn part4_dry_run_flag_rejects_a_value() {
    let (code, _, err) = invoke(&["rustref-service", "serve", "--dry-run=yes"], &valid_env());
    assert_eq!(code, ExitCode::from(2), "--dry-run is a switch and takes no value");
    assert!(!err.is_empty(), "the rejection message goes to stderr");
}

// --- run: exit codes and output -------------------------------------------

#[test]
fn part4_help_goes_to_stdout_and_exits_zero() {
    let (code, out, err) = invoke(&["rustref-service", "--help"], &[]);
    assert_eq!(code, ExitCode::SUCCESS, "--help exits 0");
    assert!(out.contains("Usage"), "help text goes to stdout, got {out:?}");
    assert!(err.is_empty(), "nothing on stderr for --help, got {err:?}");
}

#[test]
fn part4_usage_error_exits_two_with_message_on_stderr() {
    let (code, out, err) = invoke(&["rustref-service", "frobnicate"], &[]);
    assert_eq!(code, ExitCode::from(2), "usage errors exit 2");
    assert!(!err.is_empty(), "usage error text goes to stderr");
    assert!(out.is_empty(), "nothing on stdout for usage errors, got {out:?}");
}

#[test]
fn part4_version_prints_crate_version_and_needs_no_config() {
    let (code, out, err) = invoke(&["rustref-service", "version"], &[]);
    assert_eq!(code, ExitCode::SUCCESS, "version exits 0 with empty environment");
    assert!(err.is_empty(), "no error output, got {err:?}");
    assert!(out.starts_with("rustref-service "), "output starts with the app name, got {out:?}");
    assert!(
        out.contains(env!("CARGO_PKG_VERSION")),
        "output must contain the crate version, got {out:?}"
    );
    assert!(out.ends_with('\n'), "output is one line terminated by newline");
}

#[test]
fn part4_check_config_ok_prints_configuration_ok() {
    let (code, out, err) = invoke(&["rustref-service", "check-config"], &valid_env());
    assert_eq!(code, ExitCode::SUCCESS, "valid config exits 0");
    assert_eq!(out, "configuration ok\n", "exact success line");
    assert!(err.is_empty(), "no stderr on success, got {err:?}");
}

#[test]
fn part4_check_config_reports_each_issue_on_stderr_and_exits_two() {
    let (code, out, err) = invoke(&["rustref-service", "check-config"], &[]);
    assert_eq!(code, ExitCode::from(2), "invalid config exits 2");
    assert!(out.is_empty(), "no stdout on config errors, got {out:?}");
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(lines.len(), 2, "one `error:` line per bad variable, got {err:?}");
    assert!(lines[0].starts_with("error: API_TOKEN: "), "first line: {:?}", lines[0]);
    assert!(lines[1].starts_with("error: DATABASE_URL: "), "second line: {:?}", lines[1]);
}

#[test]
fn part4_dry_run_prints_plan_with_default_listen_address() {
    let (code, out, err) = invoke(&["rustref-service", "serve", "--dry-run"], &valid_env());
    assert_eq!(code, ExitCode::SUCCESS, "dry run exits 0");
    assert_eq!(out, "dry run: rustref-service would listen on 0.0.0.0:8080\n");
    assert!(err.is_empty(), "no stderr on dry run, got {err:?}");
}

#[test]
fn part4_dry_run_does_not_bind_the_socket() {
    // Hold the address open. A real serve would fail with "address in use";
    // a dry run must succeed without touching it.
    let held = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = held.local_addr().expect("addr").to_string();
    let env = [("API_TOKEN", TOKEN), ("DATABASE_URL", DB), ("LISTEN_ADDR", addr.as_str())];
    let (code, out, _) = invoke(&["rustref-service", "serve", "--dry-run"], &env);
    assert_eq!(code, ExitCode::SUCCESS, "dry run must not try to bind {addr}");
    assert_eq!(out, format!("dry run: rustref-service would listen on {addr}\n"));
}

#[test]
fn part4_dry_run_with_invalid_config_exits_two() {
    let (code, out, err) = invoke(&["rustref-service", "serve", "--dry-run"], &[("LOG_LEVEL", "loud")]);
    assert_eq!(code, ExitCode::from(2), "invalid config exits 2 for dry run too");
    assert!(out.is_empty(), "no plan printed for invalid config, got {out:?}");
    assert!(err.contains("error: LOG_LEVEL: "), "stderr names the bad variable, got {err:?}");
    assert!(err.contains("error: API_TOKEN: "), "every issue is reported, got {err:?}");
}

#[test]
fn part4_config_errors_never_echo_secrets() {
    let (_, _, err) = invoke(
        &["rustref-service", "check-config"],
        &[("API_TOKEN", "short"), ("DATABASE_URL", "mysql://app:pw-very-secret@db/x")],
    );
    assert!(!err.contains("pw-very-secret"), "stderr leaked the database password: {err}");
    assert!(!err.contains("short\n") && !err.contains(": short"), "stderr leaked the token: {err}");
}
