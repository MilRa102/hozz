use anyhow::Result;
use async_trait::async_trait;

use crate::{apps::Orchestrator, core::models::conf::default_rules, db::SledManager};

/// A trait defining the interface for managing proxy rules.
///
/// This trait provides methods to synchronize proxy rules with the system configuration.
/// The `Orchestrator` struct implements this trait to manage the lifecycle of proxy rules.
#[async_trait]
pub(crate) trait RuleManager {
    /// Synchronizes all active and non-ignored proxy rules with the system configuration.
    ///
    /// Retrieves all rules from the database, filters out inactive and ignored ones, and adds
    /// them to a list of active rules. A default "MATCH,AUTO" rule is always appended at the end.
    /// The system configuration is then updated with these active rules, and the changes are
    /// applied to the system.
    ///
    /// # Arguments
    /// * `self` - A reference to the rule manager instance.
    ///
    /// # Returns
    /// * `Result<()>` - Success if rules are synchronized, or an error if failed.
    async fn sync_rules(&self) -> Result<()>;
}

#[async_trait]
impl RuleManager for Orchestrator {
    /// Synchronizes all active and non-ignored proxy rules with the system configuration.
    ///
    /// Retrieves all rules from the database, filters out inactive and ignored ones, and adds
    /// them to a list of active rules. A default "MATCH,AUTO" rule is always appended at the end.
    /// The system configuration is then updated with these active rules, and the changes are
    /// applied to the system.
    ///
    /// # Arguments
    /// * `self` - A reference to the rule manager instance.
    async fn sync_rules(&self) -> Result<()> {
        let rules = self.rules.all()?;
        let mut active_rules = default_rules();

        for rule in rules
            .iter()
            .filter(|r| r.is_active() && !r.is_ignored())
        {
            active_rules.push(rule.to_rule());
        }
        active_rules.push("MATCH,AUTO".to_string());

        {
            let mut cfg = self.dispatch.conf.write().await;
            cfg.rules = active_rules;
        }

        self.dispatch.apply_changes().await?;
        Ok(())
    }
}
