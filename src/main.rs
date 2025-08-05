//! KessokuTeaTime API backend._

#![allow(clippy::future_not_send)]

use std::{env, sync::LazyLock};

use actix_web::{App, HttpServer, middleware};
use spdlog::info;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
    info!("Starting server on port {PORT}");

    HttpServer::new(move || {
        let mut app = App::new().wrap(middleware::Logger::default());

        app = gets::register_services(app);
        app = posts::register_services(app);

        app
    })
    .bind(format!("0.0.0.0:{PORT}"))?
    .run()
    .await
}
