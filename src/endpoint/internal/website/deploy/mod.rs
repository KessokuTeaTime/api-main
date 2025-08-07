use crate::framework::{
    queued_async::{QueuedAsyncFramework, QueuedAsyncFrameworkContext},
    state::State,
    transaction::{
        Transaction,
        global::{download_artifact, fetch_artifact},
        transaction,
    },
};

use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::{fmt::Display, sync::LazyLock};

mod fetch_to_download;

static FRAMEWORK: LazyLock<QueuedAsyncFramework<'static, String, Payload>> = LazyLock::new(|| {
    QueuedAsyncFramework::new(|cx| {
        Transaction::create(transaction! {
            |input: fetch_artifact::Input<Payload>| -> State<fetch_artifact::Output<Payload>>;
            cx => await fetch_artifact::run::<QueuedAsyncFrameworkContext, Payload>
        })
        .and_then(cx.check_transaction())
        .map_next(transaction! {
            |input: fetch_to_download::Input| -> State<fetch_to_download::Output>;
            await fetch_to_download::run
        })
        .map_next(transaction! {
            |input: download_artifact::Input<Payload>| -> State<download_artifact::Output<Payload>>;
            cx => await download_artifact::run::<QueuedAsyncFrameworkContext, Payload>
        })
        .map_next_become(())
    })
});

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

impl From<Payload> for fetch_artifact::Input<Payload> {
    fn from(payload: Payload) -> Self {
        Self {
            passthrough: payload.clone(),
            run_id: payload.run_id,
        }
    }
}

impl Payload {
    pub fn validate(self) -> Self {
        Self {
            run_id: self.run_id,
            dest: self
                .dest
                .trim_matches(|c: char| c.is_whitespace() || c == '/')
                .to_owned(),
        }
    }

    pub fn path(&self) -> String {
        format!("/var/{}/html", &self.dest)
    }
}

pub async fn post(Json(payload): Json<Payload>) -> impl IntoResponse {
    tokio::spawn(FRAMEWORK.run(payload.clone().dest, payload.clone().validate()));
    StatusCode::OK
}
