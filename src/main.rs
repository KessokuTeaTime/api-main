//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![allow(clippy::future_not_send)]

use std::net::SocketAddr;

use axum::Router;
use spdlog::info;
use tokio::net::TcpListener;

mod env;
mod fs;
mod state;
mod workflow;

mod endpoints;
mod middlewares;

const PORT: u16 = 8086;
const MAX_RETRY: u8 = 5;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
    info!("Starting server on port {PORT}");

    let mut app = Router::new();
    app = endpoints::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
