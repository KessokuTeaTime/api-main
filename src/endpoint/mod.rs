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
pub fn route_from(mut app: Router) -> Router {
    app = route_gets(app);
    app = route_posts(app);
    app.layer(from_fn(log_request))
}

fn route_gets(app: Router) -> Router {
    app.route("/health", get(health::get))
}

fn route_posts(app: Router) -> Router {
    app.route(
        "/internal/website/deploy",
        post(internal::website::deploy::post).route_layer(kessoku_private_ci_authorization()),
    )
    .route(
        "/internal/update/main",
        post(internal::update::post).route_layer(kessoku_private_ci_authorization()),
    )
}
