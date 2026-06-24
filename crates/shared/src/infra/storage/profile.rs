use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{
    app::{
        orchestrator::Orchestrator,
        profile::{Profile, Source},
    },
    core::models::conf::Provider,
    db::SledManager,
    infra::{LoggingLayer, storage::group::GroupManager},
};

#[async_trait]
pub trait ProfileManager {
    /// Adds a new profile and synchronizes states
    ///
    /// # Arguments
    /// * `url` - Link to the deleted profile
    ///
    /// # Returns
    /// Successful profile addition and successful synchronization or an error at which step it failed
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// let _ = orch.add_profile("https://profile.example.com").await;
    /// ```
    async fn add_profile(&self, url: &str) -> Result<()>;

    /// Delete existing profile and synchronization states
    ///
    /// # Arguments
    /// * `profile_id` - Profile ID
    ///
    /// # Returns
    /// Successful profile deleting and successful synchronization or an error at which step it failed
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// let _ = orch.delete_profile("153B0D457CB211C3").await;
    /// ```
    async fn delete_profile(&self, id: &str) -> Result<()>;

    async fn update_profile(&self, id: &str);

    async fn health_profile(&self, id: &str);

    /// Synchronization of rules, with config update
    ///
    /// # Returns
    /// Successful update or error while executing queries
    async fn sync_profiles(&self) -> Result<()>;

    async fn toggle_profile(&self, id: &str) -> Result<()>;
}

#[async_trait]
impl ProfileManager for Orchestrator {
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

    async fn update_profile(&self, id: &str) {
        if let Err(e) = self.dispatch.api.update_provider(id).await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to update provider");
            self.warning(e.to_string());
        }
        self.health_profile(id).await;
    }

    async fn health_profile(&self, id: &str) {
        if let Err(e) = self.dispatch.api.healthcheck_provider(id).await {
            tracing::warn!(error = %e, profile_id = %id, "Failed to healthcheck provider");
            self.warning(e.to_string());
        };

        self.sync_groups().await;
    }

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
