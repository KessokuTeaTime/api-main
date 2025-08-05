use axum::{Router, routing::get};

pub mod health;

pub fn route_from(app: Router) -> Router {
    app.route("/health", get(health::get))
}
