//! Helpers shared by the integration tests. Each test file pulls this in with `mod support;`.
#![allow(dead_code)]

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ex15_async_tokio::resilience::Clock;
use tokio::time::Instant;

/// An in-memory log sink that tests can read back.
#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<Vec<u8>>>);

impl Captured {
    /// Everything written so far, as UTF-8 text.
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).expect("log output is UTF-8")
    }
}

impl io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Installs a thread-local `ERROR`-level subscriber that writes into `captured`.
///
/// The subscriber lives only as long as the returned guard, so tests never touch global state.
pub fn capture_errors(captured: Captured) -> tracing::subscriber::DefaultGuard {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_ansi(false)
        .without_time()
        .with_writer(move || captured.clone())
        .finish();
    tracing::subscriber::set_default(subscriber)
}

/// A clock that only moves when a test says so. Clones share the same time.
#[derive(Clone)]
pub struct ManualClock(Arc<Mutex<Instant>>);

impl Default for ManualClock {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }
}

impl ManualClock {
    /// Moves the clock forward by `by`.
    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap() += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}
