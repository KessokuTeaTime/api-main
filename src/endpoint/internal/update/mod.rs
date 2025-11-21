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
use serde::Deserialize;

use crate::{env::DOCKER_CONTAINER_NAME, transactions};

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

    match transactions::docker::pull_image(&payload.image).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to update: {e:?}");
            return State::Retry;
        }
    }

    unwrap!(cx.check());

    match transactions::docker::compose_up(&DOCKER_CONTAINER_NAME).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to deploy {}: {e:?}", &*DOCKER_CONTAINER_NAME);
            return State::Retry;
        }
    }

    State::Success(())
}
