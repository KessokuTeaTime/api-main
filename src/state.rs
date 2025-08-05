use spdlog::{error, warn};

use crate::MAX_RETRY;

pub enum State<T> {
    Retry,
    Stop,
    Success(T),
}

pub fn retry_if_possible(retry: &mut u8) -> Result<(), ()> {
    *retry += 1;
    if *retry > MAX_RETRY {
        error!("Retried for too many times ({MAX_RETRY}), stopping deployment!");
        Err(())
    } else {
        warn!("Retrying… ({retry} / {MAX_RETRY})");
        Ok(())
    }
}
