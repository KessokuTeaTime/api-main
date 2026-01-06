//! Endpoint `/health`.

use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use tracing::info;

/// Gets the server health info.
/// Responds with [`StatusCode::OK`].
pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    info!("server is healthy. responding to {addr}…");
    (StatusCode::OK, format!("Good to hear from {addr}!"))
}
