use std::env;

use anyhow::anyhow;

macro_rules! static_lazy_lock {
    ($name:ident: $type:ty = $expr:expr $(; $doc:expr)?) => {
        $(#[doc=$doc])?
        pub static $name: std::sync::LazyLock<$type> =
            std::sync::LazyLock::new(|| $expr);
    };
}

static_lazy_lock!(
    PORT: u16 = env::var("PORT").map_err(|e| anyhow!(e)).and_then(|s| s.parse::<u16>().map_err(|e| anyhow!(e))).expect("PORT not set in environment");
    "The port to listen to."
);
static_lazy_lock!(
    KTT_API_USERNAME: String = env::var("KTT_API_USERNAME").expect("KTT_API_USERNAME not set in environment");
    "The username of the API key."
);
static_lazy_lock!(
    KTT_API_PASSWORD: String = env::var("KTT_API_PASSWORD").expect("KTT_API_PASSWORD not set in environment");
    "The password of the API key."
);
static_lazy_lock!(
    GITHUB_TOKEN: String = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment");
    "The GitHub token."
);
static_lazy_lock!(
    MAX_RETRY: u8 = env::var("MAX_RETRY").map_err(|e| anyhow!(e)).and_then(|s| s.parse::<u8>().map_err(|e| anyhow!(e))).unwrap_or(5);
    "The maximum retry limit for transactions."
);
static_lazy_lock!(
    TRACING_MAX_FILES: usize = env::var("TRACING_MAX_FILES").map_err(|e| anyhow!(e)).and_then(|s| s.parse::<usize>().map_err(|e| anyhow!(e))).unwrap_or(5);
    "The maximum file count to use for tracing."
);
static_lazy_lock!(
    TRACING_DIR: String = env::var("TRACING_DIR").unwrap_or("/tmp/api/tracing".to_owned());
    "The directory for tracing files. Defaults to `/tmp/api/tracing` if not specified."
);
