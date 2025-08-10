//! Endpoint `/internal/api/update`.

use crate::{
    SHUTDOWN, ShutdownAction,
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext, unwrap},
        transactions::download_and_extract,
    },
    static_lazy_lock,
    workflow::artifact::fetch_artifact,
};

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

static_lazy_lock! {
    FRAMEWORK: QueuedAsyncFramework<String> = QueuedAsyncFramework::new();
}

/// The client posted an api update request.
/// Responds with [`StatusCode::OK`] right after the deployment is triggered.
///
/// See: [`Payload`], [transaction]
pub async fn post(Json(payload): Json<Payload>) -> impl IntoResponse {
    tokio::spawn(FRAMEWORK.run(payload.run_id.clone(), move |cx| {
        Box::pin(transaction(cx.clone(), payload.clone()))
    }));

    StatusCode::OK
}

async fn transaction(cx: QueuedAsyncFrameworkContext, payload: Payload) -> State<()> {
    let artifact = unwrap!(fetch_artifact("KessokuTeaTime", "api", &payload.run_id).await);
    unwrap!(cx.check());

    let path = "./update";
    unwrap!(download_and_extract(artifact, path).await);

    drop(SHUTDOWN.get().unwrap().send(ShutdownAction::Update {
        binary_path: format!("{}/{}", path, "api"),
    }));

    State::Success(())
}

/// The payload of the post.
#[derive(Debug, Clone, Deserialize)]
pub struct Payload {
    /// The run id of the GitHub workflow.
    pub run_id: String,
}
