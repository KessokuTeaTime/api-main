use anyhow::Result;

use crate::env::DOCKER_COMPOSE_FILE;

pub async fn pull_image(image: &str) -> Result<()> {
    match tokio::process::Command::new("docker")
        .arg("pull")
        .arg(image)
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                tracing::info!("successfully pulled image {image}");
                Ok(())
            } else {
                tracing::error!(
                    "failed to pull image {image}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Err(anyhow::anyhow!("failed to pull image {image}"))
            }
        }
        Err(e) => {
            tracing::error!("command failed to execute: pulling image {image}: {e:?}");
            Err(anyhow::anyhow!(
                "command failed to execute: pulling image {image}"
            ))
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
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                tracing::info!("successfully upped container {container_name}");
                Ok(())
            } else {
                tracing::error!(
                    "failed to up container {container_name}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Err(anyhow::anyhow!("failed to up container {container_name}"))
            }
        }
        Err(e) => {
            tracing::error!("command failed to execute: upping container {container_name}: {e:?}");
            Err(anyhow::anyhow!(
                "command failed to execute: upping container {container_name}"
            ))
        }
    }
}
