use crate::{KTT_API_PASSWORD, KTT_API_USERNAME};
use tower_http::auth::AddAuthorizationLayer;

pub(crate) fn ktt_api_key_authorization_layer() -> AddAuthorizationLayer {
    AddAuthorizationLayer::basic(&KTT_API_USERNAME, &KTT_API_PASSWORD)
}
