//! Endpoint `/internal/website/deploy`.

use api_framework::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext},
        unwrap,
    },
    static_lazy_lock,
    transactions::download_and_extract_archive,
    workflow::artifact::fetch_artifact,
};

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::fmt::Display;

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

impl PostPayloadDestination {
    /// Returns the path of the destination. Often at `/var/{slug}/html`.
    pub fn path(&self) -> String {
        let slug = match &self {
            Self::Website => "www",
        };
        format!("/var/{slug}/html")
    }
}

impl Display for PostPayloadDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.path())
    }
}

/// The payload of the post.
#[derive(Debug, Clone, Deserialize)]
pub struct PostPayload {
    run_id: String,
    dest: PostPayloadDestination,
}

impl Display for PostPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <~ {}", self.dest, self.run_id)
    }
}

unsafe impl Send for PostPayload {}

/// The client posted a website deployment request.
/// Responds with [`StatusCode::OK`] right after the deployment is triggered.
///
/// See: [`PostPayload`], [`post_transaction`]
pub async fn post(Json(payload): Json<PostPayload>) -> impl IntoResponse {
    tokio::spawn(
        QUEUED_ASYNC.run_with_name(payload.dest, format!("{}", &payload), move |cx| {
            Box::pin(post_transaction(cx.clone(), payload.clone()))
        }),
    );

    StatusCode::OK
}

async fn post_transaction(cx: QueuedAsyncFrameworkContext, payload: PostPayload) -> State<()> {
    let artifact = unwrap!(fetch_artifact("KessokuTeaTime", "website", &payload.run_id).await);
    unwrap!(cx.check());
    unwrap!(download_and_extract_archive(artifact, &payload.dest.path()).await);
    State::Success(())
}
