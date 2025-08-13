//! KessokuTeaTime API backend.

#![allow(clippy::future_not_send)]

use crate::env::{
    PORT,
    info::{BUILD_TIMESTAMP, GIT_HASH},
};

use std::net::SocketAddr;

use api_framework::shutdown;
use axum::Router;
use tokio::net::TcpListener;
use tracing::{info, trace};

pub mod env;
pub mod trace;

pub mod endpoint;
pub mod middleware;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    dotenvy::from_filename_override(format!("{}.env", clap::crate_name!())).ok();
    trace::setup().unwrap();

    trace!("loaded environment: {:#?}", std::env::vars());
    info!(
        "binary {} version {}",
        clap::crate_name!(),
        clap::crate_version!()
    );
    info!("compiled from commit {GIT_HASH} at {BUILD_TIMESTAMP}");
    info!("starting server on port {}…", *PORT);

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT))
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::signal())
    .await
    .unwrap();

    info!("stopping!");
}
