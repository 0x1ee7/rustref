# 10 — Logging and tracing with `tracing`

## Scenario

You own the order service of a small shop. Three things run in the same process:

- an HTTP-like request handler that answers `GET /orders/<id>`;
- a payment call (`charge_card`) that talks to a card gateway and must never leak card tokens;
- a nightly settlement job that fans a batch of amounts out to worker threads.

When an incident page fires at 03:00, the on-call engineer needs one request id to
follow a request across every log line it produced, a JSON stream that a log
pipeline can index, per-module verbosity that can be changed through configuration,
and proof that secrets never reach the sink. This exercise builds that
observability layer with `tracing` and `tracing-subscriber`.

Logs are a product surface. Treat field names, levels and span names as API that
dashboards and alerts depend on.

## Layout

| File | Part | What you implement |
| --- | --- | --- |
| `src/payments.rs` | 1 | `charge_card`, `quote_total` (events, `#[instrument]`) |
| `src/http.rs` | 2 | `load_order`, `handle_request` (spans, context, late field recording) |
| `src/filter.rs` | 3 | `build_filter` (`EnvFilter` directives, validation) |
| `src/json.rs` | 4 | `CaptureWriter`, `json_subscriber` (`fmt().json()`, `MakeWriter`) |
| `src/layers.rs` | 5 | `EventCounter`, `SpanTimings`, `RedactedEvents` (custom `Layer`s, `Visit`) |
| `src/stack.rs` | 6 | `app_stack` (composing a whole subscriber) |
| `src/job.rs` | 6 | `settle_batch` (spans across threads) |

The types and error enums are already declared with doc comments that carry the
spec. Replace each `todo!()`. Do not change public signatures.

## Part 1 — Events, fields and `#[instrument]`

`charge_card(order_id, amount_cents, card_token) -> Result<String, ChargeError>`

- `amount_cents <= 0` → `ChargeError::InvalidAmount(amount)`; no events emitted for it.
- A token starting with `decline` → `ChargeError::Declined { reason: "do_not_honor" }`.
- A token starting with `down` → `ChargeError::GatewayUnavailable`.
- Otherwise return `txn_<order_id>_<amount_cents>`.
- Emit a DEBUG event `"authorising"` with fields `order_id` and `amount_cents`.
- Emit an INFO event `"charged"` with field `txn_id` on success.
- The function is `#[instrument]`ed: span `charge_card`, arguments recorded as fields,
  `card_token` **skipped**, `provider = "sim-gateway"` added as a static field, and the
  `err` attribute emits an ERROR event with an `error` field on failure.

`quote_total(items_cents: &[i64]) -> i64` is `#[instrument(ret)]`ed: the returned
value is recorded on the span's event under the `return` field.

**Edge cases:** the event target is the module path; a declined card's ERROR event
carries the `Display` text of the error; no subscriber installed means no panic.

## Part 2 — Spans, context and request ids

`handle_request(req: &Request, store: &OrderStore) -> Response`

- Open span `http.request` with `request_id`, `method`, `path`, and an empty `status`
  that you fill in with `Span::record` after the response is known.
- Every event inside the request (including those from nested calls) inherits
  `request_id`, because it sits inside the request span.
- `load_order` is `#[instrument(name = "store.load", skip(store, id), fields(order_id = %id))]`
  and emits a DEBUG `"order lookup"`. Its span is a child of `http.request`.
- Routing:
  - path not starting with `/orders/` → 404 `"no route"` (no lookup);
  - any method other than `GET` on `/orders/...` → 405 `"method not allowed"` (no lookup);
  - order missing → 404 `"order not found"` plus a WARN event carrying `order_id`;
  - order found → 200 with body `"order <id> amount=<cents>"`.
- Emit an INFO `"request completed"` with `status`.

**Edge cases:** the request id must not leak into a later request; the handler works
with no subscriber installed.

## Part 3 — Filtering with `EnvFilter`

`build_filter(default: &str, overrides: &[(&str, &str)]) -> Result<EnvFilter, FilterError>`

- `default` applies to every target without an override.
- An override `(target, level)` covers `target` and its child module paths.
- **Gotcha:** `EnvFilter` matches directive targets by plain string prefix, so the
  override `shop::db` also covers the sibling `shop::dbx`. Part 3 pins this behaviour.
  Name full module paths to avoid it. Do not work around it in `build_filter`.
- The most specific directive wins.
- Level names are case-insensitive. `off` silences a target.
- Validation: an empty override target → `EmptyTarget`; an unknown level →
  `UnknownLevel` (with `target` set to `""` when the bad level is the default).

**Edge cases:** directive strings in targets must not inject extra directives
(build them through typed APIs, not by concatenating untrusted text).

## Part 4 — JSON output

`json_subscriber<W>(writer) -> impl Subscriber` writes one JSON object per event
through `fmt().json()`, with no ANSI escapes, and with **no filtering**: every
level, including TRACE, reaches the sink.

`CaptureWriter` is an in-memory `io::Write` and `MakeWriter` whose clones share one
buffer. `contents()` returns everything written so far.

Object shape (checked by the tests): top-level `level` (upper case, e.g. `"ERROR"`),
`timestamp`, `target`, `fields` (with `message`, and typed values such as numbers
kept as JSON numbers), a `span` key for the innermost span, and `spans` ordered
outermost first.

## Part 5 — Custom `Layer`s

- `EventCounter`: counts events per level; ignores spans; `snapshot()` is a
  point-in-time copy; clones share the counters.
- `SpanTimings`: records a `SpanTiming { name, parent, busy }` when each span closes.
  `busy` is the sum of enter-to-exit intervals across every entry, so idle time
  between entries is excluded. Each span closes exactly once.
- `RedactedEvents`: renders each event as one line `"<LEVEL> <message> k=v ..."`.
  Strings are written raw, other values use `Debug` (except integers and booleans,
  which use their plain form), fields named in `SECRET_FIELDS` become `***`, and
  there is no trailing space.

**Edge cases:** secrets are masked whatever their type (string, `Debug`, integer);
non-secret fields are never masked; event order is preserved.

## Part 6 — Composing a stack and fanning out work

- `app_stack(filter, counter, writer)`: `Registry` with the filter first, then the
  counter, then a JSON layer writing to `writer`. Only events that pass the filter
  reach the counter and the sink.
- `settle_batch(batch_id, amounts_cents, workers: NonZeroUsize) -> SettlementReport`:
  - split the input into chunks of `ceil(len / workers)` (never more chunks than
    needed; an empty batch returns zeros);
  - open span `settle_batch` with `batch_id` (other arguments skipped);
  - each worker runs under span `settle.worker` with `worker` set to its 0-based index,
    as a child of the batch span, on its own thread;
  - amounts `> 0` produce DEBUG `"settled"` with `amount_cents`; amounts `<= 0` produce
    WARN `"rejected"` with `amount_cents`.
  - The report is independent of worker count.

Worker threads do not inherit the thread-local dispatcher. Propagate the dispatch
and the parent span explicitly into each thread.

**Edge cases:** more workers than items; an empty batch; the report for `workers`
in `1..=5` is identical.

## Concepts exercised

- `tracing` events and levels; structured fields (`%` and `?` sigils, `Empty` and
  `Span::record`); the `#[instrument]` attribute (`skip`, `fields`, `name`, `ret`, `err`).
- Span trees, context inheritance, `Span::current` and `in_scope`.
- Filtering with `EnvFilter` directives; typed error enums with `Display` and `Error`.
- `tracing_subscriber::fmt().json()`, `MakeWriter`, `with_default` for scoped subscribers.
- Custom `Layer`s with `on_event`, `on_new_span`, `on_enter`, `on_exit`, `on_close`;
  `Visit`; extensions as per-span storage; `LookupSpan`.
- `Registry` composition with `SubscriberExt::with` and `Layer::boxed`.
- Propagating the dispatcher and span context into `std::thread::scope` threads.
- Secret handling: `skip` in spans, `***` masking in sinks.

## Run

```sh
cargo test -p ex10_logging_tracing                 # all parts
cargo test -p ex10_logging_tracing part2           # one part (test names are prefixed partN_)
cargo solutions -p ex10_logging_tracing            # reference solution
```

Every test installs its subscriber with `with_default`, so tests run in parallel
without sharing global state.

## Stretch (untested)

- Make `handle_request` generate a request id when the edge did not supply one, and
  carry it in a response header-like field.
- Add a `Layer` that drops events whose message repeats within a window, using a
  `Mutex<HashMap<..>>` keyed by message, and test it with `start_paused` time.
- Write a `MakeWriter` that rotates by byte count, and measure how many writes
  reach the `Mutex` under contention.
- Replace the string prefix in Part 3 with a custom `Filter` that matches whole
  path segments, and compare it with `EnvFilter`.
