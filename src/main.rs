use actix_web::{App, HttpServer, middleware, Responder, get, HttpResponse};
use spdlog::info;

mod internal;
mod structs;

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
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