use std::net::SocketAddr;

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use spdlog::debug;

pub async fn get(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    let msg = format!("Huston, good to hear from {addr}!");
    debug!("{msg}");
    (StatusCode::OK, msg)
}
