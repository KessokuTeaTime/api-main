use actix_web::{HttpRequest, HttpResponse, Responder, get};
use spdlog::debug;

#[get("/health")]
pub async fn get(req: HttpRequest) -> impl Responder {
    debug!(
        "Huston, good to hear from {}!",
        req.connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown ip address")
    );
    HttpResponse::Ok().finish()
}
