use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use tracing::debug;

pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    debug!("server is healthy. responding to {addr}…");
    (StatusCode::OK, format!("Huston, good to hear from {addr}!"))
}
