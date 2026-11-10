//! Part 4: command-line interface.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
    _args: I,
    _lookup: impl Fn(&str) -> Option<String>,
    _out: &mut dyn Write,
    _err: &mut dyn Write,
) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    todo!("Part 4: run")
}
