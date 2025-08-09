//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![allow(clippy::future_not_send)]

use std::{net::SocketAddr, time::Duration};

use axum::Router;
use tokio::{net::TcpListener, sync::broadcast};
use tracing::{info, trace};

use crate::env::PORT;

pub mod env;
pub mod framework;
pub mod fs;
pub mod logging;
pub mod workflow;

pub mod endpoint;
pub mod middleware;

mod shutdown;

pub use shutdown::{SHUTDOWN, ShutdownAction};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    logging::setup().unwrap();

    trace!("loaded environment: {:#?}", std::env::vars());
    info!("starting server on port {}", *PORT);

    let (tx, _) = broadcast::channel::<ShutdownAction>(1);
    SHUTDOWN.set(tx).unwrap();

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT))
        .await
        .unwrap();
    let service = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::signal());

    drop(
        tokio::time::timeout(Duration::from_secs(30), service)
            .await
            .unwrap(),
    );

    info!("stopping!");
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
