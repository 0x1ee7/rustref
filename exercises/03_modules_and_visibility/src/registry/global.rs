//! Process-wide singletons. Use sparingly: explicit context is preferred.

use super::Registry;

/// The process-wide plugin registry, created on first use.
#[must_use]
pub fn global() -> &'static Registry {
    todo!()
}

/// A stable, nonzero id chosen once per process on first call.
/// Every later call returns the same value.
#[must_use]
pub fn boot_id() -> u64 {
    todo!()
}
