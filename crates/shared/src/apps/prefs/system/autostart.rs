use std::sync::Arc;

use anyhow::Ok;
use async_trait::async_trait;
use prefs::{Category, PreferenceHook, PreferenceKey, SettingMeta, SettingType};

use crate::app::orchestrator::Orchestrator;

pub(crate) struct AutostartCapability;

impl PreferenceKey for AutostartCapability {
    const ID: &'static str = "system.autostart";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AutostartCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Автозапуск",
            description: "Запускать ядро при старте системы",
            tags: &[
                "startup",
                "boot",
                "автозагрузка",
                "система",
                "трей",
                "фоновый",
            ],
            category: Category::System,
            setting_type: SettingType::Toggle,
            requirements: &[],
            default_value: "false",
        }
    }

    async fn actual_state(
        &self,
        _orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let state = machine::AutostartApp.is_enable()?;
        Ok(Some(state.to_string()))
    }

    async fn execute(&self, _orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_autostart = new.parse().unwrap_or(false);
        let launch = machine::AutostartApp;
        launch.toggle(is_autostart)?;
        Ok(())
    }
}
