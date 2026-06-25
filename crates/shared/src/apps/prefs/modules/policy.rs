use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager};

/// Represents the preference key for enabling/disabling the policy management section.
///
/// This module controls whether the policy management interface is visible in the sidebar.
pub struct PolicyCapability;

impl PreferenceKey for PolicyCapability {
    /// The unique identifier for this preference setting.
    const ID: &'static str = "module.policies";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for PolicyCapability {
    /// Returns metadata describing the preference setting.
    ///
    /// # Returns
    /// A `SettingMeta` containing configuration details such as title, description, tags, category,
    /// and default value.
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Политики",
            description: "Отображать раздел управления политиками в боковом меню",
            tags: &["policy", "политики", "интерфейс", "модуль"],
            category: Category::Modules,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::Restart, Requirement::Admin],
            default_value: "true",
        }
    }

    /// Retrieves the current state of the preference.
    ///
    /// # Arguments
    /// * `orch` - A reference to the orchestrator containing application state.
    ///
    /// # Returns
    /// A `Result` containing an `Option<String>` representing the current preference value as a string.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(Some(
            orch.preference_is_active::<Self>().to_string(),
        ))
    }

    /// Executes validation and preparation logic before applying the new preference value.
    ///
    /// # Arguments
    /// * `orch` - A reference to the orchestrator for access to application state.
    /// * `new` - The new preference value to be applied.
    ///
    /// # Errors
    /// Returns an error if the current user does not have administrator privileges required to modify this setting.
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

    /// Executes post-execution logic after the preference value has been applied.
    ///
    /// # Arguments
    /// * `orch` - A reference to the orchestrator for logging purposes.
    /// * `new` - The newly applied preference value.
    ///
    /// # Returns
    /// A `Result` indicating success or failure of the post-execution logic.
    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.info("Для применения изменений, пожалуйста перезагрузите приложение");
        Ok(())
    }
}
