use crate::{
    framework::State,
    fs::extract_archive,
    workflow::artifact::{Artifact, download_artifact},
};

use anyhow::{Error, anyhow};
use async_zip::base::read::stream::ZipFileReader;
use futures::{AsyncReadExt as _, Stream, TryStreamExt as _};

use sha2::Digest;
use tokio::fs::remove_dir_all;
use tokio_util::bytes::Bytes;
use tracing::{error, info};

enum Case {
    Deployed,
    Failed(Error),
    HashUnmatch,
}

/// Downloads an [`Artifact`] and extracts the downloaded archive to a specified path.
///
/// See: [`download_artifact`], [`extract_archive`]
pub async fn download_and_extract(artifact: Artifact, path: &str) -> State<()> {
    match download_artifact(&artifact).await {
        State::Success(stream) => {
            info!("downloading artifact {artifact}…",);
            let case = extract(stream, artifact.digest.as_deref(), path).await;
            cleanup(artifact.clone(), case, path).await;
            State::Success(())
        }
        State::Retry => {
            error!("failed to download artifact {artifact}",);
            State::Retry
        }
        State::Stop => State::Stop,
    }
}

async fn extract<S>(stream: S, digest: Option<&str>, path: &str) -> Case
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    let mut sha_hasher = sha2::Sha256::new();
    let mut read = stream
        .map_ok(|bytes| {
            sha_hasher.update(&bytes);
            bytes
        })
        .map_err(std::io::Error::other)
        .into_async_read();
    match extract_archive(ZipFileReader::new(&mut read), path).await {
        Ok(_) => {
            // Reads to end for consuming whole buf to hasher, neglecting the error
            drop(read.read_to_end(&mut Vec::new()).await);

            if hex::encode(sha_hasher.finalize()) == digest.unwrap()[7..] {
                Case::Deployed
            } else {
                Case::HashUnmatch
            }
        }
        Err(err) => Case::Failed(anyhow!(err)),
    }
}

async fn cleanup(artifact: Artifact, case: Case, path: &str) {
    match case {
        Case::Deployed => info!("successfully extracted {artifact} to {path}!"),
        Case::HashUnmatch => {
            error!("failed to extract {artifact} to {path}: broken artifact",);
            drop(remove_dir_all(path).await);
        }
        Case::Failed(err) => {
            error!("failed to extract {artifact} to {path}: {err}",);
            drop(remove_dir_all(path).await);
        }
    }
}
