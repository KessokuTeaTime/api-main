use axum::{
    Router,
    middleware::from_fn,
    routing::{get, post},
};

use crate::middleware::auth::ktt_api_key_authorization_layer;

mod health;
mod internal;

pub fn route_from(mut app: Router) -> Router {
    app = route_gets(app);
    app = route_posts(app);
    app.layer(from_fn(crate::middleware::logger::log_request))
}

fn route_gets(app: Router) -> Router {
    app.route("/health", get(health::get))
}

fn route_posts(app: Router) -> Router {
    app.route(
        "/internal/website/deploy",
        post(internal::website::deploy::post).route_layer(ktt_api_key_authorization_layer()),
    )
}
