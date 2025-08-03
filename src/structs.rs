use serde::Deserialize;

#[derive(Debug,  Deserialize)]
pub struct Artifacts {
    pub total_count: u8,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug,  Deserialize, Clone)]
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

#[derive(Debug,  Deserialize, Clone)]
pub struct WorkflowRun {
    pub id: u64,
    pub repository_id: u64,
    pub head_repository_id: u64,
    pub head_branch: String,
    pub head_sha: String,
}