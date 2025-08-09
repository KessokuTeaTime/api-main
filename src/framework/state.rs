use tracing::{error, warn};

use crate::env::MAX_RETRY;

/// A state that controls the flow of data.
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum State<T> {
    /// The control flow should exit with a value.
    Success(T),
    /// The control flow should retry if possible.
    ///
    /// See: [retry_if_possible]
    Retry,
    /// The control flow should exit immediately.
    Stop,
}

impl<T> State<T> {
    /// Maps the value if [`self`] is [`State::Success`].
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

    /// Replaces [`self`] with a same-typed [`State`] if [`self`] is [`State::Success`].
    pub fn replace(self, state: Self) -> Self {
        match self {
            State::Success(_) => state,
            State::Retry => State::Retry,
            State::Stop => State::Stop,
        }
    }
}

/// Decides whether retrying is allowed based on a provided retry times and the [`MAX_RETRY`] environment variable.
///
/// # Errors
///
/// Returns [`Err<()>`] if retrying is not allowed, otherwise [`Ok<()>`] is returned.
pub fn retry_if_possible(retry: &mut u8) -> Result<(), ()> {
    *retry += 1;
    if *retry > *MAX_RETRY {
        error!("retried for too many times ({}), stopping!", *MAX_RETRY);
        Err(())
    } else {
        warn!("retrying… ({retry} / {})", *MAX_RETRY);
        Ok(())
    }
}
