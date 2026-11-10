//! Part 5: build metadata baked in at compile time.

/// Crate version from `Cargo.toml`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Git commit the binary was built from, or `unknown`. Set by `build.rs`.
pub fn git_sha() -> &'static str {
    env!("BUILD_GIT_SHA")
}
