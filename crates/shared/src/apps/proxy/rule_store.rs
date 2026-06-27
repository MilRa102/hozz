use db::SledManager;

use crate::core::models::rule::Rule;

pub struct RuleStore;

impl SledManager<Rule> for RuleStore {
    const TREE_NAME: &'static str = "forwards";
}

impl RuleStore {
    pub fn upsert(&self, forward: &Rule) -> anyhow::Result<()> {
        self.save(&forward.id, forward)
    }

    pub fn extract(&self) -> Vec<Rule> {
        self.all().unwrap_or_default()
    }
}
