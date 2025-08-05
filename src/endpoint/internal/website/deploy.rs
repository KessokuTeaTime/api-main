use crate::state::{State, retry_if_possible};
use crate::workflow::artifact::{download_artifact, fetch_artifact};

use async_zip::base::read::stream::ZipFileReader;
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use futures::stream::TryStreamExt as _;
use parking_lot::Mutex;
use serde::Deserialize;
use sha2::Digest as _;
use tokio_util::io::StreamReader;
use tracing::{error, info};

use std::{
    collections::HashMap,
    fmt::Display,
    io,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

const TRACING_REALM: &str = "[ENDPOINT] [POST /internal/website/deploy]";

static FS_BUSINESS: LazyLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
}

/// Responds to a website deployment request
pub async fn post(Json(payload): Json<Payload>) -> impl IntoResponse {
    tokio::spawn(deploy(payload.clone().validate()));
    StatusCode::OK
}

/// Deploys the website with a [`Payload`].
async fn deploy(payload: Payload) {
    let mut retry: u8 = 0;

    'artifact_loop: loop {
        // Fetches the artifact
        let artifact = match fetch_artifact("KessokuTeaTime", "website", &payload.run_id).await {
            State::Success(artifact) => {
                info!("{TRACING_REALM} Fetched artifact with {payload:?}");
                artifact
            }
            State::Retry => {
                error!("{TRACING_REALM} Failed to fetch artifact with {payload:?}");
                match retry_if_possible(&mut retry) {
                    Ok(_) => continue 'artifact_loop,
                    Err(_) => break 'artifact_loop,
                }
            }
            State::Stop => break 'artifact_loop,
        };
        let digest = artifact.digest.clone();

        // Downloads the artifact
        let stream = match download_artifact(artifact).await {
            State::Success(stream) => {
                info!("{TRACING_REALM} Downloading artifact with {payload:?} ..");
                stream
            }
            State::Retry => {
                error!("{TRACING_REALM} Failed to start download artifact with {payload:?}");
                match retry_if_possible(&mut retry) {
                    Ok(_) => continue 'artifact_loop,
                    Err(_) => break 'artifact_loop,
                }
            }
            State::Stop => break 'artifact_loop,
        };

        let mut sha_hasher = sha2::Sha256::new();
        let zip_reader = ZipFileReader::with_tokio(StreamReader::new(
            stream
                .map_ok(|bytes| {
                    sha_hasher.update(&bytes);
                    bytes
                })
                .map_err(io::Error::other),
        ));
        let path = format!("/var/{}/html", &payload.dest);
        let cleanup = async {
            drop(tokio::fs::remove_dir_all(&path).await);
        };

        let lock = FS_BUSINESS
            .lock()
            .entry(PathBuf::from(&path))
            .or_default()
            .clone();

        let _fs_guard = lock.lock().await;

        match crate::fs::extract_archive(zip_reader, &path, true).await {
            Ok(_) => {
                if hex::encode(sha_hasher.finalize()) != digest.unwrap()[7..] {
                    error!("{TRACING_REALM} Failed to match artifact's hash");
                    cleanup.await;
                    break 'artifact_loop;
                }
                info!(
                    "{TRACING_REALM} Successfully deployed to {} with {}!",
                    payload.dest, payload.run_id
                );
            }
            Err(err) => {
                error!(
                    "{TRACING_REALM} Failed to extract destination archive with {payload:?}: {err}"
                );
                cleanup.await;
            }
        }
        break 'artifact_loop;
    }
}
