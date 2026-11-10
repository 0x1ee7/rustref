//! Part 4: command-line interface.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;

use crate::api::{AppState, router};
use crate::build_info;
use crate::config::{Config, ConfigError};
use crate::server::serve;

/// `rustref-service` command line.
#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "rustref-service", about = "Container-ready HTTP service")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands.
#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    /// Run the HTTP server; `--dry-run` validates configuration and prints the plan without binding.
    Serve {
        /// Print what would happen and exit 0 without binding a socket.
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate configuration from the environment and exit.
    CheckConfig,
    /// Print the version and exit.
    Version,
}

/// Runs the CLI with arguments `args` (first item is the program name), configuration
/// read through `lookup`, and output written to `out` / `err`. Returns the exit code.
///
/// Exit codes: `0` success; `2` usage error (from clap) or invalid configuration;
/// `1` runtime failure (e.g. cannot bind). Clap's `--help` / `--version` go to `out`, exit 0.
pub fn run<I, T>(
    args: I,
    lookup: impl Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            let text = error.to_string();
            return if error.use_stderr() {
                let _ = err.write_all(text.as_bytes());
                ExitCode::from(2)
            } else {
                let _ = out.write_all(text.as_bytes());
                ExitCode::SUCCESS
            };
        }
    };

    match cli.command {
        Command::Version => {
            let _ = writeln!(out, "rustref-service {}", build_info::version());
            ExitCode::SUCCESS
        }
        Command::CheckConfig => match Config::from_env(&lookup) {
            Ok(_) => {
                let _ = writeln!(out, "configuration ok");
                ExitCode::SUCCESS
            }
            Err(error) => report_config_error(&error, err),
        },
        Command::Serve { dry_run } => match Config::from_env(&lookup) {
            Err(error) => report_config_error(&error, err),
            Ok(config) if dry_run => {
                let _ = writeln!(
                    out,
                    "dry run: {} would listen on {}",
                    config.app_name, config.listen_addr
                );
                ExitCode::SUCCESS
            }
            Ok(config) => serve_forever(config, out, err),
        },
    }
}

/// Prints every issue on its own `error:` line and returns the usage/config exit code.
fn report_config_error(error: &ConfigError, err: &mut dyn Write) -> ExitCode {
    for issue in error.issues() {
        let _ = writeln!(err, "error: {issue}");
    }
    ExitCode::from(2)
}

/// Binds the listen address and serves until SIGINT or SIGTERM.
fn serve_forever(config: Config, out: &mut dyn Write, err: &mut dyn Write) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = writeln!(err, "error: cannot start runtime: {error}");
            return ExitCode::from(1);
        }
    };
    runtime.block_on(async {
        let listener = match TcpListener::bind(config.listen_addr).await {
            Ok(listener) => listener,
            Err(error) => {
                let _ = writeln!(err, "error: cannot bind {}: {error}", config.listen_addr);
                return ExitCode::from(1);
            }
        };
        let _ = writeln!(out, "listening on {}", config.listen_addr);

        let app = with_request_timeout(
            router(AppState::new(config.app_name.clone())),
            config.request_timeout,
        );
        match serve(listener, app, shutdown_signal()).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(err, "error: server failed: {error}");
                ExitCode::from(1)
            }
        }
    })
}

/// Answers `503` for any request that takes longer than `limit`.
fn with_request_timeout(app: Router, limit: Duration) -> Router {
    app.layer(middleware::from_fn(
        move |request: Request, next: Next| async move {
            match tokio::time::timeout(limit, next.run(request)).await {
                Ok(response) => response,
                Err(_elapsed) => {
                    (StatusCode::SERVICE_UNAVAILABLE, "request timed out").into_response()
                }
            }
        },
    ))
}

/// Resolves on Ctrl-C or, on Unix, SIGTERM (what orchestrators send).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
