//! Endpoint `/health`.

use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};

/// Responds with [`StatusCode::OK`].
pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    tracing::info!(
        "service {} is healthy. responding to {addr}…",
        clap::crate_name!()
    );
    (
        StatusCode::OK,
        format!("Welcome to KessokuTeaTime, {addr}!"),
    )
}
