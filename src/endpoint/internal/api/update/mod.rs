//! Endpoint `/internal/api/update`.

use std::fs;

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

#[derive(Debug, Clone, Deserialize)]
pub struct Payload {
    pub run_id: String,
}
