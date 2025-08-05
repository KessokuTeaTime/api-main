use crate::state::{State, retry_if_possible};
use crate::workflow::artifact::{Artifact, Artifacts};
use crate::{GITHUB_TOKEN, KTT_API_KEY};
use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use async_zip::base::read::stream::ZipFileReader;
use futures::stream::TryStreamExt;
use parking_lot::Mutex;
use reqwest::RequestBuilder;
use reqwest::header;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use spdlog::{debug, error, info, warn};
use std::error::Error;
use std::fmt::Display;
use std::io::Cursor;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, io, thread};
use tokio_util::io::StreamReader;
use zip::ZipArchive;

/// The pending [`WorkflowInfo`] to be deployed.
static PENDING_INFO: LazyLock<Mutex<Option<WorkflowInfo>>> = LazyLock::new(|| Mutex::new(None));
/// Indicates whether a worker thread is currently running.
static IS_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowInfo {
    pub run_id: String,
    pub dest: String,
}

impl Display for WorkflowInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}<~{}", self.dest, self.run_id))
    }
}

impl WorkflowInfo {
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

/// Responds to a website deployment request.
#[post("/internal/website/deploy")]
async fn post(req: HttpRequest, info: web::Json<WorkflowInfo>) -> impl Responder {
    debug!(
        "Received notification from {}, authenticating…",
        req.connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
    );

    let authed = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == *KTT_API_KEY)
        .unwrap_or(false);

    if authed {
        info!(
            "Authenticated the notification from {}",
            req.connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
        );
    } else {
        error!(
            "Failed to authenticate the notification from {}",
            req.connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
        );

        return HttpResponse::NotFound().finish();
    }

    match IS_THREAD_RUNNING.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => {
            // TODO: worker threads may no longer be required as we are using async to fetch res for now
            // Spawns a new worker thread
            debug!("Spawning a thread with {info:?}");
            thread::spawn(move || deploy(info.clone().validate()));
        }
        Err(_) => {
            // Suspends the latest info
            let mut guard = PENDING_INFO.lock();
            match guard.replace(info.clone()) {
                None => {
                    warn!("A thread is already running! Suspending deployment with {info:?}");
                }
                Some(old_info) => {
                    warn!(
                        "A thread is already running! Suspending deployment with {info:?}, replacing {old_info:?}"
                    );
                }
            }
        }
    }

    HttpResponse::Ok().finish()
}

/// Builds a request for GitHub API
fn github_api_request_builder(url: String) -> RequestBuilder {
    reqwest::Client::new()
        .get(url)
        .header(header::ACCEPT, "application/vnd.github+json")
        .bearer_auth(&*GITHUB_TOKEN)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "KessokuTeaTime-API/1.0")
}

/// Fetches the artifact corresponding to a run id
async fn fetch_artifact(run_id: &str) -> State<Artifact> {
    info!("Fetching artifact…");

    let url = format!(
        "https://api.github.com/repos/KessokuTeaTime/website/actions/runs/{run_id}/artifacts"
    );

    let response = match github_api_request_builder(url).send().await {
        Ok(response) => response,
        Err(err) => {
            error!("Failed to fetch artifacts: {err}");
            return match err {
                _ if err.is_connect() || err.is_timeout() => State::Retry,
                _ => State::Stop,
            };
        }
    };

    match response.json::<Artifacts>().await {
        Ok(json) => match json.total_count {
            0 => {
                error!("Invalid workflow data: no artifacts!");
                State::Stop
            }
            1 => {
                info!("Artifact accepted");
                State::Success(json.artifacts[0].clone())
            }
            _ => {
                error!("Invalid workflow data: too many artifacts!");
                State::Stop
            }
        },
        Err(err) => {
            error!("Failed to parse data: {err}");

            if let Some(source) = err.source() {
                error!("{source}")
            }

            State::Retry
        }
    }
}

/// Extracts the archive to a specified destination.
async fn extract_archive<R>(archive: R, dest: &str) -> io::Result<()>
where
    R: tokio::io::AsyncRead,
{
    let path = format!("/var/{dest}/html");

    fs::remove_dir_all(&path)?;
    fs::create_dir(&path)?;
    // nope
    // why
    // non-seekable. leave for lunch
    archive.extract_unwrapped_root_dir(&path, |_| true)?;
    Ok(())
}

/// Deploys the website with a [`WorkflowInfo`].
async fn deploy(info: WorkflowInfo) {
    let mut info = info;

    'worker_loop: loop {
        let mut retry: u8 = 0;

        'artifact_loop: loop {
            // Fetches the artifact
            let artifact = match fetch_artifact(&info.run_id).await {
                State::Retry => {
                    error!("Failed to fetch artifact with {info:?}");
                    match retry_if_possible(&mut retry) {
                        Ok(_) => continue 'artifact_loop,
                        Err(_) => break 'artifact_loop,
                    }
                }
                State::Stop => break 'artifact_loop,
                State::Success(artifact) => {
                    info!("Fetched artifact with {info:?}");
                    artifact
                }
            };

            // Downloads the artifact
            let stream = match github_api_request_builder(artifact.archive_download_url)
                .send()
                .await
            {
                Ok(resp) => {
                    info!("Downloaded artifact with {info:?}");
                    resp.bytes_stream()
                }
                Err(err) => match err.status() {
                    Some(reqwest::StatusCode::GONE) => {
                        error!(
                            "Failed to download artifact with {info:?}: artifact expired or removed"
                        );
                        break 'artifact_loop;
                    }
                    _ => {
                        error!("Failed to download artifact with {info:?}");
                        match retry_if_possible(&mut retry) {
                            Ok(_) => continue 'artifact_loop,
                            Err(_) => break 'artifact_loop,
                        }
                    }
                },
            };

            // if hex::encode(Sha256::digest(&bytes)) != artifact.digest.unwrap()[7..] {
            //     error!("Failed to match artifact's hash");
            //     match retry_if_possible(&mut retry) {
            //         Ok(_) => continue 'artifact_loop,
            //         Err(_) => break 'artifact_loop,
            //     }
            // };

            let zip_reader = ZipFileReader::with_tokio(StreamReader::new(
                stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err)),
            ));
            match extract_archive(&mut archive, &info.dest) {
                Ok(_) => {
                    info!(
                        "Successfully deployed to {} with {}!",
                        info.dest, info.run_id
                    )
                }
                Err(err) => {
                    error!("Failed to extract destination archive with {info:?}: {err}");
                }
            }

            break 'artifact_loop;
        }

        {
            let mut guard = PENDING_INFO.lock();
            match guard.take() {
                None => break 'worker_loop,
                Some(pending_info) => {
                    info!("Resolving pending deployment: {pending_info:?}");
                    info = pending_info;
                }
            }
        }
    }

    IS_THREAD_RUNNING.store(false, Ordering::Release);
}
