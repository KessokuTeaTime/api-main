use crate::framework::state::{State, retry_if_possible};
use crate::workflow::artifact::{download_artifact, fetch_artifact};

use async_zip::base::read::stream::ZipFileReader;
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use futures::AsyncReadExt;
use futures::stream::TryStreamExt as _;
use parking_lot::Mutex;
use serde::Deserialize;
use sha2::Digest as _;
use tracing::{debug, error, info, warn};

use std::{
    collections::HashMap,
    fmt::Display,
    io,
    path::PathBuf,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(Default)]
struct BusinessHolder {
    lock: tokio::sync::Mutex<()>,
    latest_payload_index: AtomicU8,
}

static FS_BUSINESSES: LazyLock<Mutex<HashMap<PathBuf, Arc<BusinessHolder>>>> =
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
    let path = format!("/var/{}/html", &payload.dest);

    let holder = FS_BUSINESSES
        .lock()
        .entry(PathBuf::from(&path))
        .or_default()
        .clone();
    let index = holder.latest_payload_index.fetch_add(1, Ordering::SeqCst); // Acquires the index before waiting for the lock
    let _fs_guard = holder.lock.lock().await;

    let cleanup = async |succeed: bool| {
        if succeed {
            debug!("setting the latest payload index to 0…");
            holder.latest_payload_index.store(u8::MIN, Ordering::SeqCst);
            drop(_fs_guard);
        } else {
            drop(tokio::fs::remove_dir_all(&path).await);
        }
    };
    let should_exit = || {
        let latest_payload_index = holder.latest_payload_index.load(Ordering::SeqCst);
        let result = index < latest_payload_index - 1;
        if result {
            warn!(
                "current payload index ({index}) is falling behind the latest one ({latest_payload_index}), exiting deployment with {payload}!"
            );
        }
        result
    };

    'artifact_loop: loop {
        if should_exit() {
            break 'artifact_loop;
        };

        // Fetches the artifact
        let artifact = match fetch_artifact("KessokuTeaTime", "website", &payload.run_id).await {
            State::Success(artifact) => {
                info!("fetched artifact with {payload}");
                artifact
            }
            State::Retry => {
                error!("failed to fetch artifact with {payload}");
                match retry_if_possible(&mut retry) {
                    Ok(_) => continue 'artifact_loop,
                    Err(_) => break 'artifact_loop,
                }
            }
            State::Stop => break 'artifact_loop,
        };

        if should_exit() {
            break 'artifact_loop;
        }

        // Downloads the artifact
        let digest = artifact.digest.clone();
        let stream = match download_artifact(artifact).await {
            State::Success(stream) => {
                info!("downloaded artifact with {payload}");
                stream
            }
            State::Retry => {
                error!("failed to download artifact with {payload}");
                match retry_if_possible(&mut retry) {
                    Ok(_) => continue 'artifact_loop,
                    Err(_) => break 'artifact_loop,
                }
            }
            State::Stop => break 'artifact_loop,
        };

        if should_exit() {
            break 'artifact_loop;
        }

        // Extracts the archive
        let mut sha_hasher = sha2::Sha256::new();
        let mut read = stream
            .map_ok(|bytes| {
                sha_hasher.update(&bytes);
                bytes
            })
            .map_err(io::Error::other)
            .into_async_read();
        match crate::fs::extract_archive(ZipFileReader::new(&mut read), &path).await {
            Ok(_) => {
                // read to end for consuming whole buf to hasher, neglecting error
                drop(read.read_to_end(&mut Vec::new()).await);

                if hex::encode(sha_hasher.finalize()) == digest.unwrap()[7..] {
                    info!(
                        "successfully deployed to {} with {}!",
                        payload.dest, payload.run_id
                    );
                    cleanup(true).await;
                } else {
                    error!("failed to match artifact's hash");
                    cleanup(false).await;
                }
            }
            Err(err) => {
                error!("failed to extract destination archive with {payload}: {err}");
                cleanup(false).await;
            }
        }

        break 'artifact_loop;
    }
}
