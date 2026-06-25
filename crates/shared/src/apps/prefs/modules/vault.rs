//! Module for managing HashiCorp Vault integration preferences.
//!
//! This module provides functionality to enable/disable the Vault integration
//! feature, which allows secure secret management through HashiCorp Vault.

use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager};

/// Represents the Vault integration module preference.
///
/// This capability allows users to enable or disable the HashiCorp Vault integration
/// within the application for secure secret management.
pub struct VaultCapability;

impl PreferenceKey for VaultCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "module.vault";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for VaultCapability {
    /// Returns metadata about the Vault integration preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing information such as title, description, tags,
    /// category, and default value.
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Хранилище",
            description: "Включить модуль интеграции с HashiCorp Vault для управления секретами",
            tags: &[
                "vault",
                "секреты",
                "пароли",
                "хранилище",
                "интерфейс",
                "модуль",
            ],
            category: Category::Modules,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::Restart, Requirement::Admin],
            default_value: "false",
        }
    }

    /// Retrieves the current state of the Vault integration preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing preferences.
    ///
    /// # Returns
    ///
    /// An `Option<String>` representing whether the Vault module is currently active.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(Some(
            orch.preference_is_active::<Self>().to_string(),
        ))
    }

    /// Validates prerequisites before executing the preference change.
    ///
    /// Ensures that the user has administrative privileges to modify this setting.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator (unused in current implementation).
    /// * `new` - The new value being set for the preference.
    ///
    /// # Returns
    ///
    /// An error if the user lacks administrative privileges; otherwise, returns `Ok(())`.
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

    /// Provides post-execution feedback to the user.
    ///
    /// Informs the user that a restart is required for changes to take effect.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for logging information.
    /// * `new` - The new value being set (unused in current implementation).
    async fn after_execute(
        &self,
        orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        orch.info("Для применения изменений, пожалуйста перезагрузите приложение");
        Ok(())
    }
}
