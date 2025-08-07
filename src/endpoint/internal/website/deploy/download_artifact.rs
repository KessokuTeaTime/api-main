use super::{Payload, cleanup::Case};
use crate::{
    framework::{queued_async::QueuedAsyncFrameworkContext, state::State},
    workflow::artifact::Artifact,
};

use anyhow::anyhow;
use async_zip::base::read::stream::ZipFileReader;
use futures::{AsyncReadExt as _, Stream, TryStreamExt as _};

use sha2::Digest;
use tokio_util::bytes::Bytes;
use tracing::{error, info};

pub(super) struct Input {
    pub(super) payload: Payload,
    pub(super) artifact: Artifact,
}

impl From<super::fetch_artifact::Output> for Input {
    fn from(output: super::fetch_artifact::Output) -> Self {
        Self {
            payload: output.payload,
            artifact: output.artifact,
        }
    }
}

pub(super) struct Output {
    pub(super) payload: Payload,
    pub(super) case: Case,
}

pub(super) async fn run(
    cx: &QueuedAsyncFrameworkContext,
    Input { payload, artifact }: Input,
) -> State<Output> {
    match crate::workflow::artifact::download_artifact(&artifact).await {
        State::Success(stream) => {
            info!("downloading artifact with {}…", cx.payload_display);
            let case = extract_archive(stream, artifact.digest.as_deref(), &payload.path()).await;
            State::Success(Output { payload, case })
        }
        State::Retry => {
            error!("failed to download artifact with {}", cx.payload_display);
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
