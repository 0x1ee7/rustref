//! Part 4 — JSON output through `fmt().json()` and a custom `MakeWriter`.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use tracing::{Level, Subscriber};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt;

/// In-memory sink that implements [`MakeWriter`]. Clones share one buffer.
#[derive(Debug, Clone, Default)]
pub struct CaptureWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl CaptureWriter {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything written so far across all clones, decoded as UTF-8 (lossy).
    pub fn contents(&self) -> String {
        String::from_utf8_lossy(&self.buf.lock().expect("buffer lock")).into_owned()
    }
}

impl Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.lock().expect("buffer lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// A subscriber that writes one JSON object per event to `writer`.
pub fn json_subscriber<W>(writer: W) -> impl Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // `fmt()` defaults to INFO; this subscriber must not filter anything.
    fmt()
        .json()
        .with_ansi(false)
        .with_max_level(Level::TRACE)
        .with_writer(writer)
        .finish()
}
