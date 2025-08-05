use crate::KTT_API_KEY;
use crate::state::{State, retry_if_possible};
use crate::workflow::artifact::{download_artifact, fetch_artifact};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use async_zip::base::read::stream::ZipFileReader;
use futures::stream::TryStreamExt;
use parking_lot::Mutex;

use serde::Deserialize;

use spdlog::{debug, error, info, warn};

use std::fmt::Display;

use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, io, thread};
use tokio_util::io::StreamReader;

/// The pending [`WorkflowInfo`] to be deployed.
static PENDING_INFO: LazyLock<Mutex<Option<Body>>> = LazyLock::new(|| Mutex::new(None));
/// Indicates whether a worker thread is currently running.
static IS_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Deserialize, Clone)]
pub struct Body {
    pub run_id: String,
    pub dest: String,
}

impl Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}<~{}", self.dest, self.run_id))
    }
}

impl Body {
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
async fn post(req: HttpRequest, body: web::Json<Body>) -> impl Responder {
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
            debug!("Spawning a thread with {body:?}");
            thread::spawn(move || deploy(body.clone().validate()));
        }
        Err(_) => {
            // Suspends the latest request
            let mut guard = PENDING_INFO.lock();
            match guard.replace(body.clone()) {
                None => {
                    warn!("A thread is already running! Suspending deployment with {body:?}");
                }
                Some(old_body) => {
                    warn!(
                        "A thread is already running! Suspending deployment with {body:?}, replacing {old_body:?}"
                    );
                }
            }
        }
    }

    HttpResponse::Ok().finish()
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
async fn deploy(body: Body) {
    let mut body = body;

    'worker_loop: loop {
        let mut retry: u8 = 0;

        'artifact_loop: loop {
            // Fetches the artifact
            let artifact = match fetch_artifact("KessokuTeaTime", "website", &body.run_id).await {
                State::Success(artifact) => {
                    info!("Fetched artifact with {body:?}");
                    artifact
                }
                State::Retry => {
                    error!("Failed to fetch artifact with {body:?}");
                    match retry_if_possible(&mut retry) {
                        Ok(_) => continue 'artifact_loop,
                        Err(_) => break 'artifact_loop,
                    }
                }
                State::Stop => break 'artifact_loop,
            };

            // Downloads the artifact
            let stream = match download_artifact(artifact).await {
                State::Success(stream) => {
                    info!("Downloaded artifact with {body:?}");
                    stream
                }
                State::Retry => {
                    error!("Failed to download artifact with {body:?}");
                    match retry_if_possible(&mut retry) {
                        Ok(_) => continue 'artifact_loop,
                        Err(_) => break 'artifact_loop,
                    }
                }
                State::Stop => break 'artifact_loop,
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
            match extract_archive(&mut archive, &body.dest) {
                Ok(_) => {
                    info!(
                        "Successfully deployed to {} with {}!",
                        body.dest, body.run_id
                    )
                }
                Err(err) => {
                    error!("Failed to extract destination archive with {body:?}: {err}");
                }
            }

            break 'artifact_loop;
        }

        {
            let mut guard = PENDING_INFO.lock();
            match guard.take() {
                None => break 'worker_loop,
                Some(pending_body) => {
                    info!("Resolving pending deployment: {pending_body:?}");
                    body = pending_body;
                }
            }
        }
    }

    IS_THREAD_RUNNING.store(false, Ordering::Release);
}
