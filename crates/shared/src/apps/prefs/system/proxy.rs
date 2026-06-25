//! System proxy preference capability for managing network proxy settings.
//!
//! This module provides functionality to enable/disable the system-wide proxy
//! configuration and handle related side effects like orchestrator reloads.

use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, PrefsManager, proxy::CoreController};

/// Represents the system proxy preference capability.
///
/// This struct implements the `PreferenceHook` trait to allow configuration
/// of the system-wide proxy setting through the preferences system.
#[derive(Debug)]
pub(crate) struct SystemProxyCapability;

impl PreferenceKey for SystemProxyCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "network.system_proxy";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for SystemProxyCapability {
    /// Returns metadata about the preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing configuration details such as title, description,
    /// category, tags, and default value.
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

    /// Retrieves the current state of the system proxy preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing preference state.
    ///
    /// # Returns
    ///
    /// An `Option<String>` containing the current state as a string representation.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        Ok(Some(
            orch.preference_is_active::<Self>().to_string(),
        ))
    }

    /// Executes the action when the preference value changes.
    ///
    /// Toggles the system proxy based on the new value provided.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator (unused in this implementation).
    /// * `new` - The new preference value as a string.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result<()>` indicating success or failure of the toggle operation.
    async fn execute(&self, _orch: Arc<Orchestrator>, new: &str) -> anyhow::Result<()> {
        let is_enabled = new.parse().unwrap_or(false);
        let controller = machine::SysProxyController;
        controller.toggle(is_enabled)?;
        Ok(())
    }

    /// Handles post-execution actions, such as orchestrator reload.
    ///
    /// If the orchestrator is connected, it triggers a connection toggle to ensure
    /// the system proxy changes take effect.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for performing post-execution actions.
    /// * `_new` - The new preference value (unused in this implementation).
    ///
    /// # Returns
    ///
    /// An `anyhow::Result<()>` indicating success or failure of the post-execution logic.
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
