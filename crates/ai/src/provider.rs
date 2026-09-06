use std::sync::Arc;

use futures::{
    future::{AbortHandle, Abortable},
    stream::{self, BoxStream, StreamExt},
};
use rig::{
    agent::MultiTurnStreamItem,
    client::{AgentClientExt, Nothing},
    completion::{
        Message as RigMessage,
        message::{Reasoning, ReasoningContent},
    },
    providers::{copilot, gemini, ollama},
    streaming::{StreamedAssistantContent, StreamedUserContent},
    tool::server::ToolServerHandle,
};

use crate::{
    control::{ResponseControl, StreamControl},
    model::{ConversationUsage, ProviderKind},
};

/// A normalized chunk of a streaming generation, provider-agnostic — this is
/// the boundary past which none of the provider-specific response types (`R`
/// in `rig_core`) leak into the rest of the `ai` crate.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Plain text delta.
    Delta(String),
    /// Reasoning/thinking delta, if the provider exposes one.
    Reasoning(String),
    /// A complete tool call requested by the model.
    ToolCallStarted {
        /// Correlation handle echoed by the matching `ToolResultReceived`; empty
        /// when the path that produced this event can't recover one.
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Tool/hosted-tool result or provider-native tool payload surfaced
    /// through unknown stream chunks.
    ToolResultReceived {
        /// Echoes the originating `ToolCallStarted::id`; empty when unknown.
        id: String,
        name: String,
        payload: serde_json::Value,
    },
    /// Usage reported after one completed model call in a multi-turn run.
    Usage(ConversationUsage),
    /// The stream ended (naturally or via `StreamControl::cancel`). Carries the
    /// full accumulated text and a JSON-serialized `rig_core` assistant
    /// message (see `Message.raw`) for rehydrating exact multi-turn context.
    /// The caller (the engine) — not this module — decides whether this means
    /// `MessageStatus::Complete` or `MessageStatus::Cancelled`, since a
    /// cancelled `Abortable` stream simply ends like any other.
    Done { text: String, raw: String },
    /// A provider/transport error terminated the stream.
    Error(String),
}

/// Resolved per-provider connection settings, read from [`crate::AiPrefsReader`]
/// by the engine before starting a generation.
#[derive(Clone, Debug)]
pub enum ProviderConfig {
    Gemini { api_key: String },
    Copilot { api_key: String },
    Ollama { base_url: String },
}

impl ProviderConfig {
    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Gemini { .. } => ProviderKind::Gemini,
            Self::Copilot { .. } => ProviderKind::Copilot,
            Self::Ollama { .. } => ProviderKind::Ollama,
        }
    }
}

/// Starts a streaming generation for `prompt` given the conversation `history`
/// (already-recorded messages, oldest first). Returns a boxed stream of
/// normalized [`ChatEvent`]s plus a type-erased [`StreamControl`] handle that
/// stays valid for the lifetime of the generation, independent of who's
/// polling the stream.
pub async fn start_stream(
    config: &ProviderConfig,
    model: &str,
    prompt: String,
    history: Vec<RigMessage>,
    tool_server: Option<ToolServerHandle>,
    max_tool_turns: usize,
) -> anyhow::Result<(
    BoxStream<'static, ChatEvent>,
    Arc<dyn StreamControl>,
)> {
    match config {
        ProviderConfig::Gemini { api_key } => {
            let client = gemini::Client::new(api_key).map_err(|error| {
                anyhow::anyhow!("Failed to create Gemini client: {error}")
            })?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client
                    .agent(model)
                    .tool_server_handle(tool_server)
                    .build();
                run(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history, max_tool_turns).await
            }
        },
        ProviderConfig::Copilot { api_key } => {
            let client = copilot::Client::builder()
                .api_key(copilot::CopilotAuth::ApiKey(api_key.clone()))
                .build()
                .map_err(|error| {
                    anyhow::anyhow!("Failed to create Copilot client: {error}")
                })?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client
                    .agent(model)
                    .tool_server_handle(tool_server)
                    .build();
                run(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history, max_tool_turns).await
            }
        },
        ProviderConfig::Ollama { base_url } => {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(base_url)
                .build()
                .map_err(|error| {
                    anyhow::anyhow!("Failed to create Ollama client: {error}")
                })?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client
                    .agent(model)
                    .tool_server_handle(tool_server)
                    .build();
                run(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history, max_tool_turns).await
            }
        },
    }
}

fn reasoning_text(reasoning: Reasoning) -> String {
    reasoning
        .content
        .into_iter()
        .filter_map(|part| match part {
            ReasoningContent::Text { text, .. } => Some(text),
            ReasoningContent::Summary(text) => Some(text),
            ReasoningContent::Encrypted(_) | ReasoningContent::Redacted { .. } => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_text_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        },
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(extract_text_from_value)
            .collect::<Vec<_>>()
            .join(" ")
            .into(),
        serde_json::Value::Object(map) => {
            for key in [
                "text",
                "delta",
                "content",
                "value",
                "output",
                "output_text",
                "message",
                "answer",
                "response",
            ] {
                if let Some(child) = map.get(key)
                    && let Some(text) = extract_text_from_value(child)
                {
                    return Some(text);
                }
            }

            for child in map.values() {
                if let Some(text) = extract_text_from_value(child) {
                    return Some(text);
                }
            }

            None
        },
        _ => None,
    }
}

fn tool_name_from_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in ["name", "tool_name", "tool"] {
                if let Some(serde_json::Value::String(name)) = map.get(key) {
                    let name = name.trim();
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }

            for key in ["function", "tool_call", "tool_result"] {
                if let Some(child) = map.get(key)
                    && let Some(name) = tool_name_from_value(child)
                {
                    return Some(name);
                }
            }

            for child in map.values() {
                if let Some(name) = tool_name_from_value(child) {
                    return Some(name);
                }
            }

            None
        },
        serde_json::Value::Array(values) => {
            for child in values {
                if let Some(name) = tool_name_from_value(child) {
                    return Some(name);
                }
            }
            None
        },
        _ => None,
    }
}

fn event_from_unknown_chunk(value: serde_json::Value) -> Option<ChatEvent> {
    let serde_json::Value::Object(map) = &value else {
        return None;
    };

    let chunk_type = map
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let looks_tool_payload = chunk_type.contains("tool")
        || chunk_type.ends_with("_call")
        || chunk_type.ends_with("_result")
        || map.contains_key("tool")
        || map.contains_key("tool_name")
        || map.contains_key("tool_result")
        || map.contains_key("call_id")
        || map.contains_key("arguments")
        || map.contains_key("output")
        || map.contains_key("result");

    if !looks_tool_payload {
        return None;
    }

    let name = tool_name_from_value(&value)
        .or_else(|| {
            if chunk_type.is_empty() {
                None
            } else {
                Some(chunk_type.to_string())
            }
        })
        .unwrap_or_else(|| "tool_result".to_string());

    let id = map
        .get("call_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    Some(ChatEvent::ToolResultReceived {
        id,
        name,
        payload: value,
    })
}

/// Best-effort success/error classification for a tool result payload — `rig`
/// doesn't carry a structured success flag, so this looks for an `error`/
/// `failure` key or telltale wording in the payload's text.
pub(crate) fn tool_result_is_error(payload: &serde_json::Value) -> bool {
    fn contains_error_marker(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                map.contains_key("error")
                    || map.contains_key("failure")
                    || map.values().any(contains_error_marker)
            },
            serde_json::Value::Array(values) => values.iter().any(contains_error_marker),
            serde_json::Value::String(text) => {
                let lower = text.to_lowercase();
                lower.contains("error")
                    || lower.contains("failed")
                    || lower.contains("ошибка")
            },
            _ => false,
        }
    }

    contains_error_marker(payload)
}

fn missing_final_response_error(stream_error: Option<String>) -> String {
    stream_error.unwrap_or_else(|| {
        "stream ended before Rig reported a final response".to_string()
    })
}

#[allow(clippy::too_many_lines)]
async fn run(
    agent: rig::agent::Agent,
    prompt: String,
    history: Vec<RigMessage>,
    max_tool_turns: usize,
) -> anyhow::Result<(
    BoxStream<'static, ChatEvent>,
    Arc<dyn StreamControl>,
)> {
    let response = agent
        .runner(prompt)
        .history(history)
        .max_turns(max_tool_turns.max(1))
        .stream()
        .await;

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let stream = Box::pin(Abortable::new(response, abort_registration));
    let control: Arc<dyn StreamControl> = Arc::new(ResponseControl(abort_handle));
    let state = (stream, String::new(), false, None::<String>);

    let events = stream::unfold(
        state,
        |(mut stream, mut accumulated, mut done, mut stream_error)| async move {
            if done {
                return None;
            }

            loop {
                match stream.next().await {
                    Some(Ok(event)) => match event {
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text),
                        ) => {
                            accumulated.push_str(&text.text);
                            return Some((
                                ChatEvent::Delta(text.text),
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Reasoning { reasoning, .. },
                        ) => {
                            let text = reasoning_text(reasoning);
                            if text.is_empty() {
                                continue;
                            }
                            return Some((
                                ChatEvent::Reasoning(text),
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ReasoningDelta {
                                reasoning, ..
                            },
                        ) => {
                            if reasoning.is_empty() {
                                continue;
                            }
                            return Some((
                                ChatEvent::Reasoning(reasoning),
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall {
                                tool_call,
                                internal_call_id,
                            },
                        ) => {
                            let event = ChatEvent::ToolCallStarted {
                                id: internal_call_id,
                                name: tool_call.function.name,
                                arguments: tool_call.function.arguments,
                            };
                            return Some((
                                event,
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Unknown(value),
                        ) => {
                            if let Some(text) = extract_text_from_value(value.value()) {
                                accumulated.push_str(&text);
                                return Some((
                                    ChatEvent::Delta(text),
                                    (stream, accumulated, done, stream_error),
                                ));
                            }
                            if let Some(event) =
                                event_from_unknown_chunk(value.value().clone())
                            {
                                return Some((
                                    event,
                                    (stream, accumulated, done, stream_error),
                                ));
                            }
                            continue;
                        },
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Final(_),
                        )
                        | MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCallDelta { .. },
                        ) => continue,

                        MultiTurnStreamItem::StreamUserItem(item) => {
                            let event = match item {
                                StreamedUserContent::ToolResult {
                                    tool_result,
                                    internal_call_id,
                                } => ChatEvent::ToolResultReceived {
                                    id: internal_call_id,
                                    name: tool_result.name.clone(),
                                    payload: serde_json::to_value(tool_result)
                                        .unwrap_or_default(),
                                },
                            };
                            return Some((
                                event,
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::ToolExecutionCommitted { .. }
                        | MultiTurnStreamItem::ModelTurnRetried { .. } => continue,
                        MultiTurnStreamItem::CompletionCall(completion_call) => {
                            let mut usage = ConversationUsage::default();
                            usage.add(completion_call.usage);
                            return Some((
                                ChatEvent::Usage(usage),
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                        MultiTurnStreamItem::FinalResponse(resp) => {
                            done = true;
                            let raw = serde_json::to_string(&resp)
                                .unwrap_or_else(|_| format!("{:?}", resp));
                            return Some((
                                ChatEvent::Done {
                                    text: accumulated.clone(),
                                    raw,
                                },
                                (stream, accumulated, done, stream_error),
                            ));
                        },
                    },
                    Some(Err(error)) => {
                        stream_error = Some(error.to_string());
                    },
                    None => {
                        done = true;
                        let error = missing_final_response_error(stream_error);
                        return Some((
                            ChatEvent::Error(error),
                            (stream, accumulated, done, None),
                        ));
                    },
                }
            }
        },
    );

    Ok((events.boxed(), control))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_config_kind_matches_variant() {
        assert_eq!(
            ProviderConfig::Gemini {
                api_key: "k".to_string()
            }
            .kind(),
            ProviderKind::Gemini
        );
        assert_eq!(
            ProviderConfig::Copilot {
                api_key: "k".to_string()
            }
            .kind(),
            ProviderKind::Copilot
        );
        assert_eq!(
            ProviderConfig::Ollama {
                base_url: "http://localhost:11434".to_string()
            }
            .kind(),
            ProviderKind::Ollama
        );
    }

    #[test]
    fn stream_without_final_response_is_reported_as_truncated() {
        assert_eq!(
            missing_final_response_error(None),
            "stream ended before Rig reported a final response"
        );
    }

    #[test]
    fn stream_error_is_preserved_when_no_final_response_arrives() {
        assert_eq!(
            missing_final_response_error(Some("connection lost".to_string())),
            "connection lost"
        );
    }
}
