use std::{collections::HashMap, sync::Arc};

use crate::{
    PreferenceHook,
    meta::{Category, SettingMeta},
};

#[derive(Default, Clone)]
pub struct SettingsRegistry<A> {
    hooks: HashMap<String, Arc<dyn PreferenceHook<A> + Send + Sync>>,
    items: Vec<SettingMeta>,
    tags: Vec<&'static str>,
}

impl<A> SettingsRegistry<A>
where
    A: Send + Sync + 'static,
{
    fn recalculate_tags(&mut self) {
        let mut counts: HashMap<&'static str, usize> = HashMap::new();

        for item in &self.items {
            for tag in item.tags {
                *counts.entry(tag).or_insert(0) += 1;
            }
        }

        let mut sorted_tags: Vec<(&'static str, usize)> = counts.into_iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        self.tags = sorted_tags
            .into_iter()
            .take(8)
            .map(|(tag, _count)| tag)
            .collect();
    }

    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
            items: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn register<T>(&mut self, hook: T)
    where
        T: PreferenceHook<A> + Send + Sync + 'static,
    {
        let meta = hook.meta();
        let id = meta.id.to_string();

        self.items.push(meta);
        self.hooks.insert(id, Arc::new(hook));
        self.recalculate_tags();
    }

    pub fn get_hook(&self, id: &str) -> Option<Arc<dyn PreferenceHook<A> + Send + Sync>> {
        self.hooks.get(id).cloned()
    }

    pub fn all(&self) -> Vec<SettingMeta> {
        self.items.clone()
    }

    pub fn all_tags(&self) -> Vec<&'static str> {
        self.tags.clone()
    }

    /// Smart string search
    pub fn search(&self, query: &str) -> Vec<SettingMeta> {
        if query.trim().is_empty() {
            return self.items.to_vec();
        }

        let q = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&q)
                    || item.description.to_lowercase().contains(&q)
                    || item
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    /// Grouping settings by section
    pub fn by_category(&self, category: Category) -> Vec<SettingMeta> {
        self.items
            .iter()
            .filter(|i| i.category == category)
            .cloned()
            .collect()
    }

    pub fn by_id(&self, id: &str) -> Option<SettingMeta> {
        self.items.iter().find(|i| i.id == id).cloned()
    }
}
