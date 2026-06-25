//! Module for managing system autostart preferences.
//!
//! This module provides functionality to enable/disable the autostart feature,
//! which controls whether the core application launches automatically when the system starts.

use std::sync::Arc;

use anyhow::Ok;
use async_trait::async_trait;
use prefs::{Category, PreferenceHook, PreferenceKey, SettingMeta, SettingType};

use crate::apps::Orchestrator;

/// Represents the system autostart module preference.
///
/// This capability allows users to enable or disable the feature that automatically
/// launches the core application during system startup.
pub(crate) struct AutostartCapability;

impl PreferenceKey for AutostartCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "system.autostart";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AutostartCapability {
    /// Returns metadata about the system autostart preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing information such as title, description, tags,
    /// category, and default value.
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

    /// Retrieves the current state of the system autostart preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator (unused in current implementation).
    ///
    /// # Returns
    ///
    /// An `Option<String>` representing whether autostart is currently enabled.
    async fn actual_state(
        &self,
        _orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let state = machine::AutostartApp.is_enable()?;
        Ok(Some(state.to_string()))
    }

    /// Executes the preference change by toggling the autostart setting.
    ///
    /// Parses the new value and calls the `toggle` method on the `AutostartApp` instance
    /// to enable or disable automatic startup.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator (unused in current implementation).
    /// * `new` - The new value being set for the preference (e.g., "true" or "false").
    ///
    /// # Returns
    ///
    /// An error if toggling the autostart setting fails; otherwise, returns `Ok(())`.
    async fn execute(&self, _orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_autostart = new.parse().unwrap_or(false);
        let launch = machine::AutostartApp;
        launch.toggle(is_autostart)?;
        Ok(())
    }

    /// Provides post-execution feedback.
    ///
    /// In the current implementation, no additional actions are required after executing
    /// the preference change.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator (unused in current implementation).
    /// * `new` - The new value being set (unused in current implementation).
    async fn after_execute(
        &self,
        _orch: Arc<Orchestrator>,
        _new: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
