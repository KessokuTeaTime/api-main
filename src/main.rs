//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![allow(clippy::future_not_send)]

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

mod env;
mod fs;
mod logging;
mod state;
mod workflow;

mod endpoint;
mod middleware;

const PORT: u16 = 8086;
const MAX_RETRY: u8 = 5;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    logging::setup().unwrap();

    info!("Loaded environment: {:#?}", std::env::vars());
    info!("{:?}", std::env::var("KTT_API_USERNAME"));
    info!("Starting server on port {PORT}");

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{PORT}")).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
