use db::SledManager;

use crate::model::{Conversation, Folder, Message};

/// Conversations and folders each get their own dedicated Sled tree, keyed
/// directly by id — no risk of mixing archived byte layouts of different types.
const CONVERSATIONS_TREE: &str = "ai_conversations";
const FOLDERS_TREE: &str = "ai_folders";

/// Messages from every conversation share one tree, namespaced by key prefix so a
/// single conversation's messages can be scanned without decoding the whole tree.
/// Key: `<conversation_uuid>:<timestamp_ms padded>:<message_uuid>`. The zero-padded
/// timestamp keeps Sled's lexicographic key order equal to chronological order, and
/// the trailing message id breaks ties for messages created in the same millisecond.
const MESSAGES_TREE: &str = "ai_messages";

fn message_key(conversation_id: &str, timestamp_ms: i64, message_id: &str) -> String {
    format!("{conversation_id}:{timestamp_ms:019}:{message_id}")
}

fn message_prefix(conversation_id: &str) -> String {
    format!("{conversation_id}:")
}

pub struct ConversationStore;

impl SledManager<Conversation> for ConversationStore {
    const TREE_NAME: &'static str = CONVERSATIONS_TREE;
}

impl ConversationStore {
    pub fn list(&self) -> anyhow::Result<Vec<Conversation>> {
        SledManager::all(self)
    }

    pub fn find(&self, id: &str) -> anyhow::Result<Option<Conversation>> {
        SledManager::get(self, id)
    }

    pub fn upsert(&self, conversation: &Conversation) -> anyhow::Result<()> {
        SledManager::save(self, &conversation.id, conversation)
    }

    /// Deletes the conversation and (atomically) all of its messages.
    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        SledManager::delete(self, id)?;
        MessageStore.delete_all(id)
    }

    /// Unassigns every conversation currently in `folder_id` (used when a folder
    /// is deleted — conversations are kept, just moved back to "no folder").
    pub fn unassign_folder(&self, folder_id: &str) -> anyhow::Result<()> {
        for mut conversation in self.list()? {
            if conversation.folder_id.as_deref() == Some(folder_id) {
                conversation.folder_id = None;
                self.upsert(&conversation)?;
            }
        }
        Ok(())
    }
}

pub struct MessageStore;

impl SledManager<Message> for MessageStore {
    const TREE_NAME: &'static str = MESSAGES_TREE;
}

impl MessageStore {
    pub fn append(&self, conversation_id: &str, message: &Message) -> anyhow::Result<()> {
        let key = message_key(conversation_id, message.timestamp, &message.id);
        SledManager::save(self, &key, message)
    }

    /// Lists every message of a conversation in chronological order.
    pub fn list(&self, conversation_id: &str) -> anyhow::Result<Vec<Message>> {
        SledManager::scan_prefix(self, &message_prefix(conversation_id))
    }

    /// Atomically deletes every message belonging to a conversation.
    pub(crate) fn delete_all(&self, conversation_id: &str) -> anyhow::Result<()> {
        SledManager::delete_prefix(self, &message_prefix(conversation_id))
    }
}

pub struct FolderStore;

impl SledManager<Folder> for FolderStore {
    const TREE_NAME: &'static str = FOLDERS_TREE;
}

impl FolderStore {
    pub fn list(&self) -> anyhow::Result<Vec<Folder>> {
        SledManager::all(self)
    }

    pub fn find(&self, id: &str) -> anyhow::Result<Option<Folder>> {
        SledManager::get(self, id)
    }

    pub fn upsert(&self, folder: &Folder) -> anyhow::Result<()> {
        SledManager::save(self, &folder.id, folder)
    }

    /// Deletes the folder; conversations that referenced it are unassigned, not deleted.
    pub fn remove(&self, id: &str) -> anyhow::Result<()> {
        SledManager::delete(self, id)?;
        ConversationStore.unassign_folder(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{ProviderKind, Role},
        test_support::init_db,
    };

    #[test]
    #[allow(clippy::unwrap_used)]
    fn conversation_crud_roundtrip() {
        init_db();
        let store = ConversationStore;
        let conversation = Conversation::new("Test", ProviderKind::Ollama, "llama3");
        store.upsert(&conversation).unwrap();

        assert_eq!(
            store.find(&conversation.id).unwrap(),
            Some(conversation.clone())
        );

        store.remove(&conversation.id).unwrap();
        assert_eq!(store.find(&conversation.id).unwrap(), None);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn folder_crud_roundtrip() {
        init_db();
        let store = FolderStore;
        let folder = Folder::new("Work");
        store.upsert(&folder).unwrap();

        assert_eq!(
            store.find(&folder.id).unwrap(),
            Some(folder.clone())
        );

        store.remove(&folder.id).unwrap();
        assert_eq!(store.find(&folder.id).unwrap(), None);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn messages_are_listed_in_chronological_order() {
        init_db();
        let conversation = Conversation::new(
            "Order test",
            ProviderKind::Gemini,
            "gemini-2.5-flash",
        );
        ConversationStore.upsert(&conversation).unwrap();

        let mut first = Message::new(Role::User, "first", "{}");
        first.timestamp = 1_000;
        let mut second = Message::new(Role::Assistant, "second", "{}");
        second.timestamp = 2_000;

        // Appended out of order on purpose.
        MessageStore
            .append(&conversation.id, &second)
            .unwrap();
        MessageStore
            .append(&conversation.id, &first)
            .unwrap();

        let messages = MessageStore.list(&conversation.id).unwrap();
        let contents: Vec<&str> = messages
            .iter()
            .map(|m| m.content.as_str())
            .collect();
        assert_eq!(contents, vec!["first", "second"]);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn deleting_conversation_removes_its_messages_only() {
        init_db();
        let conversation =
            Conversation::new("Delete test", ProviderKind::Copilot, "gpt-5");
        let other = Conversation::new("Other", ProviderKind::Copilot, "gpt-5");
        ConversationStore.upsert(&conversation).unwrap();
        ConversationStore.upsert(&other).unwrap();

        MessageStore
            .append(
                &conversation.id,
                &Message::new(Role::User, "hi", "{}"),
            )
            .unwrap();
        MessageStore
            .append(
                &conversation.id,
                &Message::new(Role::Assistant, "hello", "{}"),
            )
            .unwrap();
        MessageStore
            .append(
                &other.id,
                &Message::new(Role::User, "unrelated", "{}"),
            )
            .unwrap();

        ConversationStore
            .remove(&conversation.id)
            .unwrap();

        assert!(
            MessageStore
                .list(&conversation.id)
                .unwrap()
                .is_empty()
        );
        assert_eq!(MessageStore.list(&other.id).unwrap().len(), 1);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn deleting_folder_unassigns_its_conversations() {
        init_db();
        let folder = Folder::new("Archive");
        FolderStore.upsert(&folder).unwrap();

        let mut conversation = Conversation::new(
            "In folder",
            ProviderKind::Gemini,
            "gemini-2.5-flash",
        );
        conversation.folder_id = Some(folder.id.clone());
        ConversationStore.upsert(&conversation).unwrap();

        FolderStore.remove(&folder.id).unwrap();

        let reloaded = ConversationStore
            .find(&conversation.id)
            .unwrap()
            .unwrap();
        assert_eq!(reloaded.folder_id, None);
        assert!(FolderStore.find(&folder.id).unwrap().is_none());
    }
}
