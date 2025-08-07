use super::Payload;
use crate::{
    framework::{queued_async::QueuedAsyncFrameworkContext, state::State},
    workflow::artifact::Artifact,
};

use tracing::{error, info};

pub(super) struct Input {
    payload: Payload,
}

impl From<Payload> for Input {
    fn from(payload: Payload) -> Self {
        Self { payload }
    }
}

pub(super) struct Output {
    pub(super) payload: Payload,
    pub(super) artifact: Artifact,
}

pub(super) async fn run(
    cx: &QueuedAsyncFrameworkContext,
    Input { payload }: Input,
) -> State<Output> {
    match crate::workflow::artifact::fetch_artifact("KessokuTeaTime", "website", &payload.run_id)
        .await
    {
        State::Success(artifact) => {
            info!("fetched artifact with {}", cx.payload_display);
            State::Success(Output { payload, artifact })
        }
        State::Retry => {
            error!("failed to fetch artifact with {}", cx.payload_display);
            State::Retry
        }
        State::Stop => State::Stop,
    }
}
