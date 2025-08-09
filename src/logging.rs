//! The logging component of the server.

use anyhow::Error;
use tracing::level_filters::LevelFilter;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    Layer, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::env::{TRACING_DIR, TRACING_MAX_FILES};

/// Setup the logging component, which contains a stderr layer, a rolling file layer, and a latest file layer.
///
/// # Errors
///
/// Returns an error if the setup progress failed.
///
/// See: [`TRACING_DIR`], [`TRACING_MAX_FILES`], [`tracing`]
pub fn setup() -> Result<(), Error> {
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_writer(std::io::stdout);

    let rolling_file_layer = tracing_subscriber::fmt::layer().with_writer(
        RollingFileAppender::builder()
            .filename_suffix("log")
            .rotation(Rotation::DAILY)
            .max_log_files(*TRACING_MAX_FILES)
            .build(&*TRACING_DIR)?,
    );

    let latest_file_layer = tracing_subscriber::fmt::layer().with_writer(
        RollingFileAppender::builder()
            .filename_prefix("latest")
            .filename_suffix("log")
            .rotation(Rotation::DAILY)
            .max_log_files(1)
            .build(&*TRACING_DIR)?,
    );

    tracing_subscriber::registry()
        .with(
            stderr_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_filter(LevelFilter::DEBUG),
        )
        .with(
            rolling_file_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_ansi(false)
                .with_filter(LevelFilter::TRACE),
        )
        .with(
            latest_file_layer
                .with_timer(ChronoLocal::rfc_3339())
                .with_ansi(false)
                .with_filter(LevelFilter::TRACE),
        )
        .try_init()?;

    Ok(())
}
