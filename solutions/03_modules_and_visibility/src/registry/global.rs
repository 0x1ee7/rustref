//! Process-wide singletons. Use sparingly: explicit context is preferred.

use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Registry;

static GLOBAL: LazyLock<Registry> = LazyLock::new(Registry::new);

static BOOT_ID: LazyLock<u64> = LazyLock::new(|| {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| u64::try_from(elapsed.as_nanos()).unwrap_or(0));
    (nanos ^ u64::from(std::process::id()).rotate_left(32)).max(1)
});

/// The process-wide plugin registry, created on first use.
#[must_use]
pub fn global() -> &'static Registry {
    &GLOBAL
}

/// A stable, nonzero id chosen once per process on first call.
/// Every later call returns the same value.
#[must_use]
pub fn boot_id() -> u64 {
    *BOOT_ID
}
