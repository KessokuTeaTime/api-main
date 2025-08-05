use tracing::{error, warn};

use crate::MAX_RETRY;

const TRACING_REALM: &str = "[STATE]";

pub enum State<T> {
    Retry,
    Stop,
    Success(T),
}

impl<T> State<T> {
    pub fn map<F, R>(self, f: F) -> State<R>
    where
        F: FnOnce(T) -> R,
    {
        match self {
            State::Success(value) => State::Success(f(value)),
            State::Retry => State::Retry,
            State::Stop => State::Stop,
        }
    }
}

pub fn retry_if_possible(retry: &mut u8) -> Result<(), ()> {
    *retry += 1;
    if *retry > MAX_RETRY {
        error!("{TRACING_REALM} Retried for too many times ({MAX_RETRY}), stopping!");
        Err(())
    } else {
        warn!("{TRACING_REALM} Retrying… ({retry} / {MAX_RETRY})");
        Ok(())
    }
}
