use crate::structs::{Artifact, Artifacts};
use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use reqwest::blocking::RequestBuilder;
use reqwest::header;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use spdlog::{debug, error, info};
use std::error::Error;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::{fs, io, thread};
use zip::ZipArchive;

static CI_KEY: &str = r##"Basic HKc"#,ae%3'_,16+u7}*J]r\.,0!M7iuiV*<whfr>K#J)rI?]I"##;

static GITHUB_TOKEN: &str =
    "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";

static GLOBAL_STR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

static THREAD_RUNNING: AtomicBool = AtomicBool::new(false);

static MAX_RETRY_TIMES: u8 = 5;

#[derive(Deserialize)]
struct ActionWorkflowInfo {
    run_id: String,
}

#[post("/internal/website/notify")]
async fn notify(req: HttpRequest, info: web::Json<ActionWorkflowInfo>) -> impl Responder {
    debug!("{} try to notify!", req.connection_info().host());
    let authed = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == CI_KEY)
        .unwrap_or(false);
    if !authed {
        return HttpResponse::NotFound().finish();
    }
    match THREAD_RUNNING.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => {
            thread::spawn(move || deploy(info.run_id.clone()));
        }
        Err(_) => {
            *GLOBAL_STR.lock().unwrap() = Some(info.run_id.clone());
            info!(
                "Thread is running, storing the deployment: {}",
                &info.run_id
            );
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
        .bearer_auth(GITHUB_TOKEN)
        .header("X-GitHub-Api-Version", "2022-11-28")
}

fn get_artifact(run_id: &str) -> State<Artifact> {
    info!("Getting artifact ...");
    let url = format!(
        "https://api.github.com/repos/KessokuTeaTime/website/actions/runs/{run_id}/artifacts"
    );

    let response = match get_http_builder(url).send() {
        Ok(response) => response,
        Err(err) => {
            error!("Failed to get available artifacts, {err}");
            return if err.is_connect() || err.is_timeout() {
                error!("Retrying ...");
                State::Retry
            } else {
                State::Stop
            };
        }
    };

    match response.json::<Artifacts>() {
        Ok(json) => match json.total_count {
            0 => {
                error!("Invalid workflow data: No artifacts!");
                State::Stop
            }
            1 => {
                info!("Artifact got.");
                State::Success(json.artifacts[0].clone())
            }
            _ => {
                error!("Invalid workflow data: Too many artifacts!");
                State::Stop
            }
        },
        Err(err) => {
            error!("Json parse failed: {err}");
            let source = err.source();
            if let Some(source) = source {
                error!("StdError: {source}");
            };
            error!("Trying again...");
            State::Retry
        }
    }
}

fn extract_archive(archive: &mut ZipArchive<Cursor<Bytes>>) -> io::Result<()> {
    fs::remove_dir_all("/var/www/html")?;
    fs::create_dir("/var/www/html")?;
    archive.extract_unwrapped_root_dir("/var/www/html", |_| true)?;
    Ok(())
}

fn deploy(initial: String) {
    let mut run_id = initial;
    loop {
        let mut retry_times: u8 = 0;
        'artifact_loop: loop {
            info!("Getting {}", &run_id);

            let artifact = match get_artifact(&run_id) {
                State::Retry => {
                    retry_times += 1;
                    if retry_times > MAX_RETRY_TIMES {
                        error!("Failed too many times, stop deploying {run_id}");
                        break 'artifact_loop;
                    }
                    continue 'artifact_loop;
                }
                State::Stop => break 'artifact_loop,
                State::Success(artifact) => artifact,
            };

            let bytes = match get_http_builder(artifact.archive_download_url)
                .send()
                .and_then(|r| r.bytes())
            {
                Ok(bytes) => bytes,
                Err(err) => {
                    if err.status().is_some_and(|code| code == 410) {
                        error!("Artifact expired or removed.");
                        break 'artifact_loop;
                    }
                    retry_times += 1;
                    if retry_times > MAX_RETRY_TIMES {
                        error!("Failed too many times, stop deploying {run_id}");
                        break 'artifact_loop;
                    }
                    continue 'artifact_loop;
                }
            };

            if hex::encode(Sha256::digest(&bytes)) != artifact.digest.unwrap()[7..] {
                error!("Artifact hash mismatch");
                if retry_times > MAX_RETRY_TIMES {
                    error!("Failed too many times, stop deploying {run_id}");
                    break 'artifact_loop;
                }
                continue 'artifact_loop;
            };

            match ZipArchive::new(Cursor::new(bytes)) {
                Ok(mut archive) => {
                    if let Err(err) = extract_archive(&mut archive) {
                        error!("Failed to extract the website archive: {err}");
                        break 'artifact_loop;
                    }
                    info!("Deployed {}", &run_id);
                    break 'artifact_loop;
                }
                Err(err) => {
                    error!("Failed to open ZipArchive: {err}");
                    break 'artifact_loop;
                }
            };
        }

        let mut guard = GLOBAL_STR.lock().unwrap();
        if let Some(stored) = guard.take() {
            info!("Resolving {stored}");
            drop(guard);
            run_id = stored;
        } else {
            break;
        }
    }
    THREAD_RUNNING.store(false, Ordering::Release);
}
