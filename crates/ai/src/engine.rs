use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt;
use futures::stream::BoxStream;
use rig_core::completion::Message as RigMessage;
use rig_core::tool::ToolDyn;
use tokio::sync::{Mutex, watch};

use crate::control::StreamControl;
use crate::model::{Message, MessageStatus, Role};
use crate::provider::{self, ChatEvent, ProviderConfig};
use crate::store::{ConversationStore, MessageStore};

/// Live snapshot of an in-flight generation, published on every text delta so
/// UI code can subscribe (see [`GenerationManager::subscribe`]) without polling.
#[derive(Debug, Clone, Default)]
pub struct GenerationSnapshot {
    pub text: String,
    pub finished: bool,
}

struct Generation {
    control: Arc<dyn StreamControl>,
    cancelled: Arc<AtomicBool>,
    snapshot_tx: watch::Sender<GenerationSnapshot>,
}

/// Process-wide registry of in-flight generations, keyed by conversation id.
///
/// Deliberately **not** tied to any UI component's lifetime: a generation is
/// driven by a detached `tokio::spawn` task owned by this manager, so
/// pause/resume/stop (and the Sled writes on completion) keep working even if
/// the user navigates away from the conversation and back before it finishes.
#[derive(Default)]
pub struct GenerationManager {
    active: Mutex<HashMap<String, Generation>>,
}

impl GenerationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn is_generating(&self, conversation_id: &str) -> bool {
        self.active.lock().await.contains_key(conversation_id)
    }

    /// Subscribes to live updates for a conversation's in-flight generation,
    /// if one is currently running.
    pub async fn subscribe(&self, conversation_id: &str) -> Option<watch::Receiver<GenerationSnapshot>> {
        self.active
            .lock()
            .await
            .get(conversation_id)
            .map(|generation| generation.snapshot_tx.subscribe())
    }

    /// Pauses stream polling; the underlying connection is kept alive. Returns
    /// `false` if no generation is running for this conversation.
    pub async fn pause(&self, conversation_id: &str) -> bool {
        match self.active.lock().await.get(conversation_id) {
            Some(generation) => {
                generation.control.pause().await;
                true
            }
            None => false,
        }
    }

    /// Resumes a previously paused generation. Returns `false` if no
    /// generation is running for this conversation.
    pub async fn resume(&self, conversation_id: &str) -> bool {
        match self.active.lock().await.get(conversation_id) {
            Some(generation) => {
                generation.control.resume().await;
                true
            }
            None => false,
        }
    }

    /// Stops the in-flight generation for a conversation; the partial
    /// assistant message is persisted with `MessageStatus::Cancelled`. Returns
    /// `false` if no generation is running for this conversation.
    pub async fn stop(&self, conversation_id: &str) -> bool {
        match self.active.lock().await.get(conversation_id) {
            Some(generation) => {
                generation.cancelled.store(true, Ordering::SeqCst);
                generation.control.cancel().await;
                true
            }
            None => false,
        }
    }

    /// Persists the user's message, starts a streaming generation for it, and
    /// spawns a detached task that streams deltas, publishes snapshots, and
    /// persists the final assistant message once the stream ends.
    pub async fn start(
        self: &Arc<Self>,
        conversation_id: String,
        config: ProviderConfig,
        model: String,
        tools: Vec<Box<dyn ToolDyn>>,
        prompt: String,
    ) -> anyhow::Result<()> {
        if self.is_generating(&conversation_id).await {
            anyhow::bail!("A generation is already running for this conversation");
        }

        let history = match MessageStore.list(&conversation_id) {
            Ok(messages) => messages.iter().map(history_message).collect::<Vec<_>>(),
            Err(error) => {
                tracing::warn!(%conversation_id, %error, "Failed to load conversation history; starting with empty history");
                Vec::new()
            }
        };

        let user_raw = serde_json::to_string(&RigMessage::user(prompt.clone())).unwrap_or_default();
        let user_message = Message::new(Role::User, prompt.clone(), user_raw);
        if let Err(error) = MessageStore.append(&conversation_id, &user_message) {
            tracing::warn!(%conversation_id, %error, "Failed to persist user message");
        }

        let (events, control) = provider::start_stream(&config, &model, tools, prompt, history).await?;

        let (snapshot_tx, _snapshot_rx) = watch::channel(GenerationSnapshot::default());
        let cancelled = Arc::new(AtomicBool::new(false));

        self.active.lock().await.insert(
            conversation_id.clone(),
            Generation {
                control,
                cancelled: cancelled.clone(),
                snapshot_tx: snapshot_tx.clone(),
            },
        );

        let manager = self.clone();
        tokio::spawn(async move {
            manager.drive(conversation_id, events, snapshot_tx, cancelled).await;
        });

        Ok(())
    }

    async fn drive(
        self: Arc<Self>,
        conversation_id: String,
        mut events: BoxStream<'static, ChatEvent>,
        snapshot_tx: watch::Sender<GenerationSnapshot>,
        cancelled: Arc<AtomicBool>,
    ) {
        let mut text = String::new();
        let mut raw = String::new();
        let mut status = MessageStatus::Complete;

        while let Some(event) = events.next().await {
            match event {
                ChatEvent::Delta(delta) => {
                    text.push_str(&delta);
                    let _ = snapshot_tx.send(GenerationSnapshot {
                        text: text.clone(),
                        finished: false,
                    });
                }
                ChatEvent::ToolCallStarted { name, .. } => {
                    tracing::debug!(%conversation_id, tool = %name, "Tool call started");
                }
                ChatEvent::Done { text: final_text, raw: final_raw } => {
                    text = final_text;
                    raw = final_raw;
                    status = if cancelled.load(Ordering::SeqCst) {
                        MessageStatus::Cancelled
                    } else {
                        MessageStatus::Complete
                    };
                }
                ChatEvent::Error(error) => {
                    tracing::warn!(%conversation_id, %error, "Generation failed");
                    status = MessageStatus::Error(error);
                }
            }
        }

        if raw.is_empty() {
            raw = serde_json::to_string(&RigMessage::assistant(text.clone())).unwrap_or_default();
        }

        let message = Message {
            status: status.clone(),
            ..Message::new(Role::Assistant, text, raw)
        };

        if let Err(error) = MessageStore.append(&conversation_id, &message) {
            tracing::warn!(%conversation_id, %error, "Failed to persist assistant message");
        }
        match ConversationStore.find(&conversation_id) {
            Ok(Some(mut conversation)) => {
                conversation.updated_at = message.timestamp;
                if let Err(error) = ConversationStore.upsert(&conversation) {
                    tracing::warn!(%conversation_id, %error, "Failed to update conversation timestamp");
                }
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%conversation_id, %error, "Failed to load conversation to update timestamp");
            }
        }

        let _ = snapshot_tx.send(GenerationSnapshot {
            text: message.content.clone(),
            finished: true,
        });
        self.active.lock().await.remove(&conversation_id);
    }
}

/// Rehydrates a persisted [`Message`] back into a `rig_core` message for
/// multi-turn context, preferring the full-fidelity `raw` JSON and falling
/// back to plain text if it's missing or fails to parse (e.g. older data).
fn history_message(message: &Message) -> RigMessage {
    serde_json::from_str(&message.raw).unwrap_or_else(|_| match message.role {
        Role::User => RigMessage::user(message.content.clone()),
        Role::Assistant | Role::Tool => RigMessage::assistant(message.content.clone()),
        Role::System => RigMessage::System {
            content: message.content.clone(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn history_message_uses_raw_json_when_valid() {
        let raw = serde_json::to_string(&RigMessage::assistant("from raw")).unwrap();
        let msg = Message {
            raw,
            ..Message::new(Role::Assistant, "from content", "{}")
        };

        match history_message(&msg) {
            RigMessage::Assistant { content, .. } => {
                let payload = serde_json::to_string(&content).unwrap_or_default();
                assert!(payload.contains("from raw"));
            }
            other => panic!("Expected assistant message, got {other:?}"),
        }
    }

    #[test]
    fn history_message_falls_back_to_role_and_content_when_raw_is_invalid() {
        let msg = Message {
            role: Role::User,
            content: "fallback".to_string(),
            raw: "not-json".to_string(),
            ..Message::new(Role::User, "ignored", "{}")
        };

        assert_eq!(history_message(&msg), RigMessage::user("fallback"));
    }

    #[tokio::test]
    async fn manager_control_methods_return_false_when_no_generation_exists() {
        let manager = Arc::new(GenerationManager::new());
        assert!(!manager.pause("missing").await);
        assert!(!manager.resume("missing").await);
        assert!(!manager.stop("missing").await);
        assert!(manager.subscribe("missing").await.is_none());
    }
}
