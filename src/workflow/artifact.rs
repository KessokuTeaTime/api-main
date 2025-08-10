//! Artifacts from GitHub REST API and related functions.

use std::{error::Error, fmt::Display};

use futures::Stream;
use reqwest::{RequestBuilder, header};
use serde::Deserialize;
use tokio_util::bytes::Bytes;
use tracing::{debug, error, info};

use crate::{env::GITHUB_TOKEN, framework::State, workflow::WorkflowRun};

/// Represents artifacts from GitHub REST API.
#[derive(Debug, Deserialize, Clone)]
pub struct Artifacts {
    pub total_count: u8,
    pub artifacts: Vec<Artifact>,
}

/// Represents an artifact from GitHub REST API.
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

impl Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} at {})",
            self.name, self.id, self.archive_download_url
        )
    }
}

/// Builds a request for GitHub REST API.
pub fn github_api_request_builder(url: &str) -> RequestBuilder {
    reqwest::Client::new()
        .get(url)
        .header(header::ACCEPT, "application/vnd.github+json")
        .bearer_auth(&*GITHUB_TOKEN)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "KessokuTeaTime-API/1.0")
}

/// Fetches artifacts from GitHub using the given parameters.
pub async fn fetch_artifacts(
    owner: &str,
    repo: &str,
    run_id: &str,
    count: Option<u8>,
) -> State<Vec<Artifact>> {
    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts");
    match &count {
        Some(1) => debug!("fetching 1 artifact from {url}…"),
        Some(count) => debug!("fetching {count} artifacts from {url}…"),
        None => debug!("fetching artifacts from {url}…"),
    }

    let response = match github_api_request_builder(&url).send().await {
        Ok(response) => response,
        Err(err) => {
            error!("failed to fetch artifacts from {url}: {err}");
            return match err {
                _ if err.is_connect() || err.is_timeout() => State::Retry,
                _ => State::Stop,
            };
        }
    };

    match response.json::<Artifacts>().await {
        Ok(json) => match json.total_count {
            0 => {
                error!("invalid workflow data: no artifacts at {url}!");
                State::Stop
            }
            total_count => match &count {
                Some(count) => match total_count {
                    total_count if total_count < *count => {
                        error!(
                            "invalid workflow data: too little artifacts at {url}! expected {count}, got {total_count}"
                        );
                        State::Stop
                    }
                    total_count if total_count > *count => {
                        error!(
                            "invalid workflow data: too many artifacts at {url}! expected {count}, got {total_count}",
                        );
                        State::Stop
                    }
                    total_count => {
                        match total_count {
                            1 => info!("fetched 1 artifact from {url}"),
                            count => info!("fetched {count} artifacts from {url}"),
                        }
                        State::Success(json.artifacts)
                    }
                },
                None => State::Success(json.artifacts),
            },
        },
        Err(err) => {
            error!("failed to parse data from {url}: {err}");

            if let Some(source) = err.source() {
                error!("{source}")
            }

            State::Retry
        }
    }
}

/// Fetches the only artifact from GitHub using the given parameters.
pub async fn fetch_artifact(owner: &str, repo: &str, run_id: &str) -> State<Artifact> {
    fetch_artifacts(owner, repo, run_id, Some(1))
        .await
        .map(|artifacts| artifacts[0].clone())
}

/// Downloads the specified artifact from GitHub.
pub async fn download_artifact(
    artifact: &Artifact,
) -> State<impl Stream<Item = Result<Bytes, reqwest::Error>> + use<>> {
    debug!(
        "requesting download from {}…",
        &artifact.archive_download_url
    );

    match github_api_request_builder(&artifact.archive_download_url)
        .send()
        .await
    {
        Ok(resp) => {
            let stream = resp.bytes_stream();
            info!("requested download from {}", artifact.archive_download_url);
            State::Success(stream)
        }
        Err(err) => match err.status() {
            Some(reqwest::StatusCode::GONE) => {
                error!("failed to request download: artifact expired or removed");
                State::Stop
            }
            Some(status) => {
                if let Some(reason) = status.canonical_reason() {
                    error!(
                        "failed to request download from {}: {} {reason}",
                        &artifact.archive_download_url,
                        status.as_u16()
                    );
                } else {
                    error!(
                        "failed to request download from {}: {}",
                        &artifact.archive_download_url,
                        status.as_u16()
                    )
                }
                State::Retry
            }
            None => {
                error!(
                    "failed to download artifact at {}",
                    &artifact.archive_download_url
                );
                State::Retry
            }
        },
    }
}
