use async_trait::async_trait;

use crate::{
    app::{nodes::GroupNode, orchestrator::Orchestrator},
    infra::log::LoggingLayer,
};

#[async_trait]
pub trait GroupManager {
    /// Selects an active node within a specified group.
    ///
    /// This method attempts to set a specific node as active within its group.
    /// On success, it notifies the state with an info message and triggers a group synchronization.
    /// On failure, it logs an error and notifies the state with an error message.
    ///
    /// # Arguments
    /// * `group` - A string slice that holds the name of the group.
    /// * `name` - A string slice that holds the name of the node to be selected.
    ///
    /// # Examples
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// orch.select_active_in_group("group_name", "node_name").await;
    /// ```
    async fn select_active_in_group(&self, group: &str, name: &str);

    /// Synchronization of groups, with state update
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// orch.sync_groups().await;
    /// ```
    async fn sync_groups(&self);
}

#[async_trait]
impl GroupManager for Orchestrator {
    async fn select_active_in_group(&self, group: &str, name: &str) {
        match self.dispatch.api.select(group, name).await {
            Ok(()) => {
                self.info(format!("Профиль переключен: {name}"));
                self.sync_groups().await;
            },
            Err(e) => {
                tracing::error!(error = %e, %group, %name, "Failed to switch profile");
                self.error("Не удалось переключить профиль")
            },
        }
    }

    async fn sync_groups(&self) {
        match self.dispatch.api.all().await {
            Ok(proxies) => {
                let groups: Vec<GroupNode> = proxies.into();

                if let Some(main_group) = groups.first() {
                    let active_node = main_group.selected.clone();
                    self.state.update_active_profile(active_node);
                }

                self.state.update_groups(groups);
            },
            Err(e) => tracing::warn!(error = %e, "Failed to get all groups"),
        }
    }
}
