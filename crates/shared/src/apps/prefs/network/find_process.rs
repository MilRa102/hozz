use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::{
    app::orchestrator::Orchestrator,
    infra::{CoreController, LoggingLayer},
};

pub(crate) struct FindProcessCapability;

impl PreferenceKey for FindProcessCapability {
    const ID: &'static str = "network.find_process";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for FindProcessCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Журнал процессов",
            description: "Определять приложение, создавшее запрос",
            tags: &["find", "process", "процессы", "журнал"],
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
        Ok(Some((cfg.find_process == "always").to_string()))
    }

    async fn execute(&self, orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_enabled = new.parse().unwrap_or(false);
        {
            let mut cfg = orch.dispatch.conf.write().await;
            cfg.find_process = if is_enabled { "always" } else { "strict" }.to_string();
        }
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
