use anyhow::Result;
use async_trait::async_trait;

use crate::{
    app::orchestrator::Orchestrator, core::models::conf::default_rules, db::SledManager,
};

#[async_trait]
pub(crate) trait RuleManager {
    /// Synchronization of rules, with config update
    ///
    /// # Returns
    /// Successful update or error while executing queries
    async fn sync_rules(&self) -> Result<()>;
}

#[async_trait]
impl RuleManager for Orchestrator {
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
