//! Part 3: serving with graceful shutdown.

use std::io;

use axum::Router;
use tokio::net::TcpListener;

/// Serves `router` on `listener` until `shutdown` resolves.
///
/// After `shutdown` resolves the server stops accepting new connections but lets
/// in-flight requests finish. Returns once they have drained. Built on
/// `axum::serve(..).with_graceful_shutdown(..)`.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
}
