use std::{error::Error, ops::Range};

use futures::Stream;
use reqwest::{RequestBuilder, header};
use serde::Deserialize;
use tokio_util::bytes::Bytes;
use tracing::{error, info};

use crate::{env::GITHUB_TOKEN, state::State, workflow::WorkflowRun};

const TRACING_REALM: &str = "[WORKFLOW]";

#[derive(Debug, Deserialize, Clone)]
pub struct Artifacts {
    pub total_count: u8,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Artifact {
    pub id: u64,
    pub node_id: String,
    pub name: String,
    pub size_in_bytes: u64,
    pub url: String,
    pub archive_download_url: String,
    pub expired: bool,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub updated_at: Option<String>,
    pub digest: Option<String>,
    pub workflow_run: Option<WorkflowRun>,
}

/// Builds a request for GitHub API
pub fn github_api_request_builder(url: &str) -> RequestBuilder {
    reqwest::Client::new()
        .get(url)
        .header(header::ACCEPT, "application/vnd.github+json")
        .bearer_auth(&*GITHUB_TOKEN)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "KessokuTeaTime-API/1.0")
}

/// Fetches possible artifacts using the given parameters
pub async fn fetch_artifacts(
    owner: &str,
    repo: &str,
    run_id: &str,
    count_range: Option<Range<u8>>,
) -> State<Vec<Artifact>> {
    info!("Fetching artifact…");

    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts");

    let response = match github_api_request_builder(&url).send().await {
        Ok(response) => response,
        Err(err) => {
            error!("[WORKFLOW] Failed to fetch artifacts from {url}: {err}");
            return match err {
                _ if err.is_connect() || err.is_timeout() => State::Retry,
                _ => State::Stop,
            };
        }
    };

    match response.json::<Artifacts>().await {
        Ok(json) => match json.total_count {
            0 => {
                error!("{TRACING_REALM} Invalid workflow data: no artifacts at {url}!");
                State::Stop
            }
            count => match count_range {
                Some(count_range) => match count {
                    count if count < count_range.start => {
                        error!(
                            "{TRACING_REALM} Invalid workflow data: too little artifacts at {url}! Expected {}~{}, got {count}",
                            count_range.start, count_range.end
                        );
                        State::Stop
                    }
                    count if count > count_range.end => {
                        error!(
                            "{TRACING_REALM} Invalid workflow data: too many artifacts at {url}! Expected {}~{}, got {count}",
                            count_range.start, count_range.end
                        );
                        State::Stop
                    }
                    count => {
                        match count {
                            1 => info!("{TRACING_REALM} Accepted 1 artifact from {url}"),
                            count => info!("{TRACING_REALM} Accepted {count} artifacts from {url}"),
                        }
                        State::Success(json.artifacts)
                    }
                },
                None => State::Success(json.artifacts),
            },
        },
        Err(err) => {
            error!("{TRACING_REALM} Failed to parse data from {url}: {err}");

            if let Some(source) = err.source() {
                error!("{source}")
            }

            State::Retry
        }
    }
}

/// Fetches the only artifact using the given parameters
pub async fn fetch_artifact(owner: &str, repo: &str, run_id: &str) -> State<Artifact> {
    fetch_artifacts(owner, repo, run_id, None)
        .await
        .map(|artifacts| artifacts[0].clone())
}

// Downloads the specified artifact
pub async fn download_artifact(
    artifact: Artifact,
) -> State<impl Stream<Item = Result<Bytes, reqwest::Error>>> {
    match github_api_request_builder(&artifact.archive_download_url)
        .send()
        .await
    {
        Ok(resp) => {
            let stream = resp.bytes_stream();
            match stream.size_hint() {
                (min, Some(max)) => info!(
                    "{TRACING_REALM} Downloaded artifact at {} with size {}..{}",
                    artifact.archive_download_url, min, max
                ),
                (min, None) => info!(
                    "{TRACING_REALM} Downloaded artifact at {} with size >={}",
                    artifact.archive_download_url, min
                ),
            }
            State::Success(stream)
        }
        Err(err) => match err.status() {
            Some(reqwest::StatusCode::GONE) => {
                error!("{TRACING_REALM} Failed to download artifact: artifact expired or removed");
                State::Stop
            }
            Some(status) => {
                if let Some(reason) = status.canonical_reason() {
                    error!(
                        "{TRACING_REALM} Failed to download artifact at {}: {} {reason}",
                        &artifact.archive_download_url,
                        status.as_u16()
                    );
                } else {
                    error!(
                        "{TRACING_REALM} Failed to download artifact at {}: {}",
                        &artifact.archive_download_url,
                        status.as_u16()
                    )
                }
                State::Retry
            }
            None => {
                error!(
                    "{TRACING_REALM} Failed to download artifact at {}",
                    &artifact.archive_download_url
                );
                State::Retry
            }
        },
    }
}
