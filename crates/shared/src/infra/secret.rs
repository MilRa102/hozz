use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Map, Value as Json};
use vaultrs::{
    client::{Client, VaultClient, VaultClientSettingsBuilder},
    kv2,
};

use crate::{
    app::orchestrator::Orchestrator,
    apps::vault::{SecretItem, TokenInfo},
    db::vault::SecretVisit,
    infra::LoggingLayer,
};

#[async_trait]
pub trait SecretManager {
    fn vault(&self) -> Result<VaultClient>;
    async fn session(&self) -> Result<TokenInfo>;
    async fn secrets(&self, mount: &str, path: &str) -> Result<Vec<SecretItem>>;
    async fn secret(&self, mount: &str, path: &str) -> Result<Map<String, Json>>;
    async fn mounts(&self) -> Result<Vec<String>>;
    fn track_visit(&self, mount: &str, path: &str);
    fn frequent_visits(&self) -> Vec<SecretVisit>;
}

#[async_trait]
impl SecretManager for Orchestrator {
    fn vault(&self) -> Result<VaultClient> {
        let cfg = self
            .vaults
            .fetch()
            .ok_or(anyhow!("Vault client not initialized"))?;

        Ok(VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(cfg.url)
                .token(cfg.token)
                .build()?,
        )?)
    }

    async fn session(&self) -> Result<TokenInfo> {
        let client = self.vault()?;
        Ok(client.lookup().await?.into())
    }

    async fn secrets(&self, mount: &str, path: &str) -> Result<Vec<SecretItem>> {
        let client = self.vault()?;
        let keys = kv2::list(&client, mount, path).await?;
        let items = keys.into_iter().map(SecretItem::from).collect();
        Ok(items)
    }

    async fn secret(&self, mount: &str, path: &str) -> Result<Map<String, Json>> {
        let client = self.vault()?;
        let path = path.trim_start_matches('/');
        let secret = kv2::read::<Map<String, Json>>(&client, mount, path).await?;
        Ok(secret)
    }

    async fn mounts(&self) -> Result<Vec<String>> {
        let client = self.vault()?;
        let mounts = vaultrs::sys::mount::list(&client).await?;

        let kvs = mounts
            .into_iter()
            .filter(|(_, info)| info.mount_type == "kv")
            .map(|(path, _)| path.trim_end_matches('/').to_string())
            .collect();
        Ok(kvs)
    }

    fn track_visit(&self, mount: &str, path: &str) {
        if let Some(mut cfg) = self.vaults.fetch() {
            if let Some(visit) = cfg
                .visited
                .iter_mut()
                .find(|v| v.mount == mount && v.path == path)
            {
                visit.count = visit.count.saturating_add(1);
            } else {
                cfg.visited.push(SecretVisit::new(mount, path));
            }

            if cfg.visited.len() > 100 {
                cfg.visited
                    .sort_by_key(|b| std::cmp::Reverse(b.count));
                cfg.visited.truncate(50);
            }

            if let Err(e) = self.vaults.update(&cfg) {
                tracing::warn!(error = %e, "Failed to update visit information.");
                self.warning("Не удалось обновить информацию о посещении");
            }
        }
    }

    fn frequent_visits(&self) -> Vec<SecretVisit> {
        self.vaults
            .fetch()
            .map(|cfg| {
                let mut visits = cfg.visited.clone();
                visits.sort_by_key(|b| std::cmp::Reverse(b.count));
                visits.into_iter().take(10).collect()
            })
            .unwrap_or_default()
    }
}
