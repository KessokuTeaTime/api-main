//! Endpoint `/internal/website/deploy`.

use api_framework::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext},
        unwrap,
    },
    static_lazy_lock,
};

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use docker_wrapper::{
    DockerCommand as _,
    command::{ComposeCommand as _, compose_up::ComposeUpCommand},
};
use serde::Deserialize;
use std::fmt::Display;

use crate::env::DOCKER_COMPOSE_FILE;

static_lazy_lock! {
    QUEUED_ASYNC: QueuedAsyncFramework<PostPayloadDestination> = QueuedAsyncFramework::new();
}

/// The type-safe possible destinations of the post.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Deserialize)]
pub enum PostPayloadDestination {
    /// The website destination.
    #[serde(rename(deserialize = "www"))]
    Website,
}

impl Display for PostPayloadDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PostPayloadDestination::Website => "website (www)",
            }
        )
    }
}

/// The payload of the post.
#[derive(Debug, Clone, Deserialize)]
pub struct PostPayload {
    image: String,
    target: PostPayloadDestination,
}

unsafe impl Send for PostPayload {}

/// The client posted a website deployment request.
/// Responds with [`StatusCode::OK`] right after the deployment is triggered.
///
/// See: [`PostPayload`], [`post_transaction`]
pub async fn post(Json(payload): Json<PostPayload>) -> impl IntoResponse {
    tokio::spawn(QUEUED_ASYNC.run(payload.target, move |cx| {
        Box::pin(post_transaction(cx.clone(), payload.clone()))
    }));

    StatusCode::OK
}

async fn post_transaction(cx: QueuedAsyncFrameworkContext, payload: PostPayload) -> State<()> {
    unwrap!(cx.check());

    match ComposeUpCommand::new()
        .file(&*DOCKER_COMPOSE_FILE)
        .service(payload.target.to_string())
        .detach()
        .execute()
        .await
    {
        Ok(_) => State::Success(()),
        Err(e) => {
            tracing::error!("failed to update the service: {e:?}");
            State::Retry
        }
    }
}
