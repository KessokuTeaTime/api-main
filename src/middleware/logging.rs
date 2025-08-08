//! Middlewares for logging.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Level, event};

/// Logs the request in detail. The event level is set to [`Level::TRACE`].
pub async fn log_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    event!(
        Level::TRACE,
        addr = format!("{addr}"),
        "received request: {request:#?}"
    );
    next.run(request).await
}
