//! The API endpoints.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use crate::middleware::auth::layers;

pub mod health;
pub mod internal;

/// Routes an [`Router`] with the endpoints defined by this module.
pub fn route_from(mut app: Router) -> Router {
    app = route_gets(app);
    app = route_posts(app);
    app.layer(TraceLayer::new_for_http())
}

fn route_gets(app: Router) -> Router {
    app.route("/health", get(health::get))
}

fn route_posts(app: Router) -> Router {
    app.route(
        "/internal/update",
        post(internal::update::post)
            .route_layer(layers::KESSOKU_PRIVATE_CI_AUTHORIZATION.to_owned()),
    )
    .route(
        "/internal/website/deploy",
        post(internal::website::deploy::post)
            .route_layer(layers::KESSOKU_PRIVATE_CI_AUTHORIZATION.to_owned()),
    )
}
