//! KessokuTeaTime API backend.

#![allow(clippy::future_not_send)]

use crate::env::{
    PORT, TRACING_STDERR_LEVEL,
    info::{BUILD_TIMESTAMP, GIT_HASH},
};

use std::net::SocketAddr;

use anyhow::{Error, anyhow};
use axum::Router;
use tokio::net::TcpListener;

pub mod env;
pub mod trace;

pub mod endpoint;
pub mod middleware;

#[tokio::main]
async fn main() {
    env::setup();
    trace::setup().unwrap();
    tracing::info!("stderr is tracing on level {:?}", *TRACING_STDERR_LEVEL);
    tracing::trace!("loaded environment: {:#?}", std::env::vars());

    tracing::info!(
        "binary {} version {}",
        clap::crate_name!(),
        clap::crate_version!()
    );
    tracing::info!("compiled from commit {GIT_HASH} at {BUILD_TIMESTAMP}");
    tracing::info!("starting service on port {}…", *PORT);

    serve().await.unwrap();

    tracing::info!("stopping…");
}

async fn serve() -> Result<(), Error> {
    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT)).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| anyhow!(e))
}
