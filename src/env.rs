use std::env;

macro_rules! static_lazy_lock {
    ($name:ident: $type:ty = $expr:expr $(; $doc:expr)?) => {
        $(#[doc=$doc])?
        pub static $name: std::sync::LazyLock<$type> =
            std::sync::LazyLock::new(|| $expr);
    };
}

static_lazy_lock!(
    KTT_API_USERNAME: String = env::var("KTT_API_USERNAME").expect("KTT_API_USERNAME not set in environment");
    "The username of the API key."
);
static_lazy_lock!(
    KTT_API_PASSWORD: String = env::var("KTT_API_PASSWORD").expect("KTT_API_PASSWORD not set in environment");
    "The password of the API key."
);
static_lazy_lock!(
    GITHUB_TOKEN: String = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment")
);
static_lazy_lock!(
    DIR_TRACING: String = env::var("DIR_TRACING").unwrap_or("/tmp".to_owned());
    "The directory for tracing files. Defaults to `/tmp` if not specified."
);
