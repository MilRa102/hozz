use std::{sync::Arc};

use futures::{
    stream::{self, BoxStream, StreamExt},
};
use rig::{
    agent::MultiTurnStreamItem, client::{CompletionClient, Nothing}, completion::{
        CompletionModel, GetTokenUsage, Message as RigMessage, message::{Reasoning, ReasoningContent},
    }, providers::{copilot, gemini, ollama}, streaming::{StreamedAssistantContent, StreamingCompletion, StreamingCompletionResponse}, tool::server::ToolServerHandle,
};
use tokio::sync::Mutex;

use crate::{
    control::{NoopStreamControl, ResponseControl, StreamControl},
    model::ProviderKind,
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
        name: String,
        arguments: serde_json::Value,
    },
    /// Tool/hosted-tool result or provider-native tool payload surfaced
    /// through unknown stream chunks.
    ToolResultReceived {
        name: String,
        payload: serde_json::Value,
    },
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
) -> anyhow::Result<(BoxStream<'static, ChatEvent>, Arc<dyn StreamControl>)> {
    match config {
        ProviderConfig::Gemini { api_key } => {
            let client = gemini::Client::new(api_key)
                .map_err(|error| anyhow::anyhow!("Failed to create Gemini client: {error}"))?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client.agent(model).tool_server_handle(tool_server).build();
                run_tool_enabled(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history).await
            }
        }
        ProviderConfig::Copilot { api_key } => {
            let client = copilot::Client::builder()
                .api_key(copilot::CopilotAuth::ApiKey(api_key.clone()))
                .build()
                .map_err(|error| anyhow::anyhow!("Failed to create Copilot client: {error}"))?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client.agent(model).tool_server_handle(tool_server).build();
                run_tool_enabled(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history).await
            }
        }
        ProviderConfig::Ollama { base_url } => {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(base_url)
                .build()
                .map_err(|error| anyhow::anyhow!("Failed to create Ollama client: {error}"))?;
            if let Some(tool_server) = tool_server.clone() {
                let agent = client.agent(model).tool_server_handle(tool_server).build();
                run_tool_enabled(agent, prompt, history, max_tool_turns).await
            } else {
                let agent = client.agent(model).build();
                run(agent, prompt, history).await
            }
        }
    }
}

// Keep PollState strictly for the tool-free standard path
struct PollState<R>
where
    R: Clone + Unpin + GetTokenUsage,
{
    response: Arc<Mutex<StreamingCompletionResponse<R>>>,
    accumulated: String,
    done: bool,
}

fn reasoning_text(reasoning: Reasoning) -> String {
    reasoning
        .content
        .into_iter()
        .filter_map(|part| match part {
            ReasoningContent::Text { text, .. } => Some(text),
            ReasoningContent::Summary(text) => Some(text),
            ReasoningContent::Encrypted(_) | ReasoningContent::Redacted { .. } | _ => None,
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
        }
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
                    && let Some(text) = extract_text_from_value(child) {
                        return Some(text);
                    }
            }

            for child in map.values() {
                if let Some(text) = extract_text_from_value(child) {
                    return Some(text);
                }
            }

            None
        }
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
                    && let Some(name) = tool_name_from_value(child) {
                        return Some(name);
                    }
            }

            for child in map.values() {
                if let Some(name) = tool_name_from_value(child) {
                    return Some(name);
                }
            }

            None
        }
        serde_json::Value::Array(values) => {
            for child in values {
                if let Some(name) = tool_name_from_value(child) {
                    return Some(name);
                }
            }
            None
        }
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

    Some(ChatEvent::ToolResultReceived {
        name,
        payload: value,
    })
}

async fn run<M>(
    agent: rig::agent::Agent<M>,
    prompt: String,
    history: Vec<RigMessage>,
) -> anyhow::Result<(BoxStream<'static, ChatEvent>, Arc<dyn StreamControl>)>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage + Clone + Unpin + Send + 'static,
{
    let response = agent
        .stream_completion(prompt, history)
        .await
        .map_err(|error| {
            anyhow::anyhow!("Failed to build streaming completion request: {error}")
        })?
        .stream()
        .await
        .map_err(|error| anyhow::anyhow!("Failed to start streaming completion: {error}"))?;

    let response = Arc::new(Mutex::new(response));
    let control: Arc<dyn StreamControl> = Arc::new(ResponseControl(response.clone()));

    let state = PollState {
        response,
        accumulated: String::new(),
        done: false,
    };

    let events = stream::unfold(state, |mut state| async move {
        if state.done {
            return None;
        }

        loop {
            let next = {
                let mut guard = state.response.lock().await;
                guard.next().await
            };

            match next {
                Some(Ok(StreamedAssistantContent::Text(text))) => {
                    state.accumulated.push_str(&text.text);
                    return Some((ChatEvent::Delta(text.text), state));
                }
                Some(Ok(StreamedAssistantContent::Reasoning(reasoning))) => {
                    let text = reasoning_text(reasoning);
                    if text.is_empty() {
                        continue;
                    }
                    return Some((ChatEvent::Reasoning(text), state));
                }
                Some(Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. })) => {
                    if reasoning.is_empty() {
                        continue;
                    }
                    return Some((ChatEvent::Reasoning(reasoning), state));
                }
                Some(Ok(StreamedAssistantContent::ToolCall { tool_call, .. })) => {
                    let event = ChatEvent::ToolCallStarted {
                        name: tool_call.function.name,
                        arguments: tool_call.function.arguments,
                    };
                    return Some((event, state));
                }
                Some(Ok(StreamedAssistantContent::Unknown(value))) => {
                    if let Some(text) = extract_text_from_value(&value) {
                        state.accumulated.push_str(&text);
                        return Some((ChatEvent::Delta(text), state));
                    }
                    if let Some(event) = event_from_unknown_chunk(value) {
                        return Some((event, state));
                    }
                    continue;
                }
                Some(Ok(_other)) => continue,
                Some(Err(error)) => {
                    state.done = true;
                    return Some((ChatEvent::Error(error.to_string()), state));
                }
                None => {
                    state.done = true;
                    let text = state.accumulated.clone();
                    return Some((ChatEvent::Done { text, raw: String::new() }, state));
                }
            }
        }
    });

    Ok((events.boxed(), control))
}

// THE FIX: Clean, closure-based state mapping. No hallucinated structs.
async fn run_tool_enabled<M>(
    agent: rig::agent::Agent<M>,
    prompt: String,
    history: Vec<RigMessage>,
    max_tool_turns: usize,
) -> anyhow::Result<(BoxStream<'static, ChatEvent>, Arc<dyn StreamControl>)>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage + Clone + Unpin + Send + 'static,
{
    let response = agent
        .runner(prompt)
        .history(history)
        .max_turns(max_tool_turns.max(1))
        .stream()
        .await;

    // Оборачиваем стрим в Box::pin, чтобы безопасно опрашивать его в unfold
    let stream = Box::pin(response);
    let control: Arc<dyn StreamControl> = Arc::new(NoopStreamControl);

    // Стейт: (Стрим Rig, Накопленный текст, Флаг завершения)
    let state = (stream, String::new(), false);

    let events = stream::unfold(state, |(mut stream, mut accumulated, mut done)| async move {
        if done {
            return None;
        }

        loop {
            match stream.next().await {
                Some(Ok(event)) => {
                    // Используем правильный MultiTurnStreamItem из rig-core
                    match event {
                        MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text)) => {
                            accumulated.push_str(&text.text);
                            return Some((ChatEvent::Delta(text.text), (stream, accumulated, done)));
                        }
                        MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Reasoning(reasoning)) => {
                            let text = reasoning_text(reasoning);
                            if text.is_empty() {
                                continue;
                            }
                            return Some((ChatEvent::Reasoning(text), (stream, accumulated, done)));
                        }
                        MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => {
                            if reasoning.is_empty() {
                                continue;
                            }
                            return Some((ChatEvent::Reasoning(reasoning), (stream, accumulated, done)));
                        }
                        MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                            let event = ChatEvent::ToolCallStarted {
                                name: tool_call.function.name,
                                arguments: tool_call.function.arguments,
                            };
                            return Some((event, (stream, accumulated, done)));
                        }
                        MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Unknown(value)) => {
                            if let Some(text) = extract_text_from_value(&value) {
                                accumulated.push_str(&text);
                                return Some((ChatEvent::Delta(text), (stream, accumulated, done)));
                            }
                            if let Some(event) = event_from_unknown_chunk(value) {
                                return Some((event, (stream, accumulated, done)));
                            }
                            continue;
                        }
                        MultiTurnStreamItem::StreamAssistantItem(_) => continue,
                        
                        MultiTurnStreamItem::StreamUserItem(item) => {
                            // Здесь лежат результаты вызова инструментов
                            let payload = serde_json::to_value(&item).unwrap_or_default();
                            return Some((
                                ChatEvent::ToolResultReceived {
                                    name: "tool".to_string(), 
                                    payload,
                                },
                                (stream, accumulated, done),
                            ));
                        }
                        MultiTurnStreamItem::FinalResponse(resp) => {
                            done = true;
                            let raw = serde_json::to_string(&resp).unwrap_or_else(|_| format!("{:?}", resp));
                            return Some((
                                ChatEvent::Done {
                                    text: accumulated.clone(),
                                    raw,
                                },
                                (stream, accumulated, done),
                            ));
                        }
                        _ => tracing::warn!("Unhandled MultiTurnStreamItem: {:?}", serde_json::to_string(&event).unwrap_or_default()),
                    }
                }
                Some(Err(error)) => {
                    done = true;
                    // Ошибка выведется сама благодаря type inference
                    return Some((
                        ChatEvent::Error(error.to_string()),
                        (stream, accumulated, done),
                    ));
                }
                None => {
                    done = true;
                    return Some((
                        ChatEvent::Done {
                            text: accumulated.clone(),
                            raw: String::new(),
                        },
                        (stream, accumulated, done),
                    ));
                }
            }
        }
    });

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
}