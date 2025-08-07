use crate::{
    framework::{FrameworkContext, state::State},
    workflow::artifact::Artifact,
};

use tracing::{error, info};

pub struct Input<V> {
    pub passthrough: V,
    pub run_id: String,
}

pub struct Output<V> {
    pub passthrough: V,
    pub artifact: Artifact,
}

pub async fn run<Cx, V>(
    cx: &Cx,
    Input {
        passthrough,
        run_id,
    }: Input<V>,
) -> State<Output<V>>
where
    Cx: FrameworkContext,
{
    match crate::workflow::artifact::fetch_artifact("KessokuTeaTime", "website", &run_id).await {
        State::Success(artifact) => {
            info!("fetched artifact with {}", cx.payload_display());
            State::Success(Output {
                passthrough,
                artifact,
            })
        }
        State::Retry => {
            error!("failed to fetch artifact with {}", cx.payload_display());
            State::Retry
        }
        State::Stop => State::Stop,
    }
}
