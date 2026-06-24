use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::{
    app::orchestrator::Orchestrator,
    infra::{LoggingLayer, PrefsManager},
};

pub struct ContainerCapability;

impl PreferenceKey for ContainerCapability {
    const ID: &'static str = "module.containers";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for ContainerCapability {
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Контейнеры",
            description: "Отображать раздел управления Docker-контейнерами в боковом меню",
            tags: &["docker", "контейнеры", "интерфейс", "модуль"],
            category: Category::Modules,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::Restart, Requirement::Admin],
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

    async fn before_execute(
        &self,
        _orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        if !Orchestrator::is_admin() {
            anyhow::bail!(
                "Для изменения данной настройки, необходимо обладать правами администратора."
            );
        }
        Ok(())
    }

    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.info("Для применения изменений, пожалуйста перезагрузите приложение");
        Ok(())
    }
}
