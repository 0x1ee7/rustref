//! Part 3: graceful shutdown on a real loopback listener.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::routing::get;
use ex18_deployment::server::serve;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, oneshot};
use tokio::time::timeout;

const LIMIT: Duration = Duration::from_secs(5);

/// Handler that announces it started, then takes 300 ms to answer.
async fn slow(State(started): State<Arc<Notify>>) -> &'static str {
    started.notify_one();
    tokio::time::sleep(Duration::from_millis(300)).await;
    "done"
}

fn app(started: Arc<Notify>) -> Router {
    Router::new()
        .route("/slow", get(slow))
        .route("/fast", get(|| async { "fast" }))
        .with_state(started)
}

/// Minimal HTTP/1.1 GET over a raw socket. Returns (status code, body).
async fn raw_get(addr: SocketAddr, path: &str) -> io::Result<(u16, String)> {
    let mut stream = TcpStream::connect(addr).await?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let text = String::from_utf8_lossy(&raw).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    Ok((status, body))
}

async fn bind() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback bind");
    let addr = listener.local_addr().expect("local addr");
    (listener, addr)
}

#[tokio::test]
async fn part3_serves_requests_until_shutdown() {
    let (listener, addr) = bind().await;
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, app(Arc::new(Notify::new())), async {
        let _ = rx.await;
    }));

    let (status, body) = timeout(LIMIT, raw_get(addr, "/fast"))
        .await
        .expect("request must not hang")
        .expect("request must succeed");
    assert_eq!((status, body.as_str()), (200, "fast"), "server must answer before shutdown");

    tx.send(()).expect("server still listening");
    timeout(LIMIT, server).await.expect("serve must return after shutdown").unwrap().unwrap();
}

#[tokio::test]
async fn part3_in_flight_request_completes_after_shutdown_is_triggered() {
    let (listener, addr) = bind().await;
    let started = Arc::new(Notify::new());
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, app(started.clone()), async {
        let _ = rx.await;
    }));

    let request = tokio::spawn(raw_get(addr, "/slow"));
    timeout(LIMIT, started.notified())
        .await
        .expect("the slow handler must start");

    // Shutdown is requested while /slow is still sleeping.
    tx.send(()).expect("server still listening");

    let (status, body) = timeout(LIMIT, request)
        .await
        .expect("in-flight request must finish")
        .unwrap()
        .expect("in-flight request must not be cut off");
    assert_eq!(status, 200, "in-flight request must complete with 200, not be dropped");
    assert_eq!(body, "done", "in-flight request must receive its full response");

    timeout(LIMIT, server)
        .await
        .expect("serve must return once in-flight work has drained")
        .unwrap()
        .expect("serve returns Ok after a graceful shutdown");
}

#[tokio::test]
async fn part3_no_new_connections_are_accepted_after_shutdown() {
    let (listener, addr) = bind().await;
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, app(Arc::new(Notify::new())), async {
        let _ = rx.await;
    }));

    tx.send(()).expect("server still listening");
    timeout(LIMIT, server).await.expect("serve returns").unwrap().unwrap();

    let connect = TcpStream::connect(addr).await;
    assert!(connect.is_err(), "the listener must be closed once serve has returned");
}

#[tokio::test]
async fn part3_idle_server_stops_promptly_on_shutdown() {
    let (listener, _addr) = bind().await;
    let (tx, rx) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, app(Arc::new(Notify::new())), async {
        let _ = rx.await;
    }));
    tx.send(()).expect("server still listening");
    let result = timeout(LIMIT, server)
        .await
        .expect("an idle server must stop without waiting for traffic");
    result.unwrap().expect("serve returns Ok");
}

#[tokio::test]
async fn part3_shutdown_future_that_never_resolves_keeps_serving() {
    let (listener, addr) = bind().await;
    let server = tokio::spawn(serve(
        listener,
        app(Arc::new(Notify::new())),
        std::future::pending::<()>(),
    ));

    let (status, body) = timeout(LIMIT, raw_get(addr, "/fast"))
        .await
        .expect("request must not hang")
        .expect("request must succeed");
    assert_eq!((status, body.as_str()), (200, "fast"));
    assert!(!server.is_finished(), "a pending shutdown future must keep the server running");
    server.abort();
}
