use std::sync::Arc;

use ai::{
    AiPrefsReader, Conversation, ConversationStore, Folder, FolderStore,
    GenerationManager, Message, MessageStore, MessageStatus, ProviderConfig,
    ProviderKind, Role,
};
use dioxus::prelude::*;
use shared::apps::{LoggingLayer, Orchestrator};

fn conversation_title_from_prompt(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "Новый диалог".to_string();
    }

    trimmed.chars().take(48).collect()
}

fn provider_config(reader: &AiPrefsReader) -> anyhow::Result<ProviderConfig> {
    let provider = reader.provider().unwrap_or(ProviderKind::Gemini);

    match provider {
        ProviderKind::Gemini => {
            let key = reader
                .gemini_api_key()
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("Не указан Gemini API key в настройках"))?;
            Ok(ProviderConfig::Gemini { api_key: key })
        }
        ProviderKind::Copilot => {
            let key = reader
                .copilot_api_key()
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("Не указан Copilot API key в настройках"))?;
            Ok(ProviderConfig::Copilot { api_key: key })
        }
        ProviderKind::Ollama => Ok(ProviderConfig::Ollama {
            base_url: reader.ollama_base_url(),
        }),
    }
}

fn model_for_provider(reader: &AiPrefsReader, provider: ProviderKind) -> String {
    reader.model().unwrap_or_else(|| match provider {
        ProviderKind::Gemini => "gemini-2.5-flash".to_string(),
        ProviderKind::Copilot => "gpt-5.3-codex".to_string(),
        ProviderKind::Ollama => "llama3".to_string(),
    })
}

#[component]
pub fn ChatPage() -> Element {
    let orch = consume_context::<Arc<Orchestrator>>();
    let manager = use_context::<Arc<GenerationManager>>();

    let mut reload_tick = use_signal(|| 0u64);
    let mut conversations = use_signal(Vec::<Conversation>::new);
    let mut folders = use_signal(Vec::<Folder>::new);
    let mut messages = use_signal(Vec::<Message>::new);
    let mut selected_conversation_id = use_signal(|| None::<String>);
    let mut input = use_signal(String::new);
    let mut new_folder_name = use_signal(String::new);
    let mut stream_text = use_signal(String::new);
    let mut generation_active = use_signal(|| false);
    let manager_effect = manager.clone();

    use_effect(move || {
        let _ = reload_tick();

        let mut list = ConversationStore.list().unwrap_or_default();
        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        conversations.set(list.clone());

        let mut fs = FolderStore.list().unwrap_or_default();
        fs.sort_by(|a, b| a.name.cmp(&b.name));
        folders.set(fs);

        if selected_conversation_id().is_none() {
            selected_conversation_id.set(list.first().map(|c| c.id.clone()));
        }
    });

    use_effect(move || {
        if let Some(conversation_id) = selected_conversation_id() {
            let loaded = MessageStore.list(&conversation_id).unwrap_or_default();
            messages.set(loaded);

            let manager = manager_effect.clone();
            let mut generation_active = generation_active;
            let mut stream_text = stream_text;
            let conversation_id_for_state = conversation_id.clone();
            spawn(async move {
                generation_active
                    .set(manager.is_generating(&conversation_id_for_state).await);
                if !generation_active() {
                    stream_text.set(String::new());
                }
            });

            let manager = manager_effect.clone();
            let mut reload_tick = reload_tick;
            spawn(async move {
                if let Some(mut rx) = manager.subscribe(&conversation_id).await {
                    loop {
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let snapshot = rx.borrow().clone();
                        stream_text.set(snapshot.text);
                        generation_active.set(!snapshot.finished);
                        if snapshot.finished {
                            reload_tick.set(reload_tick() + 1);
                            break;
                        }
                    }
                }
            });
        } else {
            messages.set(Vec::new());
            stream_text.set(String::new());
            generation_active.set(false);
        }
    });

    let selected_id = selected_conversation_id();

    rsx! {
        div { class: "h-full w-full flex overflow-hidden bg-zinc-950 text-zinc-200",
            // Left sidebar
            div { class: "w-[320px] border-r border-white/10 flex flex-col",
                div { class: "p-4 border-b border-white/10 flex flex-col gap-3",
                    button {
                        class: "w-full px-3 py-2 rounded-md bg-zinc-100 text-zinc-900 text-sm font-semibold hover:bg-white transition-colors cursor-pointer",
                        onclick: {
                            let orch = orch.clone();
                            move |_| {
                            let reader = AiPrefsReader;
                            let provider = reader.provider().unwrap_or(ProviderKind::Gemini);
                            let model = model_for_provider(&reader, provider);
                            let conversation = Conversation::new("Новый диалог", provider, model);
                            if let Err(error) = ConversationStore.upsert(&conversation) {
                                orch.error(format!("Не удалось создать диалог: {error}"));
                                return;
                            }
                            selected_conversation_id.set(Some(conversation.id));
                            reload_tick.set(reload_tick() + 1);
                            }
                        },
                        "Новый диалог"
                    }

                    div { class: "flex gap-2",
                        input {
                            class: "flex-1 bg-black border border-white/10 rounded-md px-3 py-2 text-sm",
                            placeholder: "Имя папки",
                            value: "{new_folder_name}",
                            oninput: move |e| new_folder_name.set(e.value())
                        }
                        button {
                            class: "px-3 py-2 rounded-md bg-white/10 text-sm hover:bg-white/15 transition-colors cursor-pointer",
                            onclick: {
                                let orch = orch.clone();
                                move |_| {
                                let name = new_folder_name().trim().to_string();
                                if name.is_empty() {
                                    return;
                                }
                                let folder = Folder::new(name);
                                if let Err(error) = FolderStore.upsert(&folder) {
                                    orch.error(format!("Не удалось создать папку: {error}"));
                                    return;
                                }
                                new_folder_name.set(String::new());
                                reload_tick.set(reload_tick() + 1);
                                }
                            },
                            "Папка"
                        }
                    }
                }

                div { class: "flex-1 overflow-y-auto p-3 space-y-4",
                    div { class: "space-y-1",
                        div { class: "text-[10px] uppercase tracking-wider text-zinc-500 px-2 pb-1", "Без папки" }
                        for conversation in conversations().iter().filter(|c| c.folder_id.is_none()) {
                            ConversationRow {
                                key: "{conversation.id}",
                                conversation: conversation.clone(),
                                is_selected: selected_id.as_deref() == Some(conversation.id.as_str()),
                                folders: folders(),
                                on_select: {
                                    let id = conversation.id.clone();
                                    move |_| selected_conversation_id.set(Some(id.clone()))
                                },
                                on_delete: {
                                    let id = conversation.id.clone();
                                    let orch = orch.clone();
                                    move |_| {
                                        if let Err(error) = ConversationStore.remove(&id) {
                                            orch.error(format!("Не удалось удалить диалог: {error}"));
                                        }
                                        if selected_conversation_id().as_deref() == Some(id.as_str()) {
                                            selected_conversation_id.set(None);
                                        }
                                        reload_tick.set(reload_tick() + 1);
                                    }
                                },
                                on_move: {
                                    let conversation = conversation.clone();
                                    let orch = orch.clone();
                                    move |folder_id: Option<String>| {
                                        let mut updated = conversation.clone();
                                        updated.folder_id = folder_id;
                                        if let Err(error) = ConversationStore.upsert(&updated) {
                                            orch.error(format!("Не удалось переместить диалог: {error}"));
                                        }
                                        reload_tick.set(reload_tick() + 1);
                                    }
                                }
                            }
                        }
                    }

                    for folder in folders().iter() {
                        div { class: "space-y-1",
                            div { class: "flex items-center justify-between px-2 pb-1",
                                span { class: "text-[10px] uppercase tracking-wider text-zinc-500", "{folder.name}" }
                                button {
                                    class: "text-[10px] text-red-400 hover:text-red-300 cursor-pointer",
                                    onclick: {
                                        let folder_id = folder.id.clone();
                                        let orch = orch.clone();
                                        move |_| {
                                            if let Err(error) = FolderStore.remove(&folder_id) {
                                                orch.error(format!("Не удалось удалить папку: {error}"));
                                            }
                                            reload_tick.set(reload_tick() + 1);
                                        }
                                    },
                                    "Удалить"
                                }
                            }

                            for conversation in conversations().iter().filter(|c| c.folder_id.as_deref() == Some(folder.id.as_str())) {
                                ConversationRow {
                                    key: "{conversation.id}",
                                    conversation: conversation.clone(),
                                    is_selected: selected_id.as_deref() == Some(conversation.id.as_str()),
                                    folders: folders(),
                                    on_select: {
                                        let id = conversation.id.clone();
                                        move |_| selected_conversation_id.set(Some(id.clone()))
                                    },
                                    on_delete: {
                                        let id = conversation.id.clone();
                                        let orch = orch.clone();
                                        move |_| {
                                            if let Err(error) = ConversationStore.remove(&id) {
                                                orch.error(format!("Не удалось удалить диалог: {error}"));
                                            }
                                            if selected_conversation_id().as_deref() == Some(id.as_str()) {
                                                selected_conversation_id.set(None);
                                            }
                                            reload_tick.set(reload_tick() + 1);
                                        }
                                    },
                                    on_move: {
                                        let conversation = conversation.clone();
                                        let orch = orch.clone();
                                        move |folder_id: Option<String>| {
                                            let mut updated = conversation.clone();
                                            updated.folder_id = folder_id;
                                            if let Err(error) = ConversationStore.upsert(&updated) {
                                                orch.error(format!("Не удалось переместить диалог: {error}"));
                                            }
                                            reload_tick.set(reload_tick() + 1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right chat area
            div { class: "flex-1 flex flex-col min-w-0",
                div { class: "h-12 px-4 border-b border-white/10 flex items-center justify-between",
                    div { class: "text-sm text-zinc-400",
                        if let Some(id) = selected_conversation_id() {
                            "Диалог: {id}"
                        } else {
                            "Выберите диалог"
                        }
                    }
                    if generation_active() {
                        div { class: "text-xs text-emerald-400 animate-pulse", "Стрим активен" }
                    }
                }

                div { class: "flex-1 overflow-y-auto p-5 space-y-3",
                    if messages().is_empty() && stream_text().is_empty() {
                        div { class: "text-sm text-zinc-500", "Пока нет сообщений" }
                    }

                    for message in messages().iter() {
                        MessageBubble { message: message.clone() }
                    }

                    if generation_active() && !stream_text().is_empty() {
                        div { class: "max-w-[85%] rounded-xl px-3 py-2 text-sm bg-zinc-900 border border-emerald-500/30 text-zinc-100",
                            div { class: "text-[10px] uppercase tracking-wider text-emerald-400 mb-1", "assistant • streaming" }
                            "{stream_text}"
                        }
                    }
                }

                div { class: "border-t border-white/10 p-4 flex flex-col gap-3",
                    textarea {
                        class: "w-full min-h-[92px] resize-y bg-black border border-white/10 rounded-md px-3 py-2 text-sm",
                        placeholder: "Напишите сообщение...",
                        value: "{input}",
                        oninput: move |e| input.set(e.value())
                    }

                    div { class: "flex items-center gap-2",
                        button {
                            class: "px-4 py-2 rounded-md bg-zinc-100 text-zinc-900 text-sm font-semibold hover:bg-white transition-colors cursor-pointer disabled:opacity-50",
                            disabled: input().trim().is_empty() || generation_active(),
                            onclick: {
                                let orch = orch.clone();
                                let manager = manager.clone();
                                move |_| {
                                let prompt = input().trim().to_string();
                                if prompt.is_empty() {
                                    return;
                                }

                                let manager = manager.clone();
                                let selected = selected_conversation_id();
                                let mut selected_conversation_id = selected_conversation_id;
                                let mut generation_active = generation_active;
                                let mut input = input;
                                let mut reload_tick = reload_tick;
                                let mut stream_text = stream_text;
                                let orch_for_task = orch.clone();

                                spawn(async move {
                                    let reader = AiPrefsReader;
                                    let provider_cfg = match provider_config(&reader) {
                                        Ok(cfg) => cfg,
                                        Err(error) => {
                                            orch_for_task.error(error.to_string());
                                            return;
                                        }
                                    };

                                    let model = model_for_provider(&reader, provider_cfg.kind());

                                    let conversation_id = if let Some(id) = selected {
                                        id
                                    } else {
                                        let conversation = Conversation::new(
                                            conversation_title_from_prompt(&prompt),
                                            provider_cfg.kind(),
                                            model.clone(),
                                        );
                                        let id = conversation.id.clone();
                                        if let Err(error) = ConversationStore.upsert(&conversation) {
                                            orch_for_task.error(format!("Не удалось создать диалог: {error}"));
                                            return;
                                        }
                                        selected_conversation_id.set(Some(id.clone()));
                                        id
                                    };

                                    if let Err(error) = manager
                                        .start(
                                            conversation_id,
                                            provider_cfg,
                                            model,
                                            orch_for_task.ai_tools(),
                                            prompt,
                                        )
                                        .await
                                    {
                                        orch_for_task.error(format!("Не удалось запустить генерацию: {error}"));
                                        return;
                                    }

                                    input.set(String::new());
                                    stream_text.set(String::new());
                                    generation_active.set(true);
                                    reload_tick.set(reload_tick() + 1);
                                });
                                }
                            },
                            "Отправить"
                        }

                        button {
                            class: "px-3 py-2 rounded-md bg-white/10 text-sm hover:bg-white/15 transition-colors cursor-pointer disabled:opacity-50",
                            disabled: !generation_active(),
                            onclick: {
                                let manager = manager.clone();
                                move |_| {
                                if let Some(id) = selected_conversation_id() {
                                    let manager = manager.clone();
                                    spawn(async move {
                                        let _ = manager.pause(&id).await;
                                    });
                                }
                                }
                            },
                            "Пауза"
                        }

                        button {
                            class: "px-3 py-2 rounded-md bg-white/10 text-sm hover:bg-white/15 transition-colors cursor-pointer disabled:opacity-50",
                            disabled: !generation_active(),
                            onclick: {
                                let manager = manager.clone();
                                move |_| {
                                if let Some(id) = selected_conversation_id() {
                                    let manager = manager.clone();
                                    spawn(async move {
                                        let _ = manager.resume(&id).await;
                                    });
                                }
                                }
                            },
                            "Продолжить"
                        }

                        button {
                            class: "px-3 py-2 rounded-md bg-red-500/15 text-red-300 text-sm hover:bg-red-500/25 transition-colors cursor-pointer disabled:opacity-50",
                            disabled: !generation_active(),
                            onclick: {
                                let manager = manager.clone();
                                move |_| {
                                let mut generation_active = generation_active;
                                if let Some(id) = selected_conversation_id() {
                                    let manager = manager.clone();
                                    spawn(async move {
                                        let _ = manager.stop(&id).await;
                                        generation_active.set(false);
                                    });
                                }
                                }
                            },
                            "Стоп"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ConversationRowProps {
    conversation: Conversation,
    folders: Vec<Folder>,
    is_selected: bool,
    on_select: EventHandler<()>,
    on_delete: EventHandler<()>,
    on_move: EventHandler<Option<String>>,
}

#[component]
fn ConversationRow(props: ConversationRowProps) -> Element {
    let selected_class = if props.is_selected {
        "border-cyan-500/60 bg-cyan-500/10"
    } else {
        "border-white/10 bg-black/40"
    };

    let mut target_folder = use_signal(|| props.conversation.folder_id.clone().unwrap_or_default());

    rsx! {
        div { class: "border rounded-md p-2 space-y-2 {selected_class}",
            button {
                class: "w-full text-left",
                onclick: move |_| props.on_select.call(()),
                div { class: "text-sm text-zinc-100 truncate", "{props.conversation.title}" }
                div { class: "text-[11px] text-zinc-500 truncate", "{props.conversation.model}" }
            }

            div { class: "flex items-center gap-2",
                select {
                    class: "flex-1 bg-black border border-white/10 rounded px-2 py-1 text-xs",
                    value: "{target_folder}",
                    onchange: move |e| {
                        let value = e.value();
                        target_folder.set(value.clone());
                        if value.trim().is_empty() {
                            props.on_move.call(None);
                        } else {
                            props.on_move.call(Some(value));
                        }
                    },
                    option { value: "", "Без папки" }
                    for folder in props.folders.iter() {
                        option { value: "{folder.id}", "{folder.name}" }
                    }
                }

                button {
                    class: "px-2 py-1 text-xs text-red-300 hover:text-red-200 border border-red-500/30 rounded cursor-pointer",
                    onclick: move |_| props.on_delete.call(()),
                    "Удалить"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MessageBubbleProps {
    message: Message,
}

#[component]
fn MessageBubble(props: MessageBubbleProps) -> Element {
    let (role_label, bubble_class) = match props.message.role {
        Role::User => (
            "user",
            "ml-auto bg-cyan-500/10 border border-cyan-500/40 text-zinc-100",
        ),
        Role::Assistant => (
            "assistant",
            "mr-auto bg-zinc-900 border border-white/10 text-zinc-100",
        ),
        Role::System => (
            "system",
            "mr-auto bg-amber-500/10 border border-amber-500/30 text-zinc-100",
        ),
        Role::Tool => (
            "tool",
            "mr-auto bg-violet-500/10 border border-violet-500/30 text-zinc-100",
        ),
    };

    let status_label = match &props.message.status {
        MessageStatus::Complete => "complete".to_string(),
        MessageStatus::Partial => "partial".to_string(),
        MessageStatus::Cancelled => "cancelled".to_string(),
        MessageStatus::Error(error) => format!("error: {error}"),
    };

    rsx! {
        div { class: "max-w-[85%] rounded-xl px-3 py-2 text-sm {bubble_class}",
            div { class: "text-[10px] uppercase tracking-wider text-zinc-500 mb-1", "{role_label} • {status_label}" }
            div { class: "whitespace-pre-wrap break-words", "{props.message.content}" }
        }
    }
}
