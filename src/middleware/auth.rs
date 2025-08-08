//! Middlewares for authorization.

/// Router layers for authorization.
pub mod layers {
    use crate::env::{KTT_API_PASSWORD, KTT_API_USERNAME};
    use tower_http::auth::AddAuthorizationLayer;

    /// The layer that authorizes requests with the KessokuTeaTime private CI key in Base 64 format.
    ///
    /// See: [`KTT_API_USERNAME`], [`KTT_API_PASSWORD`], [`AddAuthorizationLayer`]
    pub fn ktt_api_key_authorization() -> AddAuthorizationLayer {
        AddAuthorizationLayer::basic(&KTT_API_USERNAME, &KTT_API_PASSWORD)
    }
}
