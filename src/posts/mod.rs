use axum::{Router, routing::post};

pub mod internal;

pub fn route_from(app: Router) -> Router {
    app.route(
        "/internal/website/deploy",
        post(internal::website::deploy::post),
    )
}
