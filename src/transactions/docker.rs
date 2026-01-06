use anyhow::Result;

use crate::env::DOCKER_COMPOSE_FILE;

pub async fn compose_down(container_name: &str) -> Result<()> {
    match tokio::process::Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&*DOCKER_COMPOSE_FILE)
        .arg("down")
        .arg("--rmi")
        .arg("local")
        .arg("--volumes")
        .arg("--remove-orphans")
        .output()
        .await
    {
        Ok(_) => {
            tracing::info!("successfully downed container {}", container_name);
            Ok(())
        }
        Err(e) => {
            tracing::error!("failed to down container {}: {e:?}", container_name);
            Err(anyhow::anyhow!("failed to down container"))
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
        .arg("--pull")
        .arg("always")
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
