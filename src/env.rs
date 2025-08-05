macro_rules! static_env {
    (unwrapping $name:ident $(; $doc:expr)?) => {
        $(#[doc=$doc])?
        pub static $name: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| std::env::var(stringify!($ident)).unwrap());
    };
    (expecting $name:ident $(; $doc:expr)?) => {
        static_env!(expecting $name with  concat!(stringify!($name), " not set in environment") $(; $doc)?);
    };
    (expecting $name:ident with $msg:expr $(; $doc:expr)?) => {
        $(#[doc=$doc])?
        pub static $name: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| std::env::var(stringify!($ident)).expect($msg));
    };
    (expecting $name:ident to $default:expr $(; $doc:expr)?) => {
        $(#[doc=$doc])?
        pub static $name: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| std::env::var(stringify!($ident)).unwrap_or(String::from($default)));
    };
}

pub static KTT_API_USERNAME: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    std::env::var("KTT_API_USERNAME").expect("KTT_API_USERNAME not set in environment")
});
// static_env!(expecting KTT_API_USERNAME; "The username of the API key.");
static_env!(expecting KTT_API_PASSWORD; "The password of the API key.");

static_env!(expecting GITHUB_TOKEN);

static_env!(expecting DIR_TRACING to "/tmp"; "The directory for tracing files. Defaults to `/tmp` if not specified.");
