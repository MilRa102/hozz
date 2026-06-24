use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::{
    app::orchestrator::Orchestrator,
    infra::{CoreController, LoggingLayer, PrefsManager},
};

pub(crate) struct SystemProxyCapability;

impl PreferenceKey for SystemProxyCapability {
    const ID: &'static str = "network.system_proxy";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for SystemProxyCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Системный прокси",
            description: "Использовать системный прокси для сетевых запросов",
            tags: &["system", "proxy", "сетевой", "прокси"],
            category: Category::System,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::CoreReload],
            default_value: "false",
        }
    }

    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(Some(
            orch.preference_is_active::<Self>().to_string(),
        ))
    }

    async fn execute(&self, _orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_enabled = new.parse().unwrap_or(false);
        let controller = machine::SysProxyController;
        controller.toggle(is_enabled)?;
        Ok(())
    }

    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        if orch.is_connected() {
            orch.toggle_connection(true).await?;
            orch.info("Ядро было успешно перезагружено");
        }
        Ok(())
    }
}
