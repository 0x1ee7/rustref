//! Binary entry point. All behaviour lives in the library crate.

use std::io;
use std::process::ExitCode;

use ex18_deployment::cli::run;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    run(
        std::env::args_os(),
        |key: &str| std::env::var(key).ok(),
        &mut stdout.lock(),
        &mut stderr.lock(),
    )
}
