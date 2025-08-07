use crate::framework::state::State;

use super::Payload;

pub type Input = crate::framework::transaction::global::fetch_artifact::Output<Payload>;

pub type Output = crate::framework::transaction::global::download_artifact::Input<Payload>;

pub async fn run(
    Input {
        passthrough: payload,
        artifact,
    }: Input,
) -> State<Output> {
    let path = payload.path();
    State::Success(Output {
        passthrough: payload,
        artifact,
        path,
    })
}
