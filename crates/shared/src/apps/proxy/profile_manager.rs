use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{
    apps::{LoggingLayer, NodeManager, Orchestrator, Profile, proxy::Source},
    core::models::conf::Provider,
};
use db::SledManager;

/// A trait defining the interface for managing proxy profiles.
///
/// This trait provides methods to perform CRUD operations on proxy profiles, including adding,
/// deleting, updating, and toggling their enabled state. It also includes functionality for
/// health checks and synchronization of profiles with the system configuration. The `Orchestrator`
/// struct implements this trait to manage the lifecycle of proxy profiles.
///
/// # Methods
/// * `add_profile()` - Adds a new proxy profile to the database.
/// * `delete_profile()` - Removes a specified proxy profile from the database.
/// * `update_profile()` - Updates the provider configuration for a specific profile.
/// * `health_profile()` - Performs a health check on a specific proxy provider.
/// * `sync_profiles()` - Synchronizes all enabled profiles with the system configuration.
/// * `toggle_profile()` - Toggles the enabled state of a specific profile.
#[async_trait]
pub trait ProfileManager {
    /// Adds a new proxy profile to the database.
    ///
    /// Creates a new `Profile` instance from the provided URL and adds it to the database.
    /// After adding, it attempts to synchronize the profiles with the system configuration.
    /// If synchronization fails, it logs a warning but returns an error to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `url` - The URL of the proxy server to add as a new profile.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the profile is added and synchronized, or an error if failed.
    async fn add_profile(&self, url: &str) -> Result<()>;

    /// Removes a specified proxy profile from the database.
    ///
    /// Deletes the profile with the given ID from the database and attempts to synchronize
    /// the remaining profiles with the system configuration. If synchronization fails, it logs
    /// a warning but returns an error to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to delete.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the profile is deleted and synchronized, or an error if failed.
    async fn delete_profile(&self, id: &str) -> Result<()>;

    /// Updates the provider configuration for a specific profile.
    ///
    /// Requests the API to update the provider configuration for the specified profile ID.
    /// After updating, it performs a health check on the provider. If the API update fails,
    /// it logs a warning and notifies the user via the logging layer.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to update.
    async fn update_profile(&self, id: &str);

    /// Performs a health check on a specific proxy provider.
    ///
    /// Requests the API to perform a health check on the provider associated with the
    /// specified profile ID. If the health check fails, it logs a warning and notifies the user.
    /// After the health check, it synchronizes the groups to reflect any changes.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to perform a health check on.
    async fn health_profile(&self, id: &str);

    /// Synchronizes all enabled profiles with the system configuration.
    ///
    /// Retrieves all profiles from the database and filters out disabled ones. For each enabled
    /// profile, it creates a `Provider` instance from the remote URL and updates the system
    /// configuration. It applies the changes to the system and updates the application state.
    /// Finally, it synchronizes the groups to reflect any changes.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    ///
    /// # Returns
    /// * `Result<()>` - Success if profiles are synchronized, or an error if failed.
    async fn sync_profiles(&self) -> Result<()>;

    /// Toggles the enabled state of a specific profile.
    ///
    /// Retrieves the profile with the given ID from the database, toggles its enabled state,
    /// and saves it back to the database. After toggling, it synchronizes the profiles with the
    /// system configuration. If synchronization fails, it logs a warning but returns an error
    /// to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to toggle.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the profile is toggled and synchronized, or an error if failed.
    async fn toggle_profile(&self, id: &str) -> Result<()>;
}

#[async_trait]
impl ProfileManager for Orchestrator {
    /// Adds a new proxy profile to the database.
    ///
    /// Creates a new `Profile` instance from the provided URL and adds it to the database.
    /// After adding, it attempts to synchronize the profiles with the system configuration.
    /// If synchronization fails, it logs a warning but returns an error to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `url` - The URL of the proxy server to add as a new profile.
    async fn add_profile(&self, url: &str) -> Result<()> {
        let profile = Profile::new(url);

        self.profiles
            .add(&profile)
            .context("Failed to save profile to DB")?;

        if let Err(e) = self.sync_profiles().await {
            tracing::warn!(error = %e, "Failed to sync profiles after adding new one");
            return Err(e);
        }

        Ok(())
    }

    /// Removes a specified proxy profile from the database.
    ///
    /// Deletes the profile with the given ID from the database and attempts to synchronize
    /// the remaining profiles with the system configuration. If synchronization fails, it logs
    /// a warning but returns an error to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to delete.
    async fn delete_profile(&self, id: &str) -> Result<()> {
        self.profiles
            .delete(id)
            .context("Failed to delete profile")?;

        if let Err(e) = self.sync_profiles().await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to sync profiles after deleting one");
            return Err(e);
        }

        Ok(())
    }

    /// Updates the provider configuration for a specific profile.
    ///
    /// Requests the API to update the provider configuration for the specified profile ID.
    /// After updating, it performs a health check on the provider. If the API update fails,
    /// it logs a warning and notifies the user via the logging layer.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to update.
    async fn update_profile(&self, id: &str) {
        if let Err(e) = self.dispatch.api.update_provider(id).await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to update provider");
            self.warning(e.to_string());
        }
        self.health_profile(id).await;
    }

    /// Performs a health check on a specific proxy provider.
    ///
    /// Requests the API to perform a health check on the provider associated with the
    /// specified profile ID. If the health check fails, it logs a warning and notifies the user.
    /// After the health check, it synchronizes the groups to reflect any changes.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to perform a health check on.
    async fn health_profile(&self, id: &str) {
        if let Err(e) = self.dispatch.api.healthcheck_provider(id).await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to healthcheck provider");
            self.warning(e.to_string());
        };

        self.sync_groups().await;
    }

    /// Synchronizes all enabled profiles with the system configuration.
    ///
    /// Retrieves all profiles from the database and filters out disabled ones. For each enabled
    /// profile, it creates a `Provider` instance from the remote URL and updates the system
    /// configuration. It applies the changes to the system and updates the application state.
    /// Finally, it synchronizes the groups to reflect any changes.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    async fn sync_profiles(&self) -> Result<()> {
        let profiles = self.profiles.all()?;
        let mut providers = HashMap::new();

        for profile in &profiles {
            if !profile.enabled {
                tracing::debug!(profile_id = %profile.id, "Profile disabled, skipping");
                continue;
            }

            let profile = profile.clone();
            let Source::Remote(url) = profile.source;
            providers.insert(profile.id.clone(), Provider::new(&url));
        }

        {
            let mut cfg = self.dispatch.conf.write().await;
            cfg.providers = providers;
        }

        self.dispatch.apply_changes().await?;

        tracing::info!(profiles_len = %profiles.len());
        self.state.update_profiles(profiles);
        self.sync_groups().await;

        Ok(())
    }

    /// Toggles the enabled state of a specific profile.
    ///
    /// Retrieves the profile with the given ID from the database, toggles its enabled state,
    /// and saves it back to the database. After toggling, it synchronizes the profiles with the
    /// system configuration. If synchronization fails, it logs a warning but returns an error
    /// to the caller.
    ///
    /// # Arguments
    /// * `self` - A reference to the profile manager instance.
    /// * `id` - The unique identifier of the profile to toggle.
    async fn toggle_profile(&self, id: &str) -> Result<()> {
        let mut profile = self
            .profiles
            .get(id)?
            .context("Profile not found in sled")?;

        profile.toggle();
        let is_now_enabled = profile.enabled;
        self.profiles.add(&profile)?;

        tracing::info!(profile_id = %id, is_enabled = %is_now_enabled, "Profile toggled");

        if let Err(e) = self.sync_profiles().await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to sync profiles");
            return Err(e);
        }
        Ok(())
    }
}
