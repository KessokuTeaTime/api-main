//! Defines the environment variables to use.

use crate::static_lazy_lock;

use std::env;

pub mod info {
    pub const GIT_HASH: &str = env!("GIT_HASH");
    pub const BUILD_DATE: &str = env!("VERGEN_BUILD_DATE");
}

macro_rules! parse_env {
    ($key:expr => |$var:ident| $expr:expr) => {
        std::env::var($key)
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|$var| $expr)
    };
    ($key:expr => |$var:ident| $expr:expr; anyhow) => {
        parse_env!($key => |$var| $expr.map_err(|e| anyhow::anyhow!(e)))
    };
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
    pub GITHUB_TOKEN: String = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment");
    "The GitHub token."
}

static_lazy_lock! {
    pub MAX_RETRY: u8 = parse_env!("MAX_RETRY" => |s| s.parse::<u8>(); anyhow).unwrap_or(5);
    "The maximum retry limit for transactions."
}

static_lazy_lock! {
    pub TRACING_MAX_FILES: usize = parse_env!("TRACING_MAX_FILES" => |s| s.parse::<usize>(); anyhow).unwrap_or(5);
    "The maximum file count to use for tracing."
}

static_lazy_lock! {
    pub TRACING_DIR: String = env::var("TRACING_DIR").unwrap_or("/tmp/api/tracing".to_owned());
    "The directory for tracing files. Defaults to `/tmp/api/tracing` if not specified."
}
