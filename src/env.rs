//! Defines environment variables.

use std::{env, path::PathBuf};

use api_framework::{env::parse_env, static_lazy_lock};

/// Sets up environment variables.
pub fn setup() {
    dotenvy::dotenv().ok();
    dotenvy::from_filename_override(format!("{}.env", clap::crate_name!())).ok();
}

/// The info generated during build.
pub mod info {
    /// The latest Git commit hash.
    pub const GIT_HASH: &str = env!("GIT_HASH");
    /// The build timestamp.
    pub const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");
}

static_lazy_lock! {
    pub PORT: u16 = parse_env!("PORT" => |s| s.parse::<u16>(); anyhow).expect("PORT not set in environment");
    "The port to listen to."
}

static_lazy_lock! {
    pub KTT_API_USERNAME: String = env::var("KTT_API_USERNAME").expect("KTT_API_USERNAME not set in environment");
    "The username of the API key."
}

static_lazy_lock! {
    pub KTT_API_PASSWORD: String = env::var("KTT_API_PASSWORD").expect("KTT_API_PASSWORD not set in environment");
    "The password of the API key."
}

static_lazy_lock! {
    pub TRACING_MAX_FILES: usize = parse_env!("TRACING_MAX_FILES" => |s| s.parse::<usize>(); anyhow).unwrap_or(5);
    "The maximum file count to use for tracing."
}

static_lazy_lock! {
    pub TRACING_DIR: PathBuf = parse_env!("TRACING_DIR" => |s| Ok(PathBuf::from(s))).unwrap_or(PathBuf::from("/tmp/tracing")).join(clap::crate_name!());
    "The directory for tracing files. Defaults to `/tmp/tracing/main` if not specified."
}

static_lazy_lock! {
    pub DOCKER_CONTAINER_NAME: String = env::var("DOCKER_CONTAINER_NAME").unwrap_or_else(|_| format!("api-{}", clap::crate_name!()));
    "The name of the Docker container that runs this service."
}

static_lazy_lock! {
    pub DOCKER_COMPOSE_FILE: PathBuf = parse_env!("DOCKER_COMPOSE_FILE" => |s| Ok(PathBuf::from(s))).unwrap();
    "The path to the Docker Compose file."
}
