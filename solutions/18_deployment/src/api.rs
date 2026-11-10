//! Part 2: HTTP API: health, readiness, version, JSON 404.

use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Map, Value, json};
use tokio::task::JoinSet;

use crate::build_info;

/// Boxed future returned by a readiness check. `Ok(())` means healthy;
/// `Err(message)` means unhealthy and the message is reported to the caller.
pub type CheckFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// An async readiness probe. Run on every `/readyz` request.
pub trait ReadinessCheck: Send + Sync + 'static {
    /// Starts one check run.
    fn check(&self) -> CheckFuture;
}

/// Any `Fn() -> Future<Output = Result<(), String>>` is a readiness check.
impl<F, Fut> ReadinessCheck for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    fn check(&self) -> CheckFuture {
        Box::pin(self())
    }
}

/// A registered readiness check and the name it is reported under.
#[derive(Clone)]
struct NamedCheck {
    name: Arc<str>,
    check: Arc<dyn ReadinessCheck>,
}

/// Shared state for the router: application name and named readiness checks.
///
/// Cloned per request by axum; both fields are behind `Arc`, so a clone is two refcount bumps.
#[derive(Clone)]
pub struct AppState {
    app_name: Arc<str>,
    checks: Arc<[NamedCheck]>,
}

impl AppState {
    /// State with no readiness checks, reporting `app_name` from `/version`.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: Arc::from(app_name.into()),
            checks: Arc::from(Vec::<NamedCheck>::new()),
        }
    }

    /// Registers a named readiness check. Checks are reported in `/readyz` under `name`.
    pub fn with_check(self, name: impl Into<String>, check: impl ReadinessCheck) -> Self {
        let mut checks = self.checks.to_vec();
        checks.push(NamedCheck {
            name: Arc::from(name.into()),
            check: Arc::new(check),
        });
        Self {
            checks: Arc::from(checks),
            ..self
        }
    }
}

/// Builds the router.
///
/// * `GET /healthz` → `200 {"status":"ok"}`. Never runs checks.
/// * `GET /readyz` → runs every check, all concurrently. `200 {"status":"ready","checks":{name:"ok"}}`
///   when all pass; otherwise `503 {"status":"not_ready","checks":{name:"ok"|"error: <message>"}}`.
///   Every check is reported even when another fails.
/// * `GET /version` → `200 {"app":<app_name>,"version":<CARGO_PKG_VERSION>,"git_sha":<build sha>}`.
/// * anything else → `404 {"error":"not_found","path":<request path>}`.
///
/// `HEAD` is answered by the matching `GET` route with an empty body.
/// Wrong method on a known route → `405`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/version", get(version))
        .fallback(not_found)
        .with_state(state)
}

async fn healthz() -> Response {
    json_response(StatusCode::OK, &json!({"status": "ok"}))
}

async fn readyz(State(state): State<AppState>) -> Response {
    // Spawn every check first so they all make progress together; then collect.
    let mut set = JoinSet::new();
    for (index, named) in state.checks.iter().enumerate() {
        let run = named.check.check();
        set.spawn(async move { (index, run.await) });
    }
    let mut outcomes: Vec<Option<Result<(), String>>> = vec![None; state.checks.len()];
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((index, outcome)) => outcomes[index] = Some(outcome),
            // A panicking check is a bug in the check, not a readiness result: surface it.
            Err(join_error) => std::panic::resume_unwind(join_error.into_panic()),
        }
    }

    let mut all_ready = true;
    let mut report = Map::new();
    for (named, outcome) in state.checks.iter().zip(outcomes) {
        let entry = match outcome.expect("every spawned check reports an outcome") {
            Ok(()) => "ok".to_owned(),
            Err(message) => {
                all_ready = false;
                format!("error: {message}")
            }
        };
        report.insert(named.name.to_string(), Value::String(entry));
    }

    let (status, label) = if all_ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not_ready")
    };
    json_response(status, &json!({"status": label, "checks": report}))
}

async fn version(State(state): State<AppState>) -> Response {
    json_response(
        StatusCode::OK,
        &json!({
            "app": &*state.app_name,
            "version": build_info::version(),
            "git_sha": build_info::git_sha(),
        }),
    )
}

async fn not_found(uri: Uri) -> Response {
    json_response(
        StatusCode::NOT_FOUND,
        &json!({"error": "not_found", "path": uri.path()}),
    )
}

/// Serialises `body` with an explicit content type and length.
fn json_response(status: StatusCode, body: &Value) -> Response {
    let bytes = serde_json::to_vec(body).expect("a JSON value always serializes");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(bytes.len()));
    (status, headers, bytes).into_response()
}
