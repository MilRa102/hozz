//! Module for managing process finding preferences.
//!
//! This module provides functionality to enable/disable the process finding feature,
//! which determines the application that created a network request.

use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, proxy::CoreController};

/// Represents the process finding module preference.
///
/// This capability allows users to enable or disable the feature that identifies
/// the application responsible for creating network requests.
pub(crate) struct FindProcessCapability;

impl PreferenceKey for FindProcessCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "network.find_process";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for FindProcessCapability {
    /// Returns metadata about the process finding preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing information such as title, description, tags,
    /// category, and default value.
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

    /// Retrieves the current state of the process finding preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing configuration.
    ///
    /// # Returns
    ///
    /// An `Option<String>` representing whether process finding is currently enabled.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let cfg = orch.dispatch.conf.read().await;
        Ok(Some((cfg.find_process == "always").to_string()))
    }

    /// Executes the preference change by updating the process finding configuration.
    ///
    /// Parses the new value and updates the `find_process` flag in the core controller's
    /// network configuration, switching between "always" and "strict" modes.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing and modifying configuration.
    /// * `new` - The new value being set for the preference (e.g., "true" or "false").
    ///
    /// # Returns
    ///
    /// An error if parsing the new value fails; otherwise, returns `Ok(())`.
    async fn execute(&self, orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_enabled = new.parse().unwrap_or(false);
        {
            let mut cfg = orch.dispatch.conf.write().await;
            cfg.find_process = if is_enabled { "always" } else { "strict" }.to_string();
        }
        Ok(())
    }

    /// Provides post-execution feedback and applies necessary changes.
    ///
    /// Applies configuration changes, renews the connection if connected, and informs the user.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for applying changes and logging information.
    /// * `new` - The new value being set (unused in current implementation).
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
