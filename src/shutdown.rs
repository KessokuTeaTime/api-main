use std::{fs, os::unix::process::CommandExt, process, sync::OnceLock};
use tokio::{signal, sync::broadcast};
use tracing::{debug, error, info};

/// The broadcast sender to shutdown the server.
pub static SHUTDOWN: OnceLock<broadcast::Sender<ShutdownAction>> = OnceLock::new();

pub async fn signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl + C signal handler")
    };

    let mut shutdown = SHUTDOWN.get().unwrap().subscribe();

    tokio::select! {
        _ = ctrl_c => {}
        result = shutdown.recv() => if let Ok(action) = result { match action {
        ShutdownAction::Stop => {}
        ShutdownAction::Restart => restart().await,
        ShutdownAction::Update { binary_path } => update(&binary_path).await
        } }
    }
}

/// The action to perform when shutting down a server.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ShutdownAction {
    /// Gracefully stops the server.
    Stop,
    /// Restarts the server in-place.
    Restart,
    /// Updates the server from a new binary.
    Update {
        /// The path to the new binary.
        binary_path: String,
    },
}

async fn restart() {
    info!("restarting…");
    let err = process::Command::new("./api").exec();
    panic!("unable to restart the binary: {err}!");
}

async fn update(binary_path: &str) {
    info!("updating from {}…", binary_path);
    match self_replace::self_replace(binary_path) {
        Ok(_) => {
            debug!("successfully replaced binary from {binary_path}, removing abndant files…");
            drop(fs::remove_file(binary_path));
        }
        Err(err) => error!("failed replacing binary from {binary_path}: {err}"),
    }

    restart().await
}
