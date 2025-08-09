//! Endpoint `/internal/api/update`.

use crate::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext, unwrap},
        transactions::download_and_extract,
    },
    service::{self, DrainingReason},
    static_lazy_lock,
    workflow::artifact::fetch_artifact,
};

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

static_lazy_lock! {
    FRAMEWORK: QueuedAsyncFramework<String> = QueuedAsyncFramework::new();
}

pub async fn post(Json(payload): Json<Payload>) -> impl IntoResponse {
    match service::check() {
        Err(response) => response,
        Ok(_) => {
            tokio::spawn(FRAMEWORK.run(payload.run_id.clone(), move |cx| {
                Box::pin(transaction(cx.clone(), payload.clone()))
            }));

            StatusCode::OK.into_response()
        }
    }
}

async fn transaction(cx: QueuedAsyncFrameworkContext, payload: Payload) -> State<()> {
    service::drain(DrainingReason::Updating);

    let artifact = unwrap!(fetch_artifact("KessokuTeaTime", "api", &payload.run_id).await);
    unwrap!(cx.check());

    let path = "./update";
    unwrap!(download_and_extract(artifact, path).await);

    State::Success(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Payload {
    pub run_id: String,
}
