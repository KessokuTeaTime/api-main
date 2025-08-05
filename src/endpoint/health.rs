use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use tracing::debug;

const TRACING_REALM: &str = "[ENDPOINT] [GET /health]";

pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    debug!("{TRACING_REALM} Responding to {addr}…");
    (StatusCode::OK, format!("Huston, good to hear from {addr}!"))
}
