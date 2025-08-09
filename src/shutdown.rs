use std::{fs, process, sync::OnceLock};
use tokio::{signal, sync::broadcast};
use tracing::info;

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

#[derive(Debug, Clone)]
pub enum ShutdownAction {
    Stop,
    Restart,
    Update { binary_path: String },
}

async fn restart() {
    info!("restarting…");
    process::Command::new("sudo").arg("./api").spawn().unwrap();
}

async fn update(binary_path: &str) {
    info!("updating from {}…", binary_path);
    self_replace::self_replace(binary_path);
    fs::remove_file(binary_path);

    restart().await
}
