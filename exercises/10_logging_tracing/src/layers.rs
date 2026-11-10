//! Part 5 — custom `Layer`s: event counting, span timings, and secret redaction
//! through a `Visit` implementation.

use std::time::Duration;

use tracing::span::Id;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Field names whose values must never reach an output sink.
pub const SECRET_FIELDS: &[&str] = &[
    "password",
    "token",
    "secret",
    "api_key",
    "authorization",
    "card_token",
];

/// Event totals per level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelCounts {
    pub trace: u64,
    pub debug: u64,
    pub info: u64,
    pub warn: u64,
    pub error: u64,
}

/// Layer that counts events per level. Spans are not counted.
///
/// Clones share the same counters.
#[derive(Debug, Clone, Default)]
pub struct EventCounter {}

impl EventCounter {
    /// Creates a counter with all totals at zero.
    pub fn new() -> Self {
        todo!()
    }

    /// Point-in-time copy of the totals.
    pub fn snapshot(&self) -> LevelCounts {
        todo!()
    }
}

impl<S: Subscriber> Layer<S> for EventCounter {
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {
        todo!()
    }
}

/// Timing of one closed span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanTiming {
    /// The span's name.
    pub name: String,
    /// Name of the enclosing span, if any.
    pub parent: Option<String>,
    /// Time the span was *entered* (between `on_enter` and `on_exit`, summed
    /// over every entry). Time spent alive but not entered is excluded.
    pub busy: Duration,
}

/// Layer that records a [`SpanTiming`] for every span when it closes.
///
/// Clones share the same record list.
#[derive(Debug, Clone, Default)]
pub struct SpanTimings {}

impl SpanTimings {
    /// Creates an empty recorder.
    pub fn new() -> Self {
        todo!()
    }

    /// Timings of spans closed so far, in close order.
    pub fn finished(&self) -> Vec<SpanTiming> {
        todo!()
    }
}

impl<S> Layer<S> for SpanTimings
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_close(&self, _id: Id, _ctx: Context<'_, S>) {
        todo!()
    }
}

/// Layer that renders every event as one line and keeps the lines in memory,
/// masking the values of [`SECRET_FIELDS`].
///
/// Line format: `"<LEVEL> <message>"` followed by ` <name>=<value>` for every
/// other field, in emission order. `<LEVEL>` is `Level::as_str()` (`INFO`).
/// String values are written raw, other values with `Debug`. A secret field's
/// value is written as `***` whatever its type. An event with no message and
/// no fields renders as just `<LEVEL>`.
///
/// Clones share the same line list.
#[derive(Debug, Clone, Default)]
pub struct RedactedEvents {}

impl RedactedEvents {
    /// Creates an empty sink.
    pub fn new() -> Self {
        todo!()
    }

    /// Rendered lines so far, in event order.
    pub fn lines(&self) -> Vec<String> {
        todo!()
    }
}

impl<S: Subscriber> Layer<S> for RedactedEvents {
    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {
        todo!()
    }
}
