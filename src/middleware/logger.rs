use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tracing::trace;

pub(crate) async fn log_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    trace!(
        addr = format!("{addr}"),
        "Request received: {request:#?}"
    );
    next.run(request).await
}
