//! Module for managing LAN (WSL/Docker) proxy support preferences.
//!
//! This module provides functionality to enable/disable the LAN proxy feature,
//! which allows devices on the local network and virtual machines to use a proxy.

use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, proxy::CoreController};

/// Represents the LAN proxy support module preference.
///
/// This capability allows users to enable or disable the feature that permits
/// local network devices and virtual machines (WSL/Docker) to use a proxy.
pub(crate) struct AllowLanCapability;

impl PreferenceKey for AllowLanCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "module.allow_lan";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for AllowLanCapability {
    /// Returns metadata about the LAN proxy support preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing information such as title, description, tags,
    /// category, and default value.
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

    /// Retrieves the current state of the LAN proxy support preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing configuration.
    ///
    /// # Returns
    ///
    /// An `Option<String>` representing whether LAN proxy is currently enabled.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let cfg = orch.dispatch.conf.read().await;
        Ok(Some(cfg.net.allow_lan.to_string()))
    }

    /// Executes the preference change by updating the configuration.
    ///
    /// Parses the new value and updates the `allow_lan` flag in the core controller's configuration.
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
        let mut cfg = orch.dispatch.conf.write().await;
        cfg.net.allow_lan = is_enabled;
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
