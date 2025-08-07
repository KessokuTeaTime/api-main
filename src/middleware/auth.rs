pub mod layers {
    use crate::env::{KTT_API_PASSWORD, KTT_API_USERNAME};
    use tower_http::auth::AddAuthorizationLayer;

    pub fn ktt_api_key_authorization() -> AddAuthorizationLayer {
        AddAuthorizationLayer::basic(&KTT_API_USERNAME, &KTT_API_PASSWORD)
    }
}
