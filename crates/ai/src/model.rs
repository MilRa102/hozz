use rkyv::{Archive, Deserialize, Serialize};

/// Author of a chat message.
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Lifecycle state of a message, used to track streaming/pause/resume/cancellation.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub enum MessageStatus {
    /// Fully generated (or a plain user message).
    Complete,
    /// Generation is still streaming / was paused mid-stream.
    Partial,
    /// Generation was stopped by the user before completion.
    Cancelled,
    /// Generation failed; the string carries a short error description.
    Error(String),
}

/// A single message inside a [`crate::Conversation`].
///
/// Messages are stored independently of their conversation (see
/// [`crate::MessageStore`]), keyed so that Sled's lexicographic key order matches
/// chronological order (see `store::message_key`).
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub role: Role,
    /// Plain-text content, used for fast UI rendering without JSON parsing.
    pub content: String,
    /// Full-fidelity JSON serialization of the corresponding `rig_core` message
    /// (tool calls, tool results, structured content, etc.). Used to rehydrate
    /// exact multi-turn context when replaying history back into a new agent
    /// request; `content`/`role` alone are not enough for that.
    pub raw: String,
    pub timestamp: i64,
    pub status: MessageStatus,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>, raw: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.into(),
            raw: raw.into(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            status: MessageStatus::Complete,
        }
    }
}

/// Supported LLM providers, user-selectable in settings.
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    Gemini,
    Copilot,
    Ollama,
}

/// Error returned when parsing an unknown [`ProviderKind`] from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProviderKindError(String);

impl std::fmt::Display for ParseProviderKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown provider kind: {}", self.0)
    }
}

impl std::error::Error for ParseProviderKindError {}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Gemini => "gemini",
            Self::Copilot => "copilot",
            Self::Ollama => "ollama",
        })
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = ParseProviderKindError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "gemini" => Ok(Self::Gemini),
            "copilot" => Ok(Self::Copilot),
            "ollama" => Ok(Self::Ollama),
            other => Err(ParseProviderKindError(other.to_string())),
        }
    }
}

/// A chat conversation. Messages are **not** embedded here; they're stored
/// separately (see [`crate::MessageStore`]) and iterated by key prefix.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub provider: ProviderKind,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Conversation {
    pub fn new(title: impl Into<String>, provider: ProviderKind, model: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            folder_id: None,
            provider,
            model: model.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A user-defined folder used to group conversations (flat, no nesting).
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn provider_kind_display_and_from_str_roundtrip() {
        for kind in [ProviderKind::Gemini, ProviderKind::Copilot, ProviderKind::Ollama] {
            let parsed: ProviderKind = kind.to_string().parse().unwrap();
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn provider_kind_from_str_rejects_unknown_value() {
        let error = "unknown".parse::<ProviderKind>().unwrap_err();
        assert_eq!(error.to_string(), "unknown provider kind: unknown");
    }

    #[test]
    fn message_new_defaults_to_complete_status() {
        let message = Message::new(Role::User, "hello", "{}");
        assert_eq!(message.role, Role::User);
        assert_eq!(message.content, "hello");
        assert_eq!(message.status, MessageStatus::Complete);
    }

    #[test]
    fn conversation_new_has_no_folder_and_matching_timestamps() {
        let conversation = Conversation::new("New chat", ProviderKind::Gemini, "gemini-2.5-flash");
        assert!(conversation.folder_id.is_none());
        assert_eq!(conversation.created_at, conversation.updated_at);
        assert_eq!(conversation.provider, ProviderKind::Gemini);
    }

    #[test]
    fn folder_new_generates_unique_ids() {
        let a = Folder::new("Work");
        let b = Folder::new("Work");
        assert_ne!(a.id, b.id);
    }
}
