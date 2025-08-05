use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tracing::trace;

const TRACING_REALM: &str = "[MIDDLEWARE] [LOGGER]";

pub(crate) async fn log_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    trace!(
        addr = format!("{addr}"),
        "{TRACING_REALM} Request received: {request:#?}"
    );
    next.run(request).await
}
