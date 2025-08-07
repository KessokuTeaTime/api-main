use std::fmt::Display;

use crate::{framework::State, workflow::artifact::Artifact};

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

pub async fn run<V>(payload: V, artifact: Artifact, path: &str) -> State<()>
where
    V: Clone + Display,
{
    match crate::workflow::artifact::download_artifact(&artifact).await {
        State::Success(stream) => {
            info!("downloading artifact with {payload}…",);
            let case = extract_archive(stream, artifact.digest.as_deref(), &path).await;
            cleanup(payload.clone(), case, &path).await;
            State::Success(())
        }
        State::Retry => {
            error!("failed to download artifact with {payload}",);
            State::Retry
        }
        State::Stop => State::Stop,
    }
}

async fn extract_archive<S>(stream: S, digest: Option<&str>, path: &str) -> Case
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
    match crate::fs::extract_archive(ZipFileReader::new(&mut read), path).await {
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

async fn cleanup<V>(payload: V, case: Case, path: &str)
where
    V: Display,
{
    match case {
        Case::Deployed => info!("successfully deployed {payload}!"),
        Case::HashUnmatch => {
            error!("failed to deploy {payload}: broken artifact",);
            drop(remove_dir_all(&path).await);
        }
        Case::Failed(err) => {
            error!("failed to deploy{payload}: {err}",);
            drop(remove_dir_all(&path).await);
        }
    }
}
