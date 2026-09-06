use std::sync::Arc;

use rig::{
    completion::{
        Message as RigMessage,
        message::{AssistantContent, UserContent},
    },
    memory::{ConversationMemory, DemotionHook, MemoryError},
    wasm_compat::WasmBoxedFuture,
};
use rig_memory::{
    DemotingPolicyMemory, HeuristicTokenCounter, NoopMemoryPolicy, PolicyMemory,
    SlidingWindowMemory, TokenWindowMemory,
};

use crate::{
    engine::history_message,
    model::{Message, Role},
    settings::AiPrefsReader,
    store::MessageStore,
};

/// Token budget used when `ai.memory.max_tokens` is unset or unparseable.
pub(crate) const DEFAULT_MAX_TOKENS: usize = 32_000;

/// Message count used when `ai.memory.max_messages` is unset or unparseable.
pub(crate) const DEFAULT_MAX_MESSAGES: usize = 20;

/// How loaded history is shaped before it reaches the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryPolicyKind {
    /// Send the whole conversation, exactly as stored.
    None,
    /// Keep only the newest messages that fit into a token budget.
    #[default]
    TokenWindow,
    /// Keep only the newest N messages, regardless of their size.
    SlidingWindow,
}

/// Error returned when parsing an unknown [`MemoryPolicyKind`] from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMemoryPolicyKindError(String);

impl std::fmt::Display for ParseMemoryPolicyKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown memory policy kind: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryPolicyKindError {}

impl std::fmt::Display for MemoryPolicyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::TokenWindow => "token",
            Self::SlidingWindow => "sliding",
        })
    }
}

impl std::str::FromStr for MemoryPolicyKind {
    type Err = ParseMemoryPolicyKindError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "token" => Ok(Self::TokenWindow),
            "sliding" => Ok(Self::SlidingWindow),
            other => Err(ParseMemoryPolicyKindError(other.to_string())),
        }
    }
}

/// A [`ConversationMemory`] backend over the app's own Sled message store, so
/// Rig's memory policies shape exactly the history the chat UI already shows.
#[derive(Clone)]
pub struct SledConversationMemory {
    store: Arc<MessageStore>,
}

impl SledConversationMemory {
    pub fn new(store: Arc<MessageStore>) -> Self {
        Self { store }
    }
}

impl ConversationMemory for SledConversationMemory {
    fn load<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> WasmBoxedFuture<'a, Result<Vec<RigMessage>, MemoryError>> {
        Box::pin(async move {
            let messages = self
                .store
                .list(conversation_id)
                .map_err(MemoryError::backend)?;
            Ok(messages.iter().map(history_message).collect())
        })
    }

    /// Unused on the current generation path — the engine persists messages
    /// itself — but required for `DemotingPolicyMemory` and any future handoff
    /// of persistence to Rig.
    fn append<'a>(
        &'a self,
        conversation_id: &'a str,
        messages: Vec<RigMessage>,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            for message in &messages {
                self.store
                    .append(conversation_id, &stored_message(message)?)
                    .map_err(MemoryError::backend)?;
            }
            Ok(())
        })
    }

    fn clear<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            self.store
                .delete_all(conversation_id)
                .map_err(MemoryError::backend)
        })
    }
}

/// Inverse of [`history_message`]: keeps the full-fidelity JSON in `raw` (which
/// is what rehydration prefers) and derives a plain-text `content` for the UI.
fn stored_message(message: &RigMessage) -> Result<Message, MemoryError> {
    let raw = serde_json::to_string(message)
        .map_err(|error| MemoryError::Internal(error.to_string()))?;

    let (role, content) = match message {
        RigMessage::System { content } => (Role::System, content.clone()),
        RigMessage::User { content } => {
            let is_tool_result = content
                .iter()
                .all(|part| matches!(part, UserContent::ToolResult(_)));
            let role = if is_tool_result && !content.is_empty() {
                Role::Tool
            } else {
                Role::User
            };
            (role, user_text(content))
        },
        RigMessage::Assistant { content, .. } => {
            (Role::Assistant, assistant_text(content))
        },
    };

    Ok(Message::new(role, content, raw))
}

fn user_text(content: &[UserContent]) -> String {
    content
        .iter()
        .filter_map(|part| match part {
            UserContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assistant_text(content: &[AssistantContent]) -> String {
    content
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn token_window(prefs: &AiPrefsReader) -> TokenWindowMemory {
    TokenWindowMemory::new(
        prefs.memory_max_tokens(),
        HeuristicTokenCounter::default(),
    )
}

fn sliding_window(prefs: &AiPrefsReader) -> SlidingWindowMemory {
    SlidingWindowMemory::last_messages(prefs.memory_max_messages())
}

/// Builds the conversation memory the engine loads history through, applying
/// the policy configured in settings.
pub fn build_memory(
    prefs: &AiPrefsReader,
    store: Arc<MessageStore>,
) -> Arc<dyn ConversationMemory> {
    let backend = SledConversationMemory::new(store);

    match prefs.memory_policy() {
        MemoryPolicyKind::None => Arc::new(PolicyMemory::new(backend, NoopMemoryPolicy)),
        MemoryPolicyKind::TokenWindow => {
            Arc::new(PolicyMemory::new(backend, token_window(prefs)))
        },
        MemoryPolicyKind::SlidingWindow => {
            Arc::new(PolicyMemory::new(backend, sliding_window(prefs)))
        },
    }
}

/// Same as [`build_memory`], but routes messages evicted by the policy into
/// `hook` instead of dropping them.
///
/// This is the seam for the "memory map": a hook that embeds demoted turns into
/// a vector store. No such hook exists yet, so nothing calls this today.
pub fn build_memory_with_hook<H>(
    prefs: &AiPrefsReader,
    store: Arc<MessageStore>,
    hook: H,
) -> Arc<dyn ConversationMemory>
where
    H: DemotionHook + 'static,
{
    let backend = SledConversationMemory::new(store);

    match prefs.memory_policy() {
        // `NoopMemoryPolicy` never demotes, so the hook would never fire.
        MemoryPolicyKind::None => Arc::new(PolicyMemory::new(backend, NoopMemoryPolicy)),
        MemoryPolicyKind::TokenWindow => Arc::new(DemotingPolicyMemory::new(
            backend,
            token_window(prefs),
            hook,
        )),
        MemoryPolicyKind::SlidingWindow => Arc::new(DemotingPolicyMemory::new(
            backend,
            sliding_window(prefs),
            hook,
        )),
    }
}

#[cfg(test)]
mod tests {
    use rig_memory::MemoryPolicy;

    use super::*;

    fn counted(messages: &[RigMessage]) -> Vec<String> {
        messages
            .iter()
            .map(|message| match message {
                RigMessage::User { content } => user_text(content),
                RigMessage::Assistant { content, .. } => assistant_text(content),
                RigMessage::System { content } => content.clone(),
            })
            .collect()
    }

    fn history() -> Vec<RigMessage> {
        vec![
            RigMessage::user("one"),
            RigMessage::user("two"),
            RigMessage::user("three"),
        ]
    }

    #[test]
    fn memory_policy_kind_round_trips() {
        assert_eq!("none".parse(), Ok(MemoryPolicyKind::None));
        assert_eq!("token".parse(), Ok(MemoryPolicyKind::TokenWindow));
        assert_eq!(
            "sliding".parse(),
            Ok(MemoryPolicyKind::SlidingWindow)
        );
        assert_eq!(MemoryPolicyKind::TokenWindow.to_string(), "token");
        assert_eq!(
            MemoryPolicyKind::SlidingWindow.to_string(),
            "sliding"
        );
        assert!("window".parse::<MemoryPolicyKind>().is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sliding_window_keeps_newest_messages_in_order() {
        let kept = SlidingWindowMemory::last_messages(2)
            .apply(history())
            .unwrap();

        assert_eq!(
            counted(&kept),
            vec!["two".to_string(), "three".to_string()]
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sliding_window_reports_demoted_prefix() {
        let (kept, demoted) = SlidingWindowMemory::last_messages(2)
            .apply_with_demoted(history())
            .unwrap();

        assert_eq!(counted(&demoted), vec!["one".to_string()]);
        assert_eq!(
            counted(&kept),
            vec!["two".to_string(), "three".to_string()]
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn token_window_keeps_newest_messages_in_order() {
        // Budget of 25 with a flat cost of 10 fits exactly two messages.
        let policy = TokenWindowMemory::new(25, |_: &RigMessage| 10usize);
        let kept = policy.apply(history()).unwrap();

        assert_eq!(
            counted(&kept),
            vec!["two".to_string(), "three".to_string()]
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn token_window_reports_demoted_prefix() {
        let policy = TokenWindowMemory::new(25, |_: &RigMessage| 10usize);
        let (kept, demoted) = policy.apply_with_demoted(history()).unwrap();

        // `DemotingPolicyMemory` relies on `demoted` being the dropped prefix.
        assert_eq!(counted(&demoted), vec!["one".to_string()]);
        assert_eq!(
            counted(&kept),
            vec!["two".to_string(), "three".to_string()]
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn noop_policy_keeps_everything() {
        let kept = NoopMemoryPolicy.apply(history()).unwrap();
        assert_eq!(kept.len(), 3);
    }

    #[test]
    fn built_memory_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn ConversationMemory>();
    }
}
