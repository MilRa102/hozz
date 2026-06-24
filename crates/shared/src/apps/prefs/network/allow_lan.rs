use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::{
    app::orchestrator::Orchestrator,
    infra::{CoreController, LoggingLayer},
};

pub(crate) struct AllowLanCapability;

impl PreferenceKey for AllowLanCapability {
    const ID: &'static str = "module.allow_lan";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AllowLanCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Поддержка LAN (WSL/Docker)",
            description: "Разрешить устройствам в локальной сети и виртуальным машинам использовать прокси.",
            tags: &[
                "wsl",
                "docker",
                "lan",
                "локальная сеть",
                "виртуальная машина",
            ],
            category: Category::Network,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::CoreReload],
            default_value: "false",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let cfg = orch.dispatch.conf.read().await;
        Ok(Some(cfg.net.allow_lan.to_string()))
    }

    async fn execute(&self, orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_enabled = new.parse().unwrap_or(false);
        let mut cfg = orch.dispatch.conf.write().await;
        cfg.net.allow_lan = is_enabled;
        Ok(())
    }

    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.dispatch.apply_changes().await?;
        if orch.is_connected() {
            orch.toggle_connection(true).await?;
            orch.info("Ядро было успешно перезагружено");
        }
        Ok(())
    }
}
