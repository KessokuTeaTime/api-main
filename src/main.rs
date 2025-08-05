//! KessokuTeaTime API backend._

#![allow(clippy::future_not_send)]

use std::{env, net::SocketAddr, sync::LazyLock};

use axum::Router;
use spdlog::info;
use tokio::net::TcpListener;

mod fs;
mod gets;
mod posts;
mod state;
mod workflow;

const PORT: u16 = 8086;
const MAX_RETRY: u8 = 5;

static KTT_API_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("KTT_API_KEY").expect("KTT_API_KEY not set in environment"));
static GITHUB_TOKEN: LazyLock<String> =
    LazyLock::new(|| env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment"));

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
    info!("Starting server on port {PORT}");

    let mut app = Router::new();

    app = gets::route_from(app);
    app = posts::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
