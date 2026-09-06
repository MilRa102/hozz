use rkyv::{Archive, Deserialize, Serialize};
use uuid::Uuid;

use db::SledManager;

use crate::settings::AiPrefsReader;

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
pub struct MemoryMapEntry {
    pub conversation_id: String,
    pub id: String,
    pub created_at: i64,
    pub last_used_at: i64,
    pub content: String,
    pub embed: Vec<f32>,
}

impl MemoryMapEntry {
    pub fn new(
        conversation_id: impl Into<String>,
        content: impl Into<String>,
        embed: Vec<f32>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            conversation_id: conversation_id.into(),
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
    /// Atomically replaces the memory-map document owned by a conversation.
    pub fn replace_for_conversation(
        &self,
        conversation_id: &str,
        entry: &MemoryMapEntry,
    ) -> anyhow::Result<()> {
        if entry.conversation_id != conversation_id {
            anyhow::bail!(
                "memory-map entry conversation id does not match its storage key"
            );
        }
        Self::save(self, conversation_id, entry)
    }

    /// Removes the memory-map document owned by a conversation.
    pub fn remove_for_conversation(&self, conversation_id: &str) -> anyhow::Result<()> {
        Self::delete(self, conversation_id)
    }

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
        let entry = MemoryMapEntry::new(
            "conversation",
            "remember this",
            vec![0.1, 0.2, 0.3],
        );
        assert_eq!(entry.conversation_id, "conversation");
        assert!(!entry.id.is_empty());
        assert!(entry.created_at > 0);
        assert_eq!(entry.created_at, entry.last_used_at);
        assert_eq!(entry.content, "remember this");
        assert_eq!(entry.embed, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn replacing_a_conversation_keeps_one_current_document() {
        init_db();
        let store = MemoryMapStore;
        let conversation_id = format!("conversation-a-{}", Uuid::new_v4());
        let other_id = format!("conversation-b-{}", Uuid::new_v4());
        let first = MemoryMapEntry::new(&conversation_id, "first", vec![0.1]);
        let replacement = MemoryMapEntry::new(&conversation_id, "replacement", vec![0.2]);
        let other = MemoryMapEntry::new(&other_id, "other", vec![0.3]);

        store
            .replace_for_conversation(&conversation_id, &first)
            .unwrap();
        store
            .replace_for_conversation(&conversation_id, &replacement)
            .unwrap();
        store
            .replace_for_conversation(&other_id, &other)
            .unwrap();

        assert_eq!(
            store
                .recent(100)
                .unwrap()
                .into_iter()
                .filter(|entry| {
                    entry.conversation_id == conversation_id
                        || entry.conversation_id == other_id
                })
                .count(),
            2
        );
        assert_eq!(
            SledManager::get(&store, &conversation_id)
                .unwrap()
                .unwrap()
                .content,
            "replacement"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn removing_a_conversation_keeps_other_documents() {
        init_db();
        let store = MemoryMapStore;
        let conversation_id = format!("conversation-a-{}", Uuid::new_v4());
        let other_id = format!("conversation-b-{}", Uuid::new_v4());
        let first = MemoryMapEntry::new(&conversation_id, "first", vec![0.1]);
        let other = MemoryMapEntry::new(&other_id, "other", vec![0.2]);
        store
            .replace_for_conversation(&conversation_id, &first)
            .unwrap();
        store
            .replace_for_conversation(&other_id, &other)
            .unwrap();

        store
            .remove_for_conversation(&conversation_id)
            .unwrap();

        assert!(
            SledManager::get(&store, &conversation_id)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            SledManager::get(&store, &other_id)
                .unwrap()
                .unwrap()
                .content,
            "other"
        );
    }
}
