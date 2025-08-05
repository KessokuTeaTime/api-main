use crate::state::{State, retry_if_possible};
use crate::workflow::artifact::{download_artifact, fetch_artifact};

use async_zip::base::read::stream::ZipFileReader;
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use futures::stream::TryStreamExt as _;
use parking_lot::Mutex;
use sha2::Digest as _;
use tokio_util::io::StreamReader;

use serde::Deserialize;
use spdlog::{debug, error, info, warn};

use std::{
    fmt::Display,
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    {io, thread},
};

/// The pending [`Payload`] to be deployed.
static PENDING_PAYLOAD: LazyLock<Mutex<Option<Payload>>> = LazyLock::new(|| Mutex::new(None));
/// Indicates whether a worker thread is currently running.
static IS_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

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
    match IS_THREAD_RUNNING.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => {
            // TODO: worker threads may no longer be required as we are using async to fetch res for now
            // Spawns a new worker thread
            debug!("Spawning a thread with {payload:?}");
            thread::spawn(move || deploy(payload.clone().validate()));
        }
        Err(_) => {
            // Suspends the latest request
            let mut guard = PENDING_PAYLOAD.lock();
            match guard.replace(payload.clone()) {
                None => {
                    warn!("A thread is already running! Suspending deployment with {payload:?}");
                }
                Some(old_payload) => {
                    warn!(
                        "A thread is already running! Suspending deployment with {payload:?}, replacing {old_payload:?}"
                    );
                }
            }
        }
    }

    StatusCode::OK
}

/// Deploys the website with a [`Payload`].
async fn deploy(mut payload: Payload) {
    'worker_loop: loop {
        let mut retry: u8 = 0;

        'artifact_loop: loop {
            // Fetches the artifact
            let artifact = match fetch_artifact("KessokuTeaTime", "website", &payload.run_id).await
            {
                State::Success(artifact) => {
                    info!("Fetched artifact with {payload:?}");
                    artifact
                }
                State::Retry => {
                    error!("Failed to fetch artifact with {payload:?}");
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
                    info!("Downloading artifact with {payload:?} ..");
                    stream
                }
                State::Retry => {
                    error!("Failed to start download artifact with {payload:?}");
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
            match crate::fs::extract_archive(zip_reader, &path, true).await {
                Ok(_) => {
                    if hex::encode(sha_hasher.finalize()) != digest.unwrap()[7..] {
                        error!("Failed to match artifact's hash");
                        cleanup.await;
                        break 'artifact_loop;
                    }
                    info!(
                        "Successfully deployed to {} with {}!",
                        payload.dest, payload.run_id
                    );
                }
                Err(err) => {
                    error!("Failed to extract destination archive with {payload:?}: {err}");
                    cleanup.await;
                }
            }
            break 'artifact_loop;
        }

        {
            let mut guard = PENDING_PAYLOAD.lock();
            match guard.take() {
                None => break 'worker_loop,
                Some(pending_payload) => {
                    info!("Resolving pending deployment: {pending_payload:?}");
                    payload = pending_payload;
                }
            }
        }
    }

    IS_THREAD_RUNNING.store(false, Ordering::Release);
}
