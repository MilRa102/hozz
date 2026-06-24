use crate::{core::models::rule::Rule, db::SledManager};

pub struct RuleStore;

impl SledManager for RuleStore {
    type Item = Rule;
    const TREE_NAME: &'static str = "forwards";
}

impl RuleStore {
    pub fn upsert(&self, forward: &Rule) -> anyhow::Result<()> {
        self.save(&forward.id, forward)
    }
}
