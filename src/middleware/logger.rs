use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use tracing::{Level, event};

const TRACING_REALM: &str = "[MIDDLEWARE] [LOGGER]";

pub(crate) async fn log_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    event!(
        Level::TRACE,
        addr = format!("{addr}"),
        request = format!("{request:#?}"),
        "{TRACING_REALM} Request received"
    );
    next.run(request).await
}
