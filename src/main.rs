//! KessokuTeaTime API backend at `api.kessokuteatime.work`.

#![allow(clippy::future_not_send)]

use crate::env::{
    PORT,
    info::{BUILD_DATE, GIT_HASH},
};

use std::net::SocketAddr;

use axum::Router;
use tokio::{net::TcpListener, sync::broadcast};
use tracing::{debug, info, trace};

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
    debug!("binary compiled at {BUILD_DATE} from commit {GIT_HASH}");
    info!("starting server on port {}", *PORT);

    let (tx, _) = broadcast::channel::<ShutdownAction>(1);
    SHUTDOWN.set(tx).unwrap();

    let mut app = Router::new();
    app = endpoint::route_from(app);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", *PORT))
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::signal())
    .await
    .unwrap();

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
