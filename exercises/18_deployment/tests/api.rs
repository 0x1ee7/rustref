//! Part 2: HTTP API: health, readiness, version, JSON 404. Driven with `oneshot`; no sockets.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use ex18_deployment::api::{AppState, router};
use ex18_deployment::build_info;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Sends one request through the router and returns status, headers and the parsed body bytes.
async fn send(app: Router, method: Method, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("valid request");
    let resp = app.oneshot(req).await.expect("router is infallible");
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp.into_body().collect().await.expect("body collects").to_bytes();
    (status, headers, body.to_vec())
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let (status, _, body) = send(app, Method::GET, uri).await;
    let value = serde_json::from_slice(&body)
        .unwrap_or_else(|e| panic!("{uri} must return JSON, got {:?}: {e}", String::from_utf8_lossy(&body)));
    (status, value)
}

/// A check that counts its invocations and always succeeds.
fn counting_ok(counter: Arc<AtomicUsize>) -> impl Fn() -> std::future::Ready<Result<(), String>> + Send + Sync + 'static {
    move || {
        counter.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

// --- /healthz --------------------------------------------------------------

#[tokio::test]
async fn part2_healthz_returns_ok_json() {
    let (status, body) = get_json(router(AppState::new("svc")), "/healthz").await;
    assert_eq!(status, StatusCode::OK, "/healthz must be 200");
    assert_eq!(body, json!({"status": "ok"}), "/healthz body");
}

#[tokio::test]
async fn part2_healthz_never_runs_readiness_checks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let state = AppState::new("svc").with_check("db", counting_ok(calls.clone()));
    let (status, _) = get_json(router(state), "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 0, "liveness must not run readiness checks");
}

// --- /readyz ---------------------------------------------------------------

#[tokio::test]
async fn part2_readyz_all_checks_pass_returns_200() {
    let calls = Arc::new(AtomicUsize::new(0));
    let state = AppState::new("svc")
        .with_check("db", counting_ok(calls.clone()))
        .with_check("cache", counting_ok(calls.clone()));
    let (status, body) = get_json(router(state), "/readyz").await;
    assert_eq!(status, StatusCode::OK, "all checks pass → 200");
    assert_eq!(
        body,
        json!({"status": "ready", "checks": {"db": "ok", "cache": "ok"}}),
        "readyz body lists every check"
    );
}

#[tokio::test]
async fn part2_readyz_failing_check_returns_503_and_reports_every_check() {
    let state = AppState::new("svc")
        .with_check("db", || async { Err::<(), String>("connection refused".to_string()) })
        .with_check("cache", || async { Ok::<(), String>(()) });
    let (status, body) = get_json(router(state), "/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "a failing check → 503");
    assert_eq!(
        body,
        json!({
            "status": "not_ready",
            "checks": {"db": "error: connection refused", "cache": "ok"}
        }),
        "every check is reported, even the passing one"
    );
}

#[tokio::test]
async fn part2_readyz_runs_checks_on_every_request() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = router(AppState::new("svc").with_check("db", counting_ok(calls.clone())));
    for _ in 0..3 {
        let (status, _) = get_json(app.clone(), "/readyz").await;
        assert_eq!(status, StatusCode::OK);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3, "readiness must not be cached: one run per request");
}

#[tokio::test]
async fn part2_readyz_without_checks_is_ready() {
    let (status, body) = get_json(router(AppState::new("svc")), "/readyz").await;
    assert_eq!(status, StatusCode::OK, "no checks registered means nothing can be unhealthy");
    assert_eq!(body, json!({"status": "ready", "checks": {}}));
}

#[tokio::test]
async fn part2_readyz_runs_checks_concurrently() {
    // Two checks that each wait on a 2-party barrier. Sequential execution deadlocks,
    // so the timeout turns a sequential implementation into a failure instead of a hang.
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let make = |b: Arc<tokio::sync::Barrier>| {
        move || {
            let b = b.clone();
            async move {
                b.wait().await;
                Ok::<(), String>(())
            }
        }
    };
    let state = AppState::new("svc")
        .with_check("a", make(barrier.clone()))
        .with_check("b", make(barrier.clone()));
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        get_json(router(state), "/readyz"),
    )
    .await;
    let (status, body) = result.expect("checks must run concurrently; sequential execution deadlocks");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

// --- /version --------------------------------------------------------------

#[tokio::test]
async fn part2_version_reports_app_version_and_git_sha() {
    let (status, body) = get_json(router(AppState::new("orders")), "/version").await;
    assert_eq!(status, StatusCode::OK, "/version must be 200");
    assert_eq!(body["app"], "orders", "app comes from AppState");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"), "version comes from the crate");
    assert_eq!(
        body["git_sha"],
        build_info::git_sha(),
        "git_sha must be the build-time value from build_info"
    );
}

#[tokio::test]
async fn part2_version_git_sha_is_unknown_or_full_hex() {
    let (_, body) = get_json(router(AppState::new("svc")), "/version").await;
    let sha = body["git_sha"].as_str().expect("git_sha is a JSON string");
    let ok = sha == "unknown" || (sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    assert!(ok, "git_sha must be `unknown` or 40 lowercase hex chars, got {sha:?}");
}

// --- HEAD, 404, 405, headers ----------------------------------------------

#[tokio::test]
async fn part2_head_returns_same_status_and_headers_with_empty_body() {
    let app = router(AppState::new("svc"));
    let (get_status, get_headers, _) = send(app.clone(), Method::GET, "/healthz").await;
    let (head_status, head_headers, head_body) = send(app, Method::HEAD, "/healthz").await;
    assert_eq!(head_status, get_status, "HEAD must return the GET status");
    assert_eq!(
        head_headers.get(header::CONTENT_TYPE),
        get_headers.get(header::CONTENT_TYPE),
        "HEAD must return the GET content type"
    );
    assert!(head_body.is_empty(), "HEAD must not carry a body");
}

#[tokio::test]
async fn part2_unknown_path_is_json_404_naming_the_path() {
    let (status, body) = get_json(router(AppState::new("svc")), "/no/such/thing").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "unknown path → 404");
    assert_eq!(body, json!({"error": "not_found", "path": "/no/such/thing"}));
}

#[tokio::test]
async fn part2_wrong_method_on_known_route_is_405() {
    let (status, _, _) = send(router(AppState::new("svc")), Method::POST, "/healthz").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "POST /healthz → 405");
}

#[tokio::test]
async fn part2_json_responses_have_accurate_content_length() {
    let app = router(AppState::new("svc"));
    for uri in ["/healthz", "/readyz", "/version"] {
        let (_, headers, body) = send(app.clone(), Method::GET, uri).await;
        assert_eq!(
            headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
            Some("application/json; charset=utf-8"),
            "{uri} must be application/json; charset=utf-8"
        );
        let declared: usize = headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("{uri} must declare Content-Length"));
        assert_eq!(declared, body.len(), "{uri} Content-Length must match the body");
    }
}
