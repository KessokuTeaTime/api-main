//! Middleware for authorization.

/// Router layers for authorization.
pub mod layers {
    use crate::env::{KTT_API_PASSWORD, KTT_API_USERNAME};

    use api_framework::static_lazy_lock;
    use tower_http::auth::AddAuthorizationLayer;

    static_lazy_lock! {
        /// The layer that authorizes requests with the KessokuTeaTime private CI key in Base 64 format.
        ///
        /// See: [`KTT_API_USERNAME`], [`KTT_API_PASSWORD`], [`AddAuthorizationLayer`]
        pub KESSOKU_PRIVATE_CI_AUTHORIZATION: AddAuthorizationLayer = AddAuthorizationLayer::basic(&KTT_API_USERNAME, &KTT_API_PASSWORD);
    }
}
