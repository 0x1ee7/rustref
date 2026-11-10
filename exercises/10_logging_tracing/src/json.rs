//! Part 4 — JSON output through `fmt().json()` and a custom `MakeWriter`.

use std::io;

use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::Registry;

/// In-memory sink that implements [`MakeWriter`]. Clones share one buffer, so
/// a subscriber can own one clone while the test keeps another to read back.
#[derive(Debug, Clone, Default)]
pub struct CaptureWriter {}

impl CaptureWriter {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        todo!()
    }

    /// Everything written so far across all clones, decoded as UTF-8 (lossy).
    pub fn contents(&self) -> String {
        todo!()
    }
}

impl io::Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = buf;
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl<'a> MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriter;

    fn make_writer(&'a self) -> Self::Writer {
        todo!()
    }
}

/// A subscriber that writes one JSON object per event to `writer`.
///
/// Behaviour (Part 4): no level filtering (every level is emitted); output via
/// `tracing_subscriber::fmt().json()` with the current span's fields under a
/// `span` key and the whole span stack under `spans`, outermost first. Each
/// event is exactly one line, with no ANSI escapes.
#[allow(unreachable_code)]
pub fn json_subscriber<W>(writer: W) -> impl Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let _ = writer;
    todo!();
    Registry::default()
}
