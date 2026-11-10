#![allow(dead_code)]
//! Test-only capture layer shared by the `part*` test files.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::{LookupSpan, Registry, SpanRef};

pub type Fields = BTreeMap<String, String>;

/// A span as seen when it was entered or closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanSnapshot {
    pub name: String,
    pub target: String,
    pub fields: Fields,
}

/// An event plus the spans that were open around it (outermost first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    pub level: Level,
    pub target: String,
    pub fields: Fields,
    pub scope: Vec<SpanSnapshot>,
}

impl Captured {
    /// The `message` field, or `""` when absent.
    pub fn message(&self) -> &str {
        self.field("message").unwrap_or("")
    }

    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    pub fn scope_names(&self) -> Vec<&str> {
        self.scope.iter().map(|s| s.name.as_str()).collect()
    }
}

/// Everything a [`Recorder`] saw.
#[derive(Debug, Clone, Default)]
pub struct Recording {
    pub events: Vec<Captured>,
    pub closed: Vec<SpanSnapshot>,
}

/// Layer that stores events and closed spans in memory. Clones share storage.
#[derive(Clone, Default)]
pub struct Recorder {
    inner: Arc<Mutex<Recording>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Recording {
        self.inner.lock().unwrap().clone()
    }
}

/// Span fields stored in the span's extensions, kept up to date by `on_record`.
struct SpanFields(Fields);

struct FieldWriter<'a>(&'a mut Fields);

impl Visit for FieldWriter<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

fn snapshot_span<S: for<'a> LookupSpan<'a>>(span: &SpanRef<'_, S>) -> SpanSnapshot {
    let fields = span
        .extensions()
        .get::<SpanFields>()
        .map(|f| f.0.clone())
        .unwrap_or_default();
    SpanSnapshot {
        name: span.name().to_owned(),
        target: span.metadata().target().to_owned(),
        fields,
    }
}

impl<S> Layer<S> for Recorder
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut fields = Fields::new();
        attrs.record(&mut FieldWriter(&mut fields));
        span.extensions_mut().insert(SpanFields(fields));
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut ext = span.extensions_mut();
        if let Some(SpanFields(fields)) = ext.get_mut::<SpanFields>() {
            values.record(&mut FieldWriter(fields));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Fields::new();
        event.record(&mut FieldWriter(&mut fields));
        let scope = ctx
            .event_scope(event)
            .map(|scope| scope.from_root().map(|span| snapshot_span(&span)).collect())
            .unwrap_or_default();
        let meta = event.metadata();
        self.inner.lock().unwrap().events.push(Captured {
            level: *meta.level(),
            target: meta.target().to_owned(),
            fields,
            scope,
        });
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(&id) {
            let snap = snapshot_span(&span);
            self.inner.lock().unwrap().closed.push(snap);
        }
    }
}

/// Runs `f` with a [`Recorder`] installed for this thread only and returns what was recorded.
pub fn record<F: FnOnce()>(f: F) -> Recording {
    let recorder = Recorder::new();
    let subscriber = Registry::default().with(recorder.clone());
    tracing::subscriber::with_default(subscriber, f);
    recorder.snapshot()
}

/// Every key and value captured in events and spans, concatenated. Handy for "secret never appears" checks.
pub fn all_text(rec: &Recording) -> String {
    let mut out = String::new();
    let spans = rec.events.iter().flat_map(|e| e.scope.iter()).chain(rec.closed.iter());
    for span in spans {
        for (k, v) in &span.fields {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
    }
    for event in &rec.events {
        for (k, v) in &event.fields {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
    }
    out
}
