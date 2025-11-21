//! Endpoint `/internal/update`.

use api_framework::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext},
        unwrap,
    },
    static_lazy_lock,
};

use axum::{Json, http::StatusCode, response::IntoResponse};
use docker_wrapper::{
    DockerCommand, PullCommand,
    command::{ComposeCommand, compose_up::ComposeUpCommand},
};
use serde::Deserialize;

use crate::env::{DOCKER_COMPOSE_FILE, DOCKER_CONTAINER_NAME};

static_lazy_lock! {
    QUEUED_ASYNC: QueuedAsyncFramework<String> = QueuedAsyncFramework::new();
}

/// The payload of the post.
#[derive(Debug, Clone, Deserialize)]
pub struct PostPayload {
    image: String,
}

/// The client posted an api update request.
/// Responds with [`StatusCode::OK`] right after the deployment is triggered.
///
/// See: [`PostPayload`], [`post_transaction`]
pub async fn post(Json(payload): Json<PostPayload>) -> impl IntoResponse {
    tokio::spawn(QUEUED_ASYNC.run(payload.image.clone(), move |cx| {
        Box::pin(post_transaction(cx.clone(), payload.clone()))
    }));

    StatusCode::OK
}

async fn post_transaction(cx: QueuedAsyncFrameworkContext, payload: PostPayload) -> State<()> {
    unwrap!(cx.check());
    unwrap!(cx.check());

    match PullCommand::new(payload.image.clone()).execute().await {
        Ok(_) => {
            tracing::info!("successfully pulled image {}", &payload.image);
        }
        Err(e) => {
            tracing::error!("failed to pull image {}: {e:?}", &payload.image);
            return State::Retry;
        }
    }

    unwrap!(cx.check());

    match ComposeUpCommand::new()
        .file(&*DOCKER_COMPOSE_FILE)
        .project_name(&*DOCKER_CONTAINER_NAME)
        .detach()
        .execute()
        .await
    {
        Ok(_) => {
            tracing::info!("successfully uped container {}", &*DOCKER_CONTAINER_NAME);
        }
        Err(e) => {
            tracing::error!("failed to up container {}: {e:?}", &*DOCKER_CONTAINER_NAME);
            return State::Retry;
        }
    }

    State::Success(())
}
