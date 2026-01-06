use anyhow::Result;

use crate::env::DOCKER_COMPOSE_FILE;

pub async fn pull_image(image: &str) -> Result<()> {
    match tokio::process::Command::new("docker")
        .arg("pull")
        .arg(image)
        .output()
        .await
    {
        Ok(_) => {
            tracing::info!("successfully pulled image {}", image);
            Ok(())
        }
        Err(e) => {
            tracing::error!("failed to pull image {}: {e:?}", image);
            Err(anyhow::anyhow!("failed to pull image"))
        }
    }
}

pub async fn compose_up(container_name: &str) -> Result<()> {
    match tokio::process::Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&*DOCKER_COMPOSE_FILE)
        .arg("up")
        .arg("-d")
        .arg(container_name)
        .arg("--force-recreate")
        .output()
        .await
    {
        Ok(_) => {
            tracing::info!("successfully uped container {}", container_name);
            Ok(())
        }
        Err(e) => {
            tracing::error!("failed to up container {}: {e:?}", container_name);
            Err(anyhow::anyhow!("failed to up container"))
        }
    }
}
