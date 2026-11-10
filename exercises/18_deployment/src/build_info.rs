//! Part 5: build metadata.
//!
//! `build.rs` emits `BUILD_GIT_SHA` as a compile-time env var. Read it with
//! `option_env!` so a build without it still compiles.

/// Crate version from Cargo (`CARGO_PKG_VERSION`).
pub fn version() -> &'static str {
    todo!("Part 5: version")
}

/// Git commit the binary was built from: 40 lowercase hex characters,
/// or `"unknown"` when `build.rs` could not determine it.
pub fn git_sha() -> &'static str {
    todo!("Part 5: git_sha")
}
