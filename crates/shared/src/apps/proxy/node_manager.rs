use async_trait::async_trait;

use crate::apps::{LoggingLayer, Orchestrator, node::GroupNode};

/// A trait defining the interface for managing proxy nodes and groups.
///
/// This trait provides methods to control the selection of active proxy nodes within specific groups
/// and to synchronize the list of available groups from the system. It is implemented by the `Orchestrator`
/// struct to provide centralized management of proxy node configurations.
///
/// # Methods
/// * `select_active_in_group()` - Selects a specific proxy node as active within a given group.
/// * `sync_groups()` - Retrieves and synchronizes all available proxy groups from the system.
#[async_trait]
pub trait NodeManager {
    /// Selects a specific proxy node as active within a given group.
    ///
    /// This method requests the API to select the specified proxy node (identified by its name)
    /// as the active node within the provided group. Upon successful selection, it refreshes the
    /// list of groups to ensure the application state remains consistent with the system configuration.
    ///
    /// # Arguments
    /// * `self` - A reference to the node manager instance.
    /// * `group` - The name or identifier of the group containing the proxy nodes.
    /// * `name` - The name of the proxy node to select as active within the group.
    ///
    /// # Returns
    /// * `()` - Success if the node is selected and groups are synchronized, or an error message if failed.
    async fn select_active_in_group(&self, group: &str, name: &str);

    /// Retrieves and synchronizes all available proxy groups from the system.
    ///
    /// This method fetches the complete list of proxy groups from the API and converts them into
    /// `GroupNode` structures for use within the application. It updates the application state with
    /// the retrieved groups and identifies the active node within the primary group if one exists.
    ///
    /// # Arguments
    /// * `self` - A reference to the node manager instance.
    ///
    /// # Returns
    /// * `()` - Success if groups are synchronized, or a warning if the fetch fails.
    async fn sync_groups(&self);
}

#[async_trait]
impl NodeManager for Orchestrator {
    /// Selects a specific proxy node as active within a given group.
    ///
    /// This method requests the API to select the specified proxy node (identified by its name)
    /// as the active node within the provided group. Upon successful selection, it refreshes the
    /// list of groups to ensure the application state remains consistent with the system configuration.
    ///
    /// # Arguments
    /// * `self` - A reference to the node manager instance.
    /// * `group` - The name or identifier of the group containing the proxy nodes.
    /// * `name` - The name of the proxy node to select as active within the group.
    async fn select_active_in_group(&self, group: &str, name: &str) {
        match self.dispatch.api.select(group, name).await {
            Ok(()) => {
                // Log success and refresh groups
                self.info(format!("Профиль переключен: {name}"));
                self.sync_groups().await;
            },
            Err(e) => {
                // Log error and notify user
                tracing::error!(error = %e, group = %group, name = %name, "Failed to switch profile");
                self.error("Не удалось переключить профиль")
            },
        }
    }

    /// Retrieves and synchronizes all available proxy groups from the system.
    ///
    /// This method fetches the complete list of proxy groups from the API and converts them into
    /// `GroupNode` structures for use within the application. It updates the application state with
    /// the retrieved groups and identifies the active node within the primary group if one exists.
    ///
    /// # Arguments
    /// * `self` - A reference to the node manager instance.
    async fn sync_groups(&self) {
        match self.dispatch.api.all().await {
            Ok(proxies) => {
                // Convert API response to GroupNode structures
                let groups: Vec<GroupNode> = proxies.into();

                // Update active profile if a main group exists
                if let Some(main_group) = groups.first() {
                    let active_node = main_group.selected.clone();
                    self.state.update_active_profile(active_node);
                }

                // Store all groups in application state
                self.state.update_groups(groups);
            },
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get all groups");
            },
        }
    }
}
