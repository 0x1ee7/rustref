//! Part 6 — composing a whole subscriber stack with `registry().with(...)`.

use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

use crate::layers::EventCounter;

/// Builds the application's subscriber: filter, then counter, then JSON output.
pub fn app_stack<W>(
    filter: EnvFilter,
    counter: EventCounter,
    writer: W,
) -> impl Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let json = fmt::layer().json().with_writer(writer);
    Registry::default()
        .with(filter)
        .with(counter)
        .with(json.boxed())
}
