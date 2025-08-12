//! The API endpoints.

use axum::{
    Router,
    middleware::from_fn,
    routing::{get, post},
};

use crate::middleware::{auth::layers::kessoku_private_ci_authorization, logging::log_request};

pub mod health;
pub mod internal;

/// Routes an [`Router`] with the endpoints defined by this module.
pub fn route_from(app: Router) -> Router {
    app.route("/health", get(health::get))
        .route(
            "/internal/update",
            post(internal::update::post).route_layer(kessoku_private_ci_authorization()),
        )
        .route(
            "/internal/website/deploy",
            post(internal::website::deploy::post).route_layer(kessoku_private_ci_authorization()),
        )
        .layer(from_fn(log_request))
}
