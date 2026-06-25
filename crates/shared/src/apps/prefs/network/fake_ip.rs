//! Module for managing Fake-IP DNS enhancement preferences.
//!
//! This module provides functionality to enable/disable the Fake-IP DNS feature,
//! which intercepts DNS queries at the TUN level for enhanced network management.

use std::sync::Arc;

use async_trait::async_trait;
use prefs::{
    Category, PreferenceHook, PreferenceKey, Requirement, SettingMeta, SettingType,
};

use crate::apps::{LoggingLayer, Orchestrator, proxy::CoreController};

/// Represents the Fake-IP DNS enhancement module preference.
///
/// This capability allows users to enable or disable the DNS interception feature
/// that uses fake IP addresses at the TUN level for network traffic management.
pub(crate) struct FakeIpCapability;

impl PreferenceKey for FakeIpCapability {
    /// The unique identifier for this preference key.
    const ID: &'static str = "network.fake_ip";
}

#[async_trait]
impl PreferenceHook<Arc<Orchestrator>> for FakeIpCapability {
    /// Returns metadata about the Fake-IP DNS enhancement preference setting.
    ///
    /// # Returns
    ///
    /// A `SettingMeta` containing information such as title, description, tags,
    /// category, and default value.
    fn meta(&self) -> SettingMeta {
        SettingMeta {
            id: Self::ID,
            title: "Fake-IP DNS",
            description: "Перехват DNS-запросов на уровне TUN",
            tags: &["fake", "ip", "dns", "днс", "фейк"],
            category: Category::Network,
            setting_type: SettingType::Toggle,
            requirements: &[Requirement::CoreReload],
            default_value: "true",
        }
    }

    /// Retrieves the current state of the Fake-IP DNS enhancement preference.
    ///
    /// # Arguments
    ///
    /// * `orch` - A reference to the orchestrator for accessing configuration.
    ///
    /// # Returns
    ///
    /// An `Option<String>` representing whether Fake-IP mode is currently enabled.
    async fn actual_state(
        &self,
        orch: Arc<Orchestrator>,
    ) -> anyhow::Result<Option<String>> {
        let cfg = orch.dispatch.conf.read().await;
        Ok(Some(
            (cfg.dns.enhanced_mode == "fake-ip").to_string(),
        ))
    }

    /// Executes the preference change by updating the DNS enhanced mode configuration.
    ///
    /// Parses the new value and updates the `enhanced_mode` flag in the core controller's
    /// DNS configuration, switching between "fake-ip" and "redir-host" modes.
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
        cfg.dns.enhanced_mode =
            if is_enabled { "fake-ip" } else { "redir-host" }.to_string();
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
