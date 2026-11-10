//! Part 5 — custom `Layer`s: event counting, span timings, and secret redaction
//! through a `Visit` implementation.

use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::field::{Field, Visit};
use tracing::span::Id;
use tracing::{Event, Level, Subscriber};
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

/// Layer that counts events per level. Clones share the same counters.
#[derive(Debug, Clone, Default)]
pub struct EventCounter {
    counts: Arc<Mutex<LevelCounts>>,
}

impl EventCounter {
    /// Creates a counter with all totals at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Point-in-time copy of the totals.
    pub fn snapshot(&self) -> LevelCounts {
        *self.counts.lock().expect("counter lock")
    }
}

impl<S: Subscriber> Layer<S> for EventCounter {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut counts = self.counts.lock().expect("counter lock");
        let slot = match *event.metadata().level() {
            Level::TRACE => &mut counts.trace,
            Level::DEBUG => &mut counts.debug,
            Level::INFO => &mut counts.info,
            Level::WARN => &mut counts.warn,
            Level::ERROR => &mut counts.error,
        };
        *slot += 1;
    }
}

/// Timing of one closed span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanTiming {
    /// The span's name.
    pub name: String,
    /// Name of the enclosing span, if any.
    pub parent: Option<String>,
    /// Time the span was *entered*, summed over every entry.
    pub busy: Duration,
}

/// Per-span state kept in the span's extensions.
struct Timing {
    busy: Duration,
    entered_at: Option<Instant>,
}

/// Layer that records a [`SpanTiming`] for every span when it closes.
#[derive(Debug, Clone, Default)]
pub struct SpanTimings {
    finished: Arc<Mutex<Vec<SpanTiming>>>,
}

impl SpanTimings {
    /// Creates an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Timings of spans closed so far, in close order.
    pub fn finished(&self) -> Vec<SpanTiming> {
        self.finished.lock().expect("timings lock").clone()
    }
}

impl<S> Layer<S> for SpanTimings
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, _attrs: &tracing::span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(Timing {
                busy: Duration::ZERO,
                entered_at: None,
            });
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut extensions = span.extensions_mut();
        if let Some(timing) = extensions.get_mut::<Timing>() {
            timing.entered_at = Some(Instant::now());
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut extensions = span.extensions_mut();
        if let Some(timing) = extensions.get_mut::<Timing>()
            && let Some(start) = timing.entered_at.take()
        {
            timing.busy += start.elapsed();
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let busy = span
            .extensions()
            .get::<Timing>()
            .map_or(Duration::ZERO, |t| t.busy);
        let parent = span.parent().map(|p| p.name().to_owned());
        self.finished
            .lock()
            .expect("timings lock")
            .push(SpanTiming {
                name: span.name().to_owned(),
                parent,
                busy,
            });
    }
}

/// Collects an event's fields into a rendered string, masking secrets.
struct Redact {
    line: String,
}

impl Redact {
    fn push(&mut self, name: &str, value: fmt::Arguments<'_>) {
        let _ = write!(self.line, " {name}=");
        if SECRET_FIELDS.contains(&name) {
            self.line.push_str("***");
        } else {
            let _ = self.line.write_fmt(value);
        }
    }
}

impl Visit for Redact {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.line.push(' ');
            self.line.push_str(value);
        } else {
            self.push(field.name(), format_args!("{value}"));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.line, " {value:?}");
        } else {
            self.push(field.name(), format_args!("{value:?}"));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field.name(), format_args!("{value}"));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field.name(), format_args!("{value}"));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field.name(), format_args!("{value}"));
    }
}

/// Layer that renders every event as one redacted line and keeps the lines.
#[derive(Debug, Clone, Default)]
pub struct RedactedEvents {
    lines: Arc<Mutex<Vec<String>>>,
}

impl RedactedEvents {
    /// Creates an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rendered lines so far, in event order.
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("lines lock").clone()
    }
}

impl<S: Subscriber> Layer<S> for RedactedEvents {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut redact = Redact {
            line: event.metadata().level().to_string(),
        };
        event.record(&mut redact);
        self.lines.lock().expect("lines lock").push(redact.line);
    }
}
