//! Data models of GitHub Actions workflows.

use serde::Deserialize;

pub mod artifact;

/// Represents a GitHub Actions workflow run from GitHub REST API.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub repository_id: u64,
    pub head_repository_id: u64,
    pub head_branch: String,
    pub head_sha: String,
}
