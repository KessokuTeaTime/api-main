use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
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

pub static RUNNING: AtomicUsize = AtomicUsize::new(usize::MIN);

pub fn drain(reason: DrainingReason) {
    warn!("draining service due to {reason}!");
    STATUS.lock().drain(reason);
}

pub fn are_businesses_drained() -> bool {
    RUNNING.load(Ordering::Acquire) == usize::MIN
}

pub fn check() -> Result<(), Response> {
    match *STATUS.lock() {
        ServiceStatus::Running => Ok(()),
        status => Err(status.response()),
    }
}

pub fn run() -> BusinessGuard {
    BusinessGuard::new()
}

#[derive(Debug)]
pub struct BusinessGuard;

impl BusinessGuard {
    fn new() -> Self {
        let count = RUNNING.fetch_add(1, Ordering::Acquire);
        trace!("starting business ({} running)", count);
        Self
    }
}

impl Drop for BusinessGuard {
    fn drop(&mut self) {
        let count = RUNNING.fetch_sub(1, Ordering::Acquire);
        trace!("stopping business ({} running)", count - 1);

        if are_businesses_drained() && !IS_STOPPING.swap(true, Ordering::Acquire) {
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
