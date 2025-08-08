use crate::{
    framework::{
        State,
        queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext, unwrap},
        transactions::download_and_extract,
    },
    workflow::artifact::fetch_artifact,
};

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::{fmt::Display, sync::LazyLock};

static FRAMEWORK: LazyLock<QueuedAsyncFramework<String>> = LazyLock::new(QueuedAsyncFramework::new);

/// Responds to a website deployment request.
/// Returns [`StatusCode::OK`] right after the deployment is triggered.
///
/// See: [`transaction`]
pub async fn post(Json(payload): Json<Payload>) -> impl IntoResponse {
    let payload = payload.validate();
    tokio::spawn(FRAMEWORK.run(payload.dest.clone(), payload.clone(), |cx| {
        Box::pin(transaction(cx.clone()))
    }));
    StatusCode::OK
}

async fn transaction(cx: QueuedAsyncFrameworkContext<Payload>) -> State<()> {
    let artifact = unwrap!(fetch_artifact("KessokuTeaTime", "website", &cx.payload.run_id).await);
    unwrap!(cx.check());
    unwrap!(download_and_extract(artifact, &cx.payload.path()).await);
    State::Success(())
}

/// The payload of the post.
#[derive(Debug, Deserialize, Clone)]
pub struct Payload {
    pub run_id: String,
    pub dest: String,
}

impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}<~{}", self.dest, self.run_id))
    }
}

unsafe impl Send for Payload {}

impl Payload {
    /// Validates the [`Payload`] to normalize the destination.
    pub fn validate(self) -> Self {
        Self {
            run_id: self.run_id,
            dest: self
                .dest
                .trim_matches(|c: char| c.is_whitespace() || c == '/')
                .to_owned(),
        }
    }

    /// The path to extract the website archive.
    pub fn path(&self) -> String {
        format!("/var/{}/html", &self.dest)
    }
}
