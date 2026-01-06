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
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::transactions;

static_lazy_lock! {
    QUEUED_ASYNC: QueuedAsyncFramework<PostPayloadDestination> = QueuedAsyncFramework::new();
}

/// The type-safe possible destinations of the post.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PostPayloadDestination {
    /// The websitw.
    Website,
    /// The caledar.
    Calendar,
}

impl<'de> Deserialize<'de> for PostPayloadDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "www" => Ok(Self::Website),
            "calendar" => Ok(Self::Calendar),
            _ => {
                tracing::error!("unknown deployment destination: {}", s);
                Err(serde::de::Error::custom(format!(
                    "unknown destination: {s}",
                )))
            }
        }
    }
}

impl Serialize for PostPayloadDestination {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Website => serializer.serialize_str("www"),
            Self::Calendar => serializer.serialize_str("calendar"),
        }
    }
}

impl Display for PostPayloadDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Website => "website (www)",
                Self::Calendar => "calendar (calendar)",
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
        Box::pin(post_transaction(cx, payload.clone()))
    }));

    StatusCode::OK
}

async fn post_transaction(cx: QueuedAsyncFrameworkContext, payload: PostPayload) -> State<()> {
    unwrap!(cx.check());

    match transactions::docker::pull_image(&payload.image).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to deploy {}: {e:?}", &payload.target);
            return State::Retry;
        }
    }

    unwrap!(cx.check());

    match transactions::docker::compose_up(payload.target.to_string().as_str()).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to deploy {}: {e:?}", &payload.target);
            return State::Retry;
        }
    }

    State::Success(())
}
