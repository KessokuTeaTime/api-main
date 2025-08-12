//! Endpoint `/internal/api/update`.

use api_framework::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext, unwrap},
    },
    shutdown::{SHUTDOWN, ShutdownAction},
    static_lazy_lock,
    transactions::download_and_extract_archive,
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
    unwrap!(download_and_extract_archive(artifact, path).await);

    drop(SHUTDOWN.send(ShutdownAction::Update {
        executable_path: format!("{}/{}", path, "api"),
    }));

    State::Success(())
}

/// The payload of the post.
#[derive(Debug, Clone, Deserialize)]
pub struct Payload {
    /// The run id of the GitHub workflow.
    pub run_id: String,
}
