use anyhow::Error;
use tokio::fs::remove_dir_all;
use tracing::{error, info};

use super::Payload;
use crate::framework::{queued_async::QueuedAsyncFrameworkContext, state::State};

pub(super) enum Case {
    Deployed,
    Failed(Error),
    HashUnmatch,
}

pub(super) struct Input {
    pub(super) payload: Payload,
    pub(super) case: Case,
}

impl From<super::download_artifact::Output> for Input {
    fn from(output: super::download_artifact::Output) -> Self {
        Self {
            payload: output.payload,
            case: output.case,
        }
    }
}

pub(super) type Output = ();

pub(super) async fn run(
    cx: &QueuedAsyncFrameworkContext,
    Input { payload, case }: Input,
) -> State<Output> {
    match case {
        Case::Deployed => info!("successfully deployed {}!", cx.payload_display),
        Case::HashUnmatch => {
            error!("failed to deploy {}: broken artifact", cx.payload_display);
            remove_dir(&payload.path());
        }
        Case::Failed(err) => {
            error!("failed to deploy{}: {err}", cx.payload_display);
            remove_dir(&payload.path());
        }
    }
    State::Success(())
}

pub(super) async fn remove_dir(path: &str) {
    drop(remove_dir_all(path).await);
}
