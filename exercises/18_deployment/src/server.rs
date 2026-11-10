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
    _listener: TcpListener,
    _router: Router,
    _shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    todo!("Part 3: serve")
}
