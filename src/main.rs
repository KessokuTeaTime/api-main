//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![feature(fn_traits)]
#![feature(async_fn_traits)]
#![feature(unboxed_closures)]
#![allow(clippy::future_not_send)]

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tracing::{info, trace};

use crate::env::PORT;

pub mod env;
pub mod framework;
pub mod fs;
pub mod logging;
pub mod workflow;

pub mod endpoint;
pub mod middleware;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    logging::setup().unwrap();

    trace!("loaded environment: {:#?}", std::env::vars());
    info!("starting server on port {}", *PORT);

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT))
        .await
        .unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
