use crate::env::DOCKER_CONTAINER_NAME_NGINX;

pub async fn reload() -> anyhow::Result<()> {
    match tokio::process::Command::new("docker")
        .arg("exec")
        .arg(&*DOCKER_CONTAINER_NAME_NGINX)
        .arg("nginx")
        .arg("-s")
        .arg("reload")
        .output()
        .await
    {
        Ok(_) => {
            tracing::info!("successfully reloaded nginx");
            Ok(())
        }
        Err(e) => {
            tracing::error!("failed to reload nginx: {e:?}");
            Err(anyhow::anyhow!("failed to reload nginx"))
        }
    }
}
