//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![allow(clippy::future_not_send)]

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tracing::{info, trace};

use crate::env::PORT;

pub mod env;
pub mod framework;
pub mod fs;
pub mod logging;
pub mod workflow;

pub mod endpoint;
pub mod middleware;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    logging::setup().unwrap();

    trace!("loaded environment: {:#?}", std::env::vars());
    info!("starting server on port {}", *PORT);

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT))
        .await
        .unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// A shorthand to define a statically allocated variable using a [`std::sync::LazyLock`].
///
/// # Examples
///
/// ```rust
/// static_lazy_lock!{
///     pub VAR_1: String = String::from("a static variable");
/// }
/// // ...equals to...
/// pub static VAR_2: LazyLock<String> = LazyLock::new(|| String::from("a static variable"));
/// ```
#[macro_export]
macro_rules! static_lazy_lock {
    ($vis:vis $name:ident: $type:ty = $expr:expr; $($doc:expr)?) => {
        $(#[doc=$doc])?
        $vis static $name: std::sync::LazyLock<$type> =
            std::sync::LazyLock::new(|| $expr);
    };
}
