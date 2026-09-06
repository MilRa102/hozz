use rkyv::{Archive, Deserialize, Serialize};
use uuid::Uuid;

use db::SledManager;

use crate::settings::AiPrefsReader;

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
pub struct MemoryMapEntry {
    pub id: String,
    pub created_at: i64,
    pub last_used_at: i64,
    pub content: String,
    pub embed: Vec<f32>,
}

impl MemoryMapEntry {
    pub fn new(content: impl Into<String>, embed: Vec<f32>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            last_used_at: now,
            content: content.into(),
            embed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryMapStore;

impl db::SledManager<MemoryMapEntry> for MemoryMapStore {
    const TREE_NAME: &'static str = "ai_memory_map";
}

impl MemoryMapStore {
    pub fn recent(&self, limit: usize) -> anyhow::Result<Vec<MemoryMapEntry>> {
        let tree = self.tree()?;
        let mut items = Vec::new();
        for entry in tree.iter().values().rev() {
            let bytes = entry?;
            let item = Self::decode(&bytes)?;
            items.push(item);
            if items.len() >= limit {
                break;
            }
        }
        Ok(items)
    }

    pub fn is_enabled(prefs: &AiPrefsReader) -> bool {
        prefs.memory_map_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::init_db;

    #[test]
    fn memory_map_entry_has_generated_id_and_timestamps() {
        init_db();
        let entry = MemoryMapEntry::new("remember this", vec![0.1, 0.2, 0.3]);
        assert!(!entry.id.is_empty());
        assert!(entry.created_at > 0);
        assert_eq!(entry.created_at, entry.last_used_at);
        assert_eq!(entry.content, "remember this");
        assert_eq!(entry.embed, vec![0.1, 0.2, 0.3]);
    }
}
