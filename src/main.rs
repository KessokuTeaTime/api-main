use std::env;
use std::sync::Arc;
use actix_web::{App, HttpServer, middleware, Responder, get, HttpResponse};
use spdlog::{info, Logger};
use spdlog::sink::{RotatingFileSink, RotationPolicy};

mod internal;
mod structs;

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let path = env::current_exe()?.with_file_name("rotating_daily.log");

    let file_sink = Arc::new(
        RotatingFileSink::builder()
            .base_path(path)
            .rotation_policy(RotationPolicy::Daily { hour: 0, minute: 0 })
            .build().unwrap(),
    );
    let new_logger = Arc::new(Logger::builder().sink(file_sink).build().unwrap());
    spdlog::set_default_logger(new_logger);

    info!("the logger initialized");
    info!("Starting server on port 8086");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(internal::notify)
            .service(health)
    })
    .bind("0.0.0.0:8086")?
    .run()
    .await
}
