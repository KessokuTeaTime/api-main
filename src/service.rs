use std::{
    fmt::Display,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use parking_lot::Mutex;
use tracing::{trace, warn};

use crate::static_lazy_lock;

static_lazy_lock! {
    pub STATUS: Mutex<ServiceStatus> = Mutex::new(ServiceStatus::Running);
}

static_lazy_lock! {
    GUARD: Arc<()> = Arc::new(());
}

pub fn drain(reason: DrainingReason) {
    warn!("draining service due to {reason}!");
    STATUS.lock().drain(reason);
}

pub fn check() -> Result<(), Response> {
    match *STATUS.lock() {
        ServiceStatus::Running => Ok(()),
        status => Err(status.response()),
    }
}

pub fn run() -> Guard {
    Guard::new()
}

#[derive(Debug)]
pub struct Guard(Arc<()>);

impl Guard {
    fn new() -> Self {
        let count = Arc::strong_count(&GUARD) - 1;
        trace!("starting business ({} running)", count);
        Self(GUARD.clone())
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let count = Arc::strong_count(&GUARD) - 2;
        let is_drained = count == 1;
        trace!("stopping business ({} running)", count);

        if let ServiceStatus::Draining(_) = *STATUS.lock()
            && is_drained
            && !IS_STOPPING.swap(true, Ordering::Acquire)
        {
            warn!("stopping service because all businesses are drained!");
            tokio::spawn(stop());
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum ServiceStatus {
    Running,
    Draining(DrainingReason),
}

impl ServiceStatus {
    fn drain(&mut self, reason: DrainingReason) {
        match self {
            Self::Running => *self = Self::Draining(reason),
            _ => {}
        }
    }

    pub fn response(&self) -> Response {
        match self {
            Self::Running => StatusCode::OK.into_response(),
            Self::Draining(reason) => reason.response(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum DrainingReason {
    Updating,
}

impl Display for DrainingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Updating => "server update",
            }
        )
    }
}

impl DrainingReason {
    pub fn response(&self) -> Response {
        match self {
            Self::Updating => (StatusCode::SERVICE_UNAVAILABLE, "Server updating…").into_response(),
        }
    }
}

static IS_STOPPING: AtomicBool = AtomicBool::new(false);

async fn stop() {}
