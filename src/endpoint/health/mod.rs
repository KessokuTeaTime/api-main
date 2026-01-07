//! Endpoint `/health`.

use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use tracing::info;

/// Responds with [`StatusCode::OK`].
pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    info!(
        "service {} is healthy. responding to {addr}…",
        clap::crate_name!()
    );
    (StatusCode::OK, format!("Good to hear from {addr}!"))
}
