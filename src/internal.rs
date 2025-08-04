use crate::structs::{Artifact, Artifacts};
use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use reqwest::blocking::RequestBuilder;
use reqwest::header;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use spdlog::{debug, error, info, warn};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::Display;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::{env, fs, io, thread};
use zip::ZipArchive;

const MAX_RETRY: u8 = 5;

static KTT_API_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("KTT_API_KEY").expect("KTT_API_KEY not set in environment"));
static GITHUB_TOKEN: LazyLock<String> =
    LazyLock::new(|| env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set in environment"));

static QUEUE: LazyLock<Mutex<VecDeque<ActionWorkflowInfo>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));
static IS_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Debug, Clone)]
struct ActionWorkflowInfo {
    run_id: String,
    dest: String,
}

impl Display for ActionWorkflowInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}<~{}", self.dest, self.run_id))
    }
}

#[post("/internal/website/notify")]
async fn notify(req: HttpRequest, info: web::Json<ActionWorkflowInfo>) -> impl Responder {
    debug!(
        "Received notification from {}, authenticating…",
        req.connection_info().host()
    );

    let authed = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == *KTT_API_KEY)
        .unwrap_or(false);

    if !authed {
        error!(
            "Failed to authenticate the notification from {}",
            req.connection_info().host()
        );
        return HttpResponse::NotFound().finish();
    } else {
        info!(
            "Authenticated the notification from {}",
            req.connection_info().host()
        );
    }

    match IS_THREAD_RUNNING.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => {
            debug!("Spawning a thread with {info:?}");
            thread::spawn(move || deploy(info.clone()));
        }
        Err(_) => {
            info!("A thread is already running! Queueing deployment with {info:?}");
            (*QUEUE.lock().unwrap()).push_back(info.clone());
        }
    }

    HttpResponse::Ok().finish()
}

enum State<T> {
    Retry,
    Stop,
    Success(T),
}

fn get_http_builder(url: String) -> RequestBuilder {
    reqwest::blocking::Client::new()
        .get(url)
        .header(header::ACCEPT, "application/vnd.github+json")
        .bearer_auth(&*GITHUB_TOKEN)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "KessokuTeaTime-API/1.0")
}

fn fetch_artifact(run_id: &str) -> State<Artifact> {
    info!("Fetching artifact…");

    let url = format!(
        "https://api.github.com/repos/KessokuTeaTime/website/actions/runs/{run_id}/artifacts"
    );

    let response = match get_http_builder(url).send() {
        Ok(response) => response,
        Err(err) => {
            error!("Failed to fetch artifacts: {err}");
            return if err.is_connect() || err.is_timeout() {
                State::Retry
            } else {
                State::Stop
            };
        }
    };

    match response.json::<Artifacts>() {
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
            let source = err.source();
            if let Some(source) = source {
                error!("{source}");
            };
            State::Retry
        }
    }
}

fn extract_archive(archive: &mut ZipArchive<Cursor<Bytes>>, dest: &str) -> io::Result<()> {
    let dest = dest.trim_matches(|c: char| c.is_whitespace() || c == '/');
    let path = format!("/var/{dest}/html");

    fs::remove_dir_all(&path)?;
    fs::create_dir(&path)?;
    archive.extract_unwrapped_root_dir(&path, |_| true)?;
    Ok(())
}

fn retry_if_possible(retry: &mut u8) -> Result<(), ()> {
    *retry += 1;
    if *retry > MAX_RETRY {
        error!("Retried for too many times ({MAX_RETRY}), stopping deployment!");
        Err(())
    } else {
        warn!("Retrying… ({retry} / {MAX_RETRY})");
        Ok(())
    }
}

fn deploy(info: ActionWorkflowInfo) {
    let mut info = info;

    'worker_loop: loop {
        let mut retry: u8 = 0;

        'artifact_loop: loop {
            let artifact = match fetch_artifact(&info.run_id) {
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

            let bytes = match get_http_builder(artifact.archive_download_url)
                .send()
                .and_then(|r| r.bytes())
            {
                Ok(bytes) => {
                    info!("Downloaded artifact with {info:?}");
                    bytes
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

            if hex::encode(Sha256::digest(&bytes)) != artifact.digest.unwrap()[7..] {
                error!("Failed to match artifact's hash");
                match retry_if_possible(&mut retry) {
                    Ok(_) => continue 'artifact_loop,
                    Err(_) => break 'artifact_loop,
                }
            };

            match ZipArchive::new(Cursor::new(bytes)) {
                Ok(mut archive) => match extract_archive(&mut archive, &info.dest) {
                    Ok(_) => {
                        info!(
                            "Successfully deployed to {} with {}!",
                            info.dest, info.run_id
                        )
                    }
                    Err(err) => {
                        error!("Failed to extract destination archive with {info:?}: {err}");
                    }
                },
                Err(err) => {
                    error!("Failed to open archive: {err}");
                }
            };

            break 'artifact_loop;
        }

        match (*QUEUE.lock().unwrap()).pop_front() {
            None => break 'worker_loop,
            Some(next_info) => {
                info!("Resolving queued deployment: {next_info:?}");
                info = next_info;
            }
        }
    }

    IS_THREAD_RUNNING.store(false, Ordering::Release);
}
