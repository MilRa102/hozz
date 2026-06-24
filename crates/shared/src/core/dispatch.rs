use std::sync::Arc;

use tokio::sync::RwLock;

use super::models::{api::Client, conf::Mihomo};
use crate::core::manager;

#[derive(Debug, Clone)]
pub struct Dispatch {
    pub core: manager::Manager,
    pub api: Client,
    pub conf: Arc<RwLock<Mihomo>>,
}

impl Dispatch {
    pub async fn init() -> anyhow::Result<Self> {
        let config = Mihomo::load().await?;
        let api = Client::new();
        let core = manager::Manager::new();

        Ok(Self {
            core,
            api,
            conf: Arc::new(RwLock::new(config)),
        })
    }

    pub(crate) async fn apply_changes(&self) -> anyhow::Result<()> {
        let cfg = self.conf.read().await;
        cfg.save().await?;

        self.api.update_config(&cfg).await?;
        Ok(())
    }
}
