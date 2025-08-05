use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use tracing::debug;

pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    debug!("Responding to {addr}…");
    (StatusCode::OK, format!("Huston, good to hear from {addr}!"))
}
