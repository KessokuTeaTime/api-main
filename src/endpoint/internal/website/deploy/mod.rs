use crate::framework::{
    queued_async::QueuedAsyncFramework,
    state::State,
    transaction::{Transaction, transaction},
};

use axum::{extract::Json, http::StatusCode, response::IntoResponse};

use serde::Deserialize;

use std::{fmt::Display, sync::LazyLock};

mod cleanup;
mod download_artifact;
mod fetch_artifact;

static FRAMEWORK: LazyLock<QueuedAsyncFramework<'static, String, Payload>> = LazyLock::new(|| {
    QueuedAsyncFramework::new(|cx| {
        Transaction::create(transaction! {
            |input: fetch_artifact::Input| -> State<fetch_artifact::Output>;
            cx => await fetch_artifact::run
        })
        .and_then(cx.check_transaction())
        .map_next(transaction! {
            |input: download_artifact::Input| -> State<download_artifact::Output>;
            cx => await download_artifact::run
        })
        .map_next(transaction! {
            |input: cleanup::Input| -> State<cleanup::Output>;
            cx => await cleanup::run
        })
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
