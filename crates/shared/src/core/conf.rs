use std::path::PathBuf;

use anyhow::Context;
use config::CONF;
use tokio::fs;

use super::models::conf::Mihomo;

impl Mihomo {
    pub(crate) fn config_path() -> PathBuf {
        CONF.workspace.data_dir.join("config.yaml")
    }

    pub(crate) async fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();

        if path.exists() {
            match fs::read_to_string(&path).await {
                Ok(content) => match serde_yaml::from_str::<Self>(&content) {
                    Ok(config) => return Ok(config),
                    Err(error) => tracing::warn!(%error, "Failed to parse config"),
                },
                Err(error) => tracing::warn!(%error, "Failed to read config"),
            }
        } else {
            tracing::warn!("Config file not found.");
        }

        let config = Self::default();
        config.save().await?;

        Ok(config)
    }

    pub(crate) async fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        let contents = serde_yaml::to_string(&self)?;
        fs::write(path, contents)
            .await
            .context("Failed to write config")?;
        Ok(())
    }
}
