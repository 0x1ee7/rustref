//! Part 2: HTTP API: health, readiness, version, JSON 404.

use std::pin::Pin;

use axum::Router;

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
        todo!("Part 2: box the closure's future")
    }
}

/// Shared state for the router: application name and named readiness checks.
///
/// Fields are the engineer's to design. The state must be cheap to clone
/// (axum clones it per request), so share heavy parts behind `Arc`.
#[derive(Clone)]
pub struct AppState {}

impl AppState {
    /// State with no readiness checks, reporting `app_name` from `/version`.
    pub fn new(_app_name: impl Into<String>) -> Self {
        todo!("Part 2: AppState::new")
    }

    /// Registers a named readiness check. Checks are reported in `/readyz` under `name`.
    pub fn with_check(self, _name: impl Into<String>, _check: impl ReadinessCheck) -> Self {
        todo!("Part 2: AppState::with_check")
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
pub fn router(_state: AppState) -> Router {
    todo!("Part 2: router")
}
