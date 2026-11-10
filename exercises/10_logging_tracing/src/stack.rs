//! Part 6 — composing a whole subscriber stack with `registry().with(...)`.

use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, Registry};

use crate::layers::EventCounter;

/// Builds the application's subscriber: the `filter` gates everything, then
/// the `counter` layer counts what passed, then a JSON `fmt` layer writes it
/// to `writer`.
///
/// Behaviour (Part 6): an event rejected by `filter` reaches neither the
/// counter nor the output. Layer order matters; the filter comes first.
#[allow(unreachable_code)]
pub fn app_stack<W>(
    filter: EnvFilter,
    counter: EventCounter,
    writer: W,
) -> impl Subscriber + Send + Sync
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let _ = (filter, counter, writer);
    todo!();
    Registry::default()
}
