use std::sync::Arc;

use futures::stream::{self, BoxStream, StreamExt};
use rig_core::{
    client::{CompletionClient, Nothing},
    completion::{
        CompletionModel, GetTokenUsage, Message as RigMessage,
        message::{Reasoning, ReasoningContent},
    },
    providers::{copilot, gemini, ollama},
    streaming::{
        StreamedAssistantContent, StreamingCompletion, StreamingCompletionResponse,
    },
    tool::ToolDyn,
};
use tokio::sync::Mutex;

use crate::{
    control::{ResponseControl, StreamControl},
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
    tools: Vec<Box<dyn ToolDyn>>,
    prompt: String,
    history: Vec<RigMessage>,
) -> anyhow::Result<(
    BoxStream<'static, ChatEvent>,
    Arc<dyn StreamControl>,
)> {
    match config {
        ProviderConfig::Gemini { api_key } => {
            let client = gemini::Client::new(api_key).map_err(|error| {
                anyhow::anyhow!("Failed to create Gemini client: {error}")
            })?;
            let agent = client.agent(model).tools(tools).build();
            run(agent, prompt, history).await
        },
        ProviderConfig::Copilot { api_key } => {
            let client = copilot::Client::builder()
                .api_key(copilot::CopilotAuth::ApiKey(api_key.clone()))
                .build()
                .map_err(|error| {
                    anyhow::anyhow!("Failed to create Copilot client: {error}")
                })?;
            let agent = client.agent(model).tools(tools).build();
            run(agent, prompt, history).await
        },
        ProviderConfig::Ollama { base_url } => {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(base_url)
                .build()
                .map_err(|error| {
                    anyhow::anyhow!("Failed to create Ollama client: {error}")
                })?;
            let agent = client.agent(model).tools(tools).build();
            run(agent, prompt, history).await
        },
    }
}

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
            ReasoningContent::Encrypted(_) | ReasoningContent::Redacted { .. } | _ => {
                None
            },
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn run<M>(
    agent: rig_core::agent::Agent<M>,
    prompt: String,
    history: Vec<RigMessage>,
) -> anyhow::Result<(
    BoxStream<'static, ChatEvent>,
    Arc<dyn StreamControl>,
)>
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
        .map_err(|error| {
            anyhow::anyhow!("Failed to start streaming completion: {error}")
        })?;

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
                },
                Some(Ok(StreamedAssistantContent::Reasoning(reasoning))) => {
                    let text = reasoning_text(reasoning);
                    if text.is_empty() {
                        continue;
                    }
                    return Some((ChatEvent::Reasoning(text), state));
                },
                Some(Ok(StreamedAssistantContent::ReasoningDelta {
                    reasoning, ..
                })) => {
                    if reasoning.is_empty() {
                        continue;
                    }
                    return Some((ChatEvent::Reasoning(reasoning), state));
                },
                Some(Ok(StreamedAssistantContent::ToolCall { tool_call, .. })) => {
                    let event = ChatEvent::ToolCallStarted {
                        name: tool_call.function.name,
                        arguments: tool_call.function.arguments,
                    };
                    return Some((event, state));
                },
                // Reasoning / tool-call-delta / final-response / unknown chunks
                // are folded into `choice`/`response` internally by
                // `StreamingCompletionResponse` itself; nothing to surface here.
                Some(Ok(_other)) => continue,
                Some(Err(error)) => {
                    state.done = true;
                    return Some((ChatEvent::Error(error.to_string()), state));
                },
                None => {
                    state.done = true;
                    let guard = state.response.lock().await;
                    let message = RigMessage::Assistant {
                        id: guard.message_id.clone(),
                        content: guard.choice.clone(),
                    };
                    drop(guard);
                    let raw = serde_json::to_string(&message).unwrap_or_default();
                    let text = state.accumulated.clone();
                    return Some((ChatEvent::Done { text, raw }, state));
                },
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
