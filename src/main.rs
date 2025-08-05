//! KessokuTeaTime API backend._

#![allow(clippy::future_not_send)]

use std::{env, net::SocketAddr, sync::LazyLock};

use axum::Router;
use spdlog::info;
use tokio::net::TcpListener;

mod fs;
mod state;
mod workflow;

mod endpoints;
mod middlewares;

const PORT: u16 = 8086;
const MAX_RETRY: u8 = 5;

/// The username of the API key.
static KTT_API_USERNAME: LazyLock<String> = LazyLock::new(|| {
    env::var("KTT_API_USERNAME").expect("KTT_API_USERNAME not set in environment")
});
/// The password of the API key.
static KTT_API_PASSWORD: LazyLock<String> = LazyLock::new(|| {
    env::var("KTT_API_PASSWORD").expect("KTT_API_PASSWORD not set in environment")
});

static GITHUB_TOKEN: LazyLock<String> =
    LazyLock::new(|| env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment"));

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
