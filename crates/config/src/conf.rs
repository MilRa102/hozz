use std::path::Path;

use anyhow::Result;
use figment::{
    Figment,
    providers::{Env, Format, Toml, Yaml},
};
use serde::Deserialize;

use crate::{app::AppConfig, mihomo::MihomoConfig, workspace::WorkspaceConfig};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub mihomo: MihomoConfig,
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let mut figment = Figment::new();

        if Path::new("config.toml").exists() {
            figment = figment.merge(Toml::file("config.toml"));
        }

        if Path::new("config.yaml").exists() {
            figment = figment.merge(Yaml::file("config.yaml"));
        }

        figment = figment.merge(Env::prefixed("").split("__"));

        let cfg: Config = figment.extract()?;
        cfg.workspace.ensure_dir()?;

        println!("Configuration loaded successfully: {cfg:#?}");
        Ok(cfg)
    }
}
