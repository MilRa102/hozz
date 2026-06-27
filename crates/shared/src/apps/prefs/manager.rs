use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use prefs::PreferenceKey;

use crate::apps::Orchestrator;
use db::SledManager;

#[async_trait]
pub trait PrefsManager {
    async fn sync_preferences(self: &Arc<Self>) -> anyhow::Result<()>;

    fn get_into_bool(self: &Arc<Self>, key: &str) -> bool;

    async fn set_preference(self: &Arc<Self>, k: &str, v: &str) -> anyhow::Result<()>;

    async fn toggle_preference(self: &Arc<Self>, k: &str) -> anyhow::Result<()>;

    fn preference_is_active<K: PreferenceKey>(self: &Arc<Self>) -> bool {
        self.get_into_bool(K::ID)
    }

    async fn toggle_pref<K: PreferenceKey>(self: &Arc<Self>) -> anyhow::Result<()>;
}

#[async_trait]
impl PrefsManager for Orchestrator {
    async fn sync_preferences(self: &Arc<Self>) -> anyhow::Result<()> {
        let registry = self.registry.clone();
        let all_meta = registry.all();

        for meta in all_meta {
            if self.prefs.get(meta.id)?.is_none() {
                tracing::info!(meta_id = %meta.id, "Initializing default preference");

                self.prefs.save(
                    meta.id,
                    &prefs::AppPrefs::new(meta.id, meta.default_value),
                )?;
            }

            if let Some(hook) = registry.get_hook(meta.id)
                && let Ok(Some(actual_val)) = hook.actual_state(self.clone()).await
            {
                let db_val = self
                    .prefs
                    .get(meta.id)?
                    .map(|p| p.value)
                    .unwrap_or(meta.default_value.to_string());

                if actual_val != db_val {
                    tracing::debug!(meta_id = %meta.id, "Syncing preference DB: {} -> {}", db_val, actual_val);

                    self.prefs.save(
                        meta.id,
                        &prefs::AppPrefs::new(meta.id, &actual_val),
                    )?
                }
            }
        }

        Ok(())
    }

    fn get_into_bool(self: &Arc<Self>, key: &str) -> bool {
        match self.prefs.get(key) {
            Ok(Some(pref)) => pref.as_bool(),
            _ => false,
        }
    }

    async fn set_preference(self: &Arc<Self>, k: &str, v: &str) -> anyhow::Result<()> {
        let hook = self
            .registry
            .get_hook(k)
            .context("Настройка не найдена в реестре")?;

        // Step 1 - Pre-hook
        hook.before_execute(self.clone(), v).await?;

        // Step 2 - Hook
        hook.execute(self.clone(), v).await?;

        // Step 3 - Consistency check
        self.prefs.save(k, &prefs::AppPrefs::new(k, v))?;

        // Step 4 - Post-hook
        hook.after_execute(self.clone(), v).await?;

        Ok(())
    }

    async fn toggle_preference(self: &Arc<Self>, k: &str) -> anyhow::Result<()> {
        let now = self.get_into_bool(k);
        let new = !now;

        self.set_preference(k, &new.to_string()).await?;
        Ok(())
    }

    async fn toggle_pref<K: PreferenceKey>(self: &Arc<Self>) -> anyhow::Result<()> {
        Ok(self.toggle_preference(K::ID).await?)
    }
}
