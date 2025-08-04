use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, middleware};
use spdlog::{debug, info};

mod internal;
mod structs;

const PORT: u16 = 8086;

#[get("/health")]
async fn health(req: HttpRequest) -> impl Responder {
    debug!(
        "Huston, good to hear from {}!",
        req.connection_info().host()
    );
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);
    info!("Starting server on port {PORT}");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(internal::notify)
            .service(health)
    })
    .bind(format!("0.0.0.0:{PORT}"))?
    .run()
    .await
}
