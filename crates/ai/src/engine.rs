use std::{collections::HashMap, sync::Arc};

use futures::StreamExt;
use rig::{
    completion::Message as RigMessage, memory::ConversationMemory,
    tool::server::ToolServerHandle,
};
use serde_json::json;
use tokio::sync::{Mutex, broadcast, watch};

use crate::{
    control::{StreamCommand, StreamControl},
    model::{ConversationUsage, Message, MessageStatus, Role},
    provider::{self, ChatEvent, ProviderConfig},
    settings::AiPrefsReader,
    store::{ConversationStore, ConversationUsageStore, MessageStore},
};

/// Live snapshot of an in-flight generation, published on every text delta so
/// UI code can subscribe (see [`GenerationManager::subscribe`]) without polling.
#[derive(Debug, Clone)]
pub struct GenerationSnapshot {
    pub text: String,
    pub status: GenerationStatus,
    /// Tool calls issued so far this turn, in call order; updated in place as
    /// results arrive so the UI can flip a loader to a status icon live.
    pub tool_calls: Vec<ToolCallView>,
}

impl Default for GenerationSnapshot {
    fn default() -> Self {
        Self {
            text: String::new(),
            status: GenerationStatus::Initializing,
            tool_calls: Vec::new(),
        }
    }
}

/// The mutually exclusive lifecycle states of an in-flight generation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GenerationStatus {
    #[default]
    Initializing,
    LoadingModel,
    Thinking,
    Responding,
    Finished,
    Cancelled,
    Error(String),
}

/// Lifecycle state of a single tool call within a [`GenerationSnapshot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallStatus {
    Running,
    Success,
    Error,
}

/// A tool call's live UI state, keyed by `id` (the correlation handle echoed
/// between [`ChatEvent::ToolCallStarted`] and [`ChatEvent::ToolResultReceived`]).
#[derive(Debug, Clone)]
pub struct ToolCallView {
    pub id: String,
    pub name: String,
    pub status: ToolCallStatus,
}

/// An event emitted while a generation is streaming.
#[derive(Debug, Clone)]
pub enum GenerationEvent {
    Delta(String),
    Thinking(String),
    Finished { text: String, status: MessageStatus },
    Error(String),
}

struct Generation {
    control: Arc<Mutex<Option<Arc<dyn StreamControl>>>>,
    cancel_tx: watch::Sender<bool>,
    snapshot_tx: watch::Sender<GenerationSnapshot>,
}

/// Process-wide registry of in-flight generations, keyed by conversation id.
///
/// Deliberately **not** tied to any UI component's lifetime: a generation is
/// driven by a detached `tokio::spawn` task owned by this manager, so
/// pause/resume/stop (and the Sled writes on completion) keep working even if
/// the user navigates away from the conversation and back before it finishes.
#[derive(Default, Clone)]
pub struct GenerationManager {
    active: Arc<Mutex<HashMap<String, Generation>>>,
}

impl GenerationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn is_generating(&self, conversation_id: &str) -> bool {
        self.active
            .lock()
            .await
            .contains_key(conversation_id)
    }

    /// Subscribes to live updates for a conversation's in-flight generation,
    /// if one is currently running. The returned receiver starts from the
    /// current latest snapshot, so a late subscriber still sees the live state.
    pub async fn subscribe(
        &self,
        conversation_id: &str,
    ) -> Option<watch::Receiver<GenerationSnapshot>> {
        self.active
            .lock()
            .await
            .get(conversation_id)
            .map(|g| g.snapshot_tx.subscribe())
    }

    /// Pauses stream polling; the underlying connection is kept alive. Returns
    /// `false` if no generation is running for this conversation.
    /// Stops the in-flight generation for a conversation; the partial
    /// assistant message is persisted with `MessageStatus::Cancelled`. Returns
    /// `false` if no generation is running for this conversation.
    pub async fn stop(&self, conversation_id: &str) -> bool {
        let generation = self.active.lock().await.remove(conversation_id);
        if let Some(g) = generation {
            let _ = g.cancel_tx.send(true);
            if let Some(ctl) = g.control.lock().await.as_ref() {
                ctl.apply(StreamCommand::Cancel).await;
            }
            true
        } else {
            false
        }
    }

    /// Persists the user's message, starts a streaming generation for it, and
    /// spawns a detached task that streams deltas, publishes snapshots, and
    /// persists the final assistant message once the stream ends.
    #[allow(clippy::too_many_lines)]
    pub async fn start(
        &self,
        conversation_id: String,
        history_store: Arc<MessageStore>,
        _conversation_store: Arc<ConversationStore>,
        request: GenerationRequest,
    ) -> Result<(), String> {
        let (snapshot_tx, _) = watch::channel(GenerationSnapshot::default());
        let (event_tx, _) = broadcast::channel(64);
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let control_slot: Arc<Mutex<Option<Arc<dyn StreamControl>>>> =
            Arc::new(Mutex::new(None));

        // 1. Регистрируем активную генерацию в карте МГНОВЕННО, до асинхронной подгрузки сети
        {
            let mut guard = self.active.lock().await;
            guard.insert(
                conversation_id.clone(),
                Generation {
                    control: control_slot.clone(),
                    cancel_tx: cancel_tx.clone(),
                    snapshot_tx: snapshot_tx.clone(),
                },
            );
        }

        let active_map = self.active.clone();
        let conv_id = conversation_id.clone();
        let event_tx_for_task = event_tx.clone();

        // 2. Запускаем фоновый таск обработки генерации
        tokio::spawn(async move {
            let mut snapshot = GenerationSnapshot {
                status: GenerationStatus::LoadingModel,
                ..Default::default()
            };
            let _ = snapshot_tx.send(snapshot.clone());

            let memory =
                crate::memory::build_memory(&AiPrefsReader, history_store.clone());
            let history: Vec<RigMessage> = match memory.load(&conv_id).await {
                Ok(history) => history,
                Err(error) => {
                    tracing::warn!(%error, conversation_id = %conv_id, "failed to load conversation memory; continuing without history");
                    Vec::new()
                },
            };

            let stream_result = provider::start_stream(
                &request.config,
                &request.model,
                request.system_prompt,
                history,
                request.tools,
                request.max_tool_turns,
            )
            .await;

            let (mut events, control) = match stream_result {
                Ok(res) => res,
                Err(err) => {
                    snapshot.status = GenerationStatus::Error(err.to_string());
                    let _ = snapshot_tx.send(snapshot.clone());
                    let _ =
                        event_tx_for_task.send(GenerationEvent::Error(err.to_string()));
                    let _ = event_tx_for_task.send(GenerationEvent::Finished {
                        text: String::new(),
                        status: MessageStatus::Error(err.to_string()),
                    });

                    active_map.lock().await.remove(&conv_id);
                    return;
                },
            };

            // Сохраняем StreamControl для паузы/возобновления
            {
                let mut ctl_guard = control_slot.lock().await;
                *ctl_guard = Some(control);
            }

            snapshot.status = GenerationStatus::Thinking;
            let _ = snapshot_tx.send(snapshot.clone());
            let _ = event_tx_for_task.send(GenerationEvent::Thinking(
                "Размышляет...".to_string(),
            ));

            let mut was_cancelled = false;
            let mut last_raw = "{}".to_string();
            let mut last_error: Option<String> = None;
            let mut usage = ConversationUsage::default();

            // 3. Цикл получения чанков с неблокирующим перехватом отмены через select!
            loop {
                if *cancel_rx.borrow() {
                    was_cancelled = true;
                    break;
                }

                tokio::select! {
                    _ = cancel_rx.changed() => {
                        if *cancel_rx.borrow() {
                            was_cancelled = true;
                            break;
                        }
                    }
                    maybe_event = events.next() => {
                        match maybe_event {
                            Some(ChatEvent::Delta(delta)) => {
                                snapshot.status = GenerationStatus::Responding;
                                snapshot.text.push_str(&delta);
                                let _ = snapshot_tx.send(snapshot.clone());
                                let _ = event_tx_for_task.send(GenerationEvent::Delta(delta));
                            }
                            Some(ChatEvent::Reasoning(reasoning)) => {
                                snapshot.status = GenerationStatus::Thinking;
                                let _ = snapshot_tx.send(snapshot.clone());
                                let _ = event_tx_for_task.send(GenerationEvent::Thinking(reasoning));
                            }
                            Some(ChatEvent::ToolCallStarted { id, name, arguments }) => {
                                let raw = json!({
                                    "function": {
                                        "name": name,
                                        "arguments": arguments,
                                    },
                                    "tool_call_id": id,
                                }).to_string();
                                let msg = Message::new(Role::Tool, format!("Tool: {name}"), raw);
                                let _ = history_store.append(&conv_id, &msg);

                                snapshot.tool_calls.push(ToolCallView {
                                    id,
                                    name,
                                    status: ToolCallStatus::Running,
                                });
                                let _ = snapshot_tx.send(snapshot.clone());
                            }
                            Some(ChatEvent::ToolResultReceived { id, name, payload }) => {
                                let is_error = provider::tool_result_is_error(&payload);
                                let status = if is_error {
                                    ToolCallStatus::Error
                                } else {
                                    ToolCallStatus::Success
                                };

                                // Match by id when known, else fall back to the oldest still-running
                                // call (the id-less unknown-chunk path never sends a name either).
                                let matched_index = snapshot.tool_calls.iter().position(|entry| {
                                    (!id.is_empty() && entry.id == id)
                                        || (id.is_empty() && entry.status == ToolCallStatus::Running)
                                });
                                let resolved_name = matched_index
                                    .map(|i| snapshot.tool_calls[i].name.clone())
                                    .filter(|n| !n.is_empty())
                                    .unwrap_or(name);
                                if let Some(i) = matched_index {
                                    snapshot.tool_calls[i].status = status;
                                }
                                let _ = snapshot_tx.send(snapshot.clone());

                                let raw = json!({
                                    "name": resolved_name,
                                    "result": payload,
                                    "tool_call_id": id,
                                    "status": if is_error { "error" } else { "success" },
                                }).to_string();
                                let msg = Message::new(Role::Tool, format!("Tool result: {resolved_name}"), raw);
                                let _ = history_store.append(&conv_id, &msg);
                            }
                            Some(ChatEvent::Usage(call_usage)) => {
                                usage.input_tokens += call_usage.input_tokens;
                                usage.output_tokens += call_usage.output_tokens;
                                usage.total_tokens += call_usage.total_tokens;
                                usage.cached_input_tokens += call_usage.cached_input_tokens;
                                usage.cache_creation_input_tokens += call_usage.cache_creation_input_tokens;
                                usage.reasoning_tokens += call_usage.reasoning_tokens;
                            }
                            Some(ChatEvent::Done { text, raw }) => {
                                snapshot.text = text.clone();
                                last_raw = raw;
                                break;
                            }
                            Some(ChatEvent::Error(err)) => {
                                if *cancel_rx.borrow() {
                                    was_cancelled = true;
                                    break;
                                }
                                last_error = Some(err.clone());
                                snapshot.status = GenerationStatus::Error(err.clone());
                                let _ = snapshot_tx.send(snapshot.clone());
                                let _ = event_tx_for_task.send(GenerationEvent::Error(err));
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }

            snapshot.status = if was_cancelled {
                GenerationStatus::Cancelled
            } else if let Some(error) = last_error.clone() {
                GenerationStatus::Error(error)
            } else {
                GenerationStatus::Finished
            };
            let _ = snapshot_tx.send(snapshot.clone());

            let status = if was_cancelled {
                MessageStatus::Cancelled
            } else if let Some(ref err) = last_error {
                MessageStatus::Error(err.clone())
            } else {
                MessageStatus::Complete
            };

            let _ = event_tx_for_task.send(GenerationEvent::Finished {
                text: snapshot.text.clone(),
                status: status.clone(),
            });

            // Сохраняем сформированный или частично отмененный ответ в Sled DB
            if !snapshot.text.is_empty() || was_cancelled {
                let msg = Message::new(Role::Assistant, snapshot.text, last_raw);
                let _ = history_store.append(&conv_id, &msg);
            }

            if usage.total_tokens > 0 {
                let usage_store = ConversationUsageStore;
                let mut aggregate = usage_store
                    .find(&conv_id)
                    .unwrap_or_default()
                    .unwrap_or_default();
                aggregate.input_tokens += usage.input_tokens;
                aggregate.output_tokens += usage.output_tokens;
                aggregate.total_tokens += usage.total_tokens;
                aggregate.cached_input_tokens += usage.cached_input_tokens;
                aggregate.cache_creation_input_tokens +=
                    usage.cache_creation_input_tokens;
                aggregate.reasoning_tokens += usage.reasoning_tokens;
                let _ = usage_store.upsert(&conv_id, &aggregate);
            }

            active_map.lock().await.remove(&conv_id);
        });

        Ok(())
    }
}

pub struct GenerationRequest {
    pub config: ProviderConfig,
    pub model: String,
    pub system_prompt: String,
    pub tools: Option<ToolServerHandle>,
    pub max_tool_turns: usize,
}

/// Rehydrates a persisted [`Message`] back into a `rig_core` message for
/// multi-turn context, preferring the full-fidelity `raw` JSON and falling
/// back to plain text if it's missing or fails to parse (e.g. older data).
pub fn history_message(msg: &Message) -> RigMessage {
    if !msg.raw.is_empty()
        && msg.raw != "{}"
        && let Ok(parsed) = serde_json::from_str::<RigMessage>(&msg.raw)
    {
        return parsed;
    }

    match msg.role {
        Role::User => RigMessage::user(&msg.content),
        Role::Assistant => RigMessage::assistant(&msg.content),
        Role::System => RigMessage::user(&msg.content),
        Role::Tool => RigMessage::user(&msg.content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_events_can_model_text_deltas() {
        let event = GenerationEvent::Delta("hello".to_string());
        assert!(matches!(event, GenerationEvent::Delta(ref text) if text == "hello"));
    }

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
            },
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

        assert_eq!(
            history_message(&msg),
            RigMessage::user("fallback")
        );
    }

    #[tokio::test]
    async fn manager_control_methods_return_false_when_no_generation_exists() {
        let manager = GenerationManager::new();
        assert!(!manager.stop("missing").await);
    }

    #[test]
    fn tool_event_payloads_are_json_serializable() {
        let event = serde_json::json!({
            "function": {
                "name": "proxy_status",
                "arguments": {}
            }
        });
        let raw = serde_json::to_string(&event).unwrap_or_default();
        assert!(raw.contains("proxy_status"));
    }

    #[test]
    fn generation_snapshot_tracks_terminal_error() {
        let snapshot = GenerationSnapshot {
            status: GenerationStatus::Error("provider unavailable".to_string()),
            ..Default::default()
        };

        assert!(matches!(
            snapshot.status,
            GenerationStatus::Error(ref error) if error == "provider unavailable"
        ));
    }
}
