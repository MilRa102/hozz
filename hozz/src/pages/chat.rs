use std::{collections::BTreeSet, sync::Arc};

use ai::{
    AiPrefsReader, Conversation, ConversationStore, Folder, FolderStore,
    GenerationManager, Message, MessageStore, ProviderConfig, ProviderKind, Role,
};
use dioxus::{document::eval, logger::tracing, prelude::*};
use dioxus_free_icons::icons::{
    hi_outline_icons::{HiChat, HiFolder, HiFolderAdd, HiFolderRemove}, io_icons::IoArrowBack, ld_icons::LdList, md_av_icons::{MdPause, MdPlayArrow, MdStop}, md_communication_icons::MdChatBubble, md_content_icons::MdSend, md_file_icons::MdCreateNewFolder,
};
use shared::apps::{LoggingLayer, Orchestrator};

use crate::{components::message::MarkdownMessage, utils::Icon};

fn truncate_text(input: String, max: usize) -> String {
    if input.chars().count() <= max {
        return input;
    }

    let short = input.chars().take(max).collect::<String>();
    format!("{short}...")
}

fn value_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::String(v) => v.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_default()
        },
    }
}

fn flatten_tool_details(
    prefix: &str,
    value: &serde_json::Value,
    output: &mut Vec<(String, String)>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let next = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_tool_details(&next, child, output);
            }
        },
        serde_json::Value::Array(values) => {
            for (idx, child) in values.iter().enumerate() {
                let next = if prefix.is_empty() {
                    format!("[{idx}]")
                } else {
                    format!("{prefix}[{idx}]")
                };
                flatten_tool_details(&next, child, output);
            }
        },
        _ => {
            if prefix.is_empty() {
                return;
            }
            let text = truncate_text(value_to_text(value), 120);
            output.push((prefix.to_string(), text));
        },
    }
}

fn find_tool_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in ["name", "tool", "tool_name"] {
                if let Some(serde_json::Value::String(name)) = map.get(key) {
                    let name = name.trim();
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }

            if let Some(function) = map.get("function")
                && let Some(name) = find_tool_name(function)
            {
                return Some(name);
            }
            if let Some(call) = map.get("tool_call")
                && let Some(name) = find_tool_name(call)
            {
                return Some(name);
            }

            for child in map.values() {
                if let Some(name) = find_tool_name(child) {
                    return Some(name);
                }
            }

            None
        },
        serde_json::Value::Array(values) => {
            for child in values {
                if let Some(name) = find_tool_name(child) {
                    return Some(name);
                }
            }
            None
        },
        _ => None,
    }
}

fn parse_content_pairs(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let (key, value) = trimmed.split_once(':')?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return None;
            }

            Some((key.to_string(), truncate_text(value.to_string(), 120)))
        })
        .collect()
}

fn tool_display_data(message: &Message) -> (String, String, Vec<(String, String)>) {
    let mut name: Option<String> = None;
    let mut details = Vec::<(String, String)>::new();
    let mut event_kind = "call".to_string();

    if let Ok(raw_json) = serde_json::from_str::<serde_json::Value>(&message.raw) {
        if name.is_none() {
            name = find_tool_name(&raw_json);
        }
        if let Some(event) = raw_json.get("event")
            && event == "tool_result_received"
        {
            event_kind = "result".to_string();
        }
        flatten_tool_details("raw", &raw_json, &mut details);
    }

    if let Ok(content_json) = serde_json::from_str::<serde_json::Value>(&message.content)
    {
        if name.is_none() {
            name = find_tool_name(&content_json);
        }
        flatten_tool_details("content", &content_json, &mut details);
    } else {
        details.extend(parse_content_pairs(&message.content));
    }

    if name.is_none() {
        name = message
            .content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| {
                line.split([':', '(', '{'])
                    .next()
                    .unwrap_or(line)
                    .trim()
                    .to_string()
            })
            .filter(|line| !line.is_empty());
    }

    let mut dedup = BTreeSet::new();
    details.retain(|(key, value)| dedup.insert((key.clone(), value.clone())));
    details.truncate(24);

    let tool_name = name.unwrap_or_else(|| "Инструмент".to_string());
    (tool_name, event_kind, details)
}

fn thinking_preview(input: &str) -> Option<String> {
    let first_non_empty = input
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;

    let heading = first_non_empty.trim_start_matches('#').trim();
    let bold = first_non_empty
        .split("**")
        .nth(1)
        .or_else(|| first_non_empty.split("__").nth(1))
        .unwrap_or(first_non_empty);

    let normalized = if !heading.is_empty() && heading != first_non_empty {
        heading
    } else {
        bold
    }
    .trim_matches('*')
    .trim_matches('_')
    .trim_matches('`')
    .trim_matches('-')
    .trim_matches('>')
    .trim();

    if normalized.is_empty() {
        return None;
    }

    let end = normalized
        .find(['.', '!', '?', ':', '\n'])
        .unwrap_or(normalized.len());
    let short = normalized[..end].trim();
    let short = short
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");

    if short.is_empty() { None } else { Some(short) }
}

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
                .ok_or_else(|| {
                    anyhow::anyhow!("Не указан Gemini API key в настройках")
                })?;
            Ok(ProviderConfig::Gemini { api_key: key })
        },
        ProviderKind::Copilot => {
            let key = reader
                .copilot_api_key()
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("Не указан Copilot API key в настройках")
                })?;
            Ok(ProviderConfig::Copilot { api_key: key })
        },
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

fn row_class(selected: bool) -> &'static str {
    if selected {
        "border-cyan-500/60 bg-cyan-500/10"
    } else {
        "border-white/10 bg-black/40"
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ContextMenuTarget {
    Folder(String),
    Conversation(String),
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
    let mut active_folder_id = use_signal(|| None::<String>);

    let mut input = use_signal(String::new);
    let mut stream_text = use_signal(String::new);
    let mut generation_active = use_signal(|| false);
    let mut generation_paused = use_signal(|| false);
    let mut is_thinking = use_signal(|| false);
    let mut thinking_text = use_signal(String::new);

    let mut create_menu_open = use_signal(|| false);
    let mut creating_folder = use_signal(|| false);
    let mut new_folder_name = use_signal(String::new);
    let mut context_menu = use_signal(|| None::<ContextMenuTarget>);

    let manager_for_effect = manager.clone();

    use_effect(move || {
        let _ = messages();
        let _ = stream_text();

        let js = r#"window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });"#;
        let _ = eval(js);
    });

    use_effect(move || {
        let _ = input();

        let js = r#"(() => {
            const el = document.getElementById('chat-input-textarea');
            if (!el) return;
            const minHeight = 52;
            const maxHeight = 156;
            el.style.height = 'auto';
            const nextHeight = Math.min(Math.max(el.scrollHeight, minHeight), maxHeight);
            el.style.height = `${nextHeight}px`;
            el.style.overflowY = el.scrollHeight > maxHeight ? 'auto' : 'hidden';
        })();"#;
        let _ = eval(js);
    });

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
        let _ = reload_tick();

        if let Some(conversation_id) = selected_conversation_id() {
            let loaded = MessageStore
                .list(&conversation_id)
                .unwrap_or_default();
            messages.set(loaded);
        } else {
            messages.set(Vec::new());
        }
    });

    use_effect(move || {
        let _ = reload_tick();

        if let Some(conversation_id) = selected_conversation_id() {
            let manager = manager_for_effect.clone();
            let mut generation_active = generation_active;
            let mut generation_paused = generation_paused;
            let mut stream_text = stream_text;
            let mut thinking_text = thinking_text;
            let mut is_thinking = is_thinking;
            let mut reload_tick = reload_tick;
            spawn(async move {
                generation_active.set(manager.is_generating(&conversation_id).await);
                if !generation_active() {
                    generation_paused.set(false);
                    is_thinking.set(false);
                    thinking_text.set(String::new());
                    stream_text.set(String::new());
                }

                if let Some(mut rx) = manager.subscribe(&conversation_id).await {
                    loop {
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let snapshot = rx.borrow().clone();
                        thinking_text.set(snapshot.thinking.clone());
                        is_thinking.set(
                            !snapshot.finished && !snapshot.thinking.trim().is_empty(),
                        );
                        stream_text.set(snapshot.text);
                        generation_active.set(!snapshot.finished);
                        if snapshot.finished {
                            generation_paused.set(false);
                            is_thinking.set(false);
                            thinking_text.set(String::new());
                            reload_tick.set(reload_tick() + 1);
                            break;
                        }
                    }
                }
            });
        } else {
            thinking_text.set(String::new());
            is_thinking.set(false);
            stream_text.set(String::new());
            generation_active.set(false);
            generation_paused.set(false);
        }
    });

    let folders_list = folders();
    let conversations_all = conversations();
    let active_folder = active_folder_id();

    let chats_for_view: Vec<Conversation> = match active_folder.as_deref() {
        Some(folder_id) => conversations_all
            .iter()
            .filter(|c| c.folder_id.as_deref() == Some(folder_id))
            .cloned()
            .collect(),
        None => conversations_all.clone(),
    };

    let selected_conversation = selected_conversation_id().and_then(|id| {
        conversations_all
            .iter()
            .find(|c| c.id == id)
            .cloned()
    });

    let title = selected_conversation
        .as_ref()
        .map(|c| c.title.clone())
        .unwrap_or_else(|| "Выберите диалог".to_string());

    let input_status = if generation_active() {
        if generation_paused() {
            "Пауза"
        } else if is_thinking() {
            "Размышляет"
        } else {
            "Формирует ответ"
        }
    } else if input().trim().is_empty() {
        "Ожидает отправки"
    } else {
        "Готово к отправке"
    };
    let input_status_text = if generation_active() {
        if generation_paused() {
            "Генерация приостановлена. Нажмите кнопку воспроизведения для продолжения."
                .to_string()
        } else if is_thinking() {
            thinking_preview(&thinking_text())
                .unwrap_or_else(|| "Модель анализирует вопрос и собирает план ответа".to_string())
        } else {
            thinking_preview(&thinking_text())
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "Модель оформляет финальный ответ".to_string())
        }
    } else if input().trim().is_empty() {
        "Введите сообщение в поле ниже".to_string()
    } else {
        "Нажмите Enter или кнопку отправки".to_string()
    };
    let input_status_class = if generation_active() {
        if generation_paused() {
            "text-amber-300 bg-amber-400/10 border-amber-400/20"
        } else if is_thinking() {
            "text-cyan-300 bg-cyan-400/10 border-cyan-400/20"
        } else {
            "text-emerald-300 bg-emerald-400/10 border-emerald-400/20"
        }
    } else if input().trim().is_empty() {
        "text-zinc-300 bg-zinc-700/40 border-zinc-500/30"
    } else {
        "text-cyan-300 bg-cyan-400/10 border-cyan-400/20"
    };
    let input_status_running = generation_active() && !generation_paused();

    let input_box_class = if generation_active() {
        "w-full min-h-[52px] max-h-[156px] resize-none overflow-y-hidden bg-zinc-950/80 rounded-xl px-3 py-2 text-sm leading-6 shadow-[0_0_0_1px_rgba(52,211,153,0.22),0_0_16px_rgba(16,185,129,0.2)]"
    } else {
        "w-full min-h-[52px] max-h-[156px] resize-none overflow-y-hidden bg-zinc-950/80 rounded-xl px-3 py-2 text-sm leading-6"
    };

    rsx! {
        div {
            class: "h-full w-full flex overflow-hidden bg-zinc-950 text-zinc-200",
            onclick: move |_| {
                context_menu.set(None);
                create_menu_open.set(false);
            },

            // Sidebar
            div { class: "w-56 border-r border-white/10 flex flex-col",
                // Sidebar header
                div { class: "p-2 flex flex-col gap-2",
                    div { class: "relative flex flex-row justify-between items-center",
                        div { class: "h-8 inline-flex items-center rounded-md border border-white/10 bg-zinc-900/70 overflow-hidden",
                            button {
                                class: "w-8 h-8 flex items-center justify-center hover:bg-white/5 transition-colors cursor-pointer",
                                title: "Новый диалог",
                                onclick: {
                                    let orch = orch.clone();
                                    move |evt| {
                                        evt.stop_propagation();
                                        let reader = AiPrefsReader;
                                        let provider = reader.provider().unwrap_or(ProviderKind::Ollama);
                                        let model = model_for_provider(&reader, provider);
                                        let mut conversation = Conversation::new("Новый диалог", provider, model);
                                        if let Some(folder_id) = active_folder_id() {
                                            conversation.folder_id = Some(folder_id);
                                        }
                                        if let Err(error) = ConversationStore.upsert(&conversation) {
                                            orch.error(format!("Не удалось создать диалог: {error}"));
                                            return;
                                        }
                                        selected_conversation_id.set(Some(conversation.id));
                                        creating_folder.set(false);
                                        create_menu_open.set(false);
                                        reload_tick.set(reload_tick() + 1);
                                    }
                                },
                                Icon { icon: HiChat, size: 14, color: "#e4e4e7" }
                            }
                            button {
                                class: "w-8 h-8 border-l border-white/10 flex items-center justify-center hover:bg-white/5 transition-colors cursor-pointer",
                                title: "Меню создания",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    create_menu_open.set(!create_menu_open());
                                },
                                Icon { icon: LdList, size: 14 }
                            }
                        }
                        if create_menu_open() {
                            div {
                                class: "absolute z-20 top-10 left-0 rounded-md border border-white/10 bg-zinc-950/98 shadow-xl flex flex-col items-start",
                                onclick: move |evt| evt.stop_propagation(),
                                button {
                                    class: "w-full px-3 py-1.5 rounded hover:bg-white/5 transition-colors cursor-pointer flex items-center justify-between gap-2 text-sm",
                                    title: "Новый диалог",
                                    onclick: {
                                        move |_| {
                                            let reader = AiPrefsReader;
                                            let provider = reader.provider().unwrap_or(ProviderKind::Ollama);
                                            let model = model_for_provider(&reader, provider);
                                            let mut conversation = Conversation::new("Новый диалог", provider, model);
                                            if let Some(folder_id) = active_folder_id() {
                                                conversation.folder_id = Some(folder_id);
                                            }
                                            if let Err(e) = ConversationStore.upsert(&conversation) {
                                                tracing::error!(error = %e, "Failed to create conversation");
                                                return;
                                            }
                                            selected_conversation_id.set(Some(conversation.id));
                                            creating_folder.set(false);
                                            create_menu_open.set(false);
                                            reload_tick.set(reload_tick() + 1);
                                        }
                                    },
                                    Icon { icon: HiChat, size: 14 }
                                    "Новый диалог"
                                }
                                button {
                                    class: "w-full px-3 py-1.5 rounded hover:bg-white/5 transition-colors cursor-pointer flex items-center justify-between gap-2 text-sm",
                                    title: "Новая папка",
                                    onclick: move |_| {
                                        creating_folder.set(true);
                                        create_menu_open.set(false);
                                    },
                                    Icon { icon: HiFolderAdd, size: 14 }
                                    "Новая папка"
                                }
                            }
                        }
                        span { class: "pr-2 font-semibold text-lg", "Диалоги" }
                    }

                    if creating_folder() {
                        div { class: "flex justify-stretch gap-1",
                            input {
                                class: "flex h-8 bg-black border border-white/10 rounded px-1 text-xs",
                                placeholder: "Имя папки",
                                value: "{new_folder_name}",
                                oninput: move |e| new_folder_name.set(e.value())
                            }
                            button {
                                class: "px-1 rounded hover:bg-white/15 transition-colors cursor-pointer",
                                onclick: {
                                    move |_| {
                                        let name = new_folder_name().trim().to_string();
                                        if name.is_empty() {
                                            return;
                                        }
                                        let folder = Folder::new(name);
                                        if let Err(e) = FolderStore.upsert(&folder) {
                                            tracing::error!(error = %e, "Failed to create folder");
                                            return;
                                        }
                                        creating_folder.set(false);
                                        new_folder_name.set(String::new());
                                        reload_tick.set(reload_tick() + 1);
                                    }
                                },
                                Icon { icon: MdCreateNewFolder }
                            }
                        }
                    }
                }

                // Sidebar content
                div { class: "flex-1 overflow-y-auto",
                    // Folders list
                    div { class: "space-y-1.5 p-3",
                        for folder in folders_list.iter() {
                            div {
                                class: "relative rounded-md border px-2 py-1.5 flex items-center {row_class(active_folder.as_deref() == Some(folder.id.as_str()))}",
                                oncontextmenu: {
                                    let folder_id = folder.id.clone();
                                    move |evt| {
                                        evt.prevent_default();
                                        evt.stop_propagation();
                                        context_menu.set(Some(ContextMenuTarget::Folder(folder_id.clone())));
                                    }
                                },
                                button { 
                                    class: "flex-1 text-left min-w-0 cursor-pointer",
                                    onclick: {
                                        let folder_id = folder.id.clone();
                                        move |_| {
                                            active_folder_id.set(Some(folder_id.clone()));
                                            selected_conversation_id.set(None);
                                        }
                                    },
                                    div { class: "flex flex-row items-center gap-5 text-sm text-zinc-100 truncate px-2 py-1.5", 
                                        Icon { icon: HiFolder }
                                        "{folder.name}" 
                                    }
                                }

                                if context_menu().as_ref() == Some(&ContextMenuTarget::Folder(folder.id.clone())) {
                                    div {
                                        class: "absolute top-10 right-0 z-20 min-w-[132px] border border-white/10 bg-zinc-950 rounded-md shadow-xl",
                                        button {
                                            class: "flex flex-row items-center gap-2 w-full text-left px-2 py-1.5 rounded text-xs text-pink-300 hover:text-pink-200 hover:bg-pink-500/10 transition-colors cursor-pointer",
                                            onclick: {
                                                let folder_id = folder.id.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    if let Err(e) = FolderStore.remove(&folder_id) {
                                                        tracing::error!(error = %e, "Failed to delete folder");
                                                        return;
                                                    }
                                                    if active_folder_id().as_deref() == Some(folder_id.as_str()) {
                                                        active_folder_id.set(None);
                                                    }
                                                    context_menu.set(None);
                                                    reload_tick.set(reload_tick() + 1);
                                                }
                                            },
                                            Icon { icon: HiFolderRemove, size: 14 }
                                            "Удалить папку"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Conversations list
                    div { class: "space-y-1.5 p-3",
                        for conversation in chats_for_view.iter() {
                            div {
                                class: "relative rounded-md border px-2 py-1.5 {row_class(selected_conversation_id().as_deref() == Some(conversation.id.as_str()))}",
                                oncontextmenu: {
                                    let conversation_id = conversation.id.clone();
                                    move |evt| {
                                        evt.prevent_default();
                                        evt.stop_propagation();
                                        context_menu.set(Some(ContextMenuTarget::Conversation(conversation_id.clone())));
                                    }
                                },
                                div { class: "flex items-start gap-2",
                                    button {
                                        class: "flex-1 text-left min-w-0 cursor-pointer",
                                        onclick: {
                                            let id = conversation.id.clone();
                                            move |_| selected_conversation_id.set(Some(id.clone()))
                                        },
                                        div { class: "flex flex-row items-center gap-2 py-1.5 text-xs text-zinc-100 truncate", 
                                            Icon { icon: MdChatBubble, size: 15 }
                                            "{conversation.title}" 
                                        }
                                    }
                                }

                                if context_menu().as_ref() == Some(&ContextMenuTarget::Conversation(conversation.id.clone())) {
                                    div {
                                        class: "absolute top-8 right-0 z-20 min-w-[138px] border border-white/10 bg-zinc-950 rounded-md shadow-xl",
                                        button {
                                            class: "flex flex-row items-center gap-2 w-full text-left px-2 py-1.5 rounded text-xs text-pink-300 hover:text-pink-200 hover:bg-pink-500/10 transition-colors cursor-pointer",
                                            onclick: {
                                                let id = conversation.id.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    if let Err(e) = ConversationStore.remove(&id) {
                                                        tracing::error!(error = %e, "Failed to delete dialog");
                                                        return;
                                                    }
                                                    if selected_conversation_id().as_deref() == Some(id.as_str()) {
                                                        selected_conversation_id.set(None);
                                                    }
                                                    context_menu.set(None);
                                                    reload_tick.set(reload_tick() + 1);
                                                }
                                            },
                                            Icon { icon: MdChatBubble, size: 14 }
                                            "Удалить диалог"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Sidebar footer
                if active_folder.is_some() {
                    div { class: "p-2 border-t border-white/10",
                        button {
                            class: "w-full flex flex-row items-center gap-4 px-4 h-8 rounded-md text-lg hover:bg-white/5 transition-colors cursor-pointer",
                            onclick: move |_| active_folder_id.set(None),
                            Icon { icon: IoArrowBack }
                            "Назад"
                        }
                    }
                }
            }

            // Chat area
            div { class: "flex-1 flex flex-col min-w-0",
                // Chat header
                div { class: "h-12 px-4 flex items-center",
                    div { class: "flex-1" }
                    div { class: "text-sm text-zinc-300 text-center truncate max-w-[70%]", "{title}" }
                    div { class: "flex-1 flex justify-end",
                        if generation_active() {
                            div { class: "text-[11px] text-emerald-400 animate-pulse", "Генерация" }
                        }
                    }
                }

                // Chat messages area
                div { class: "flex-1 overflow-y-auto p-5 space-y-3",
                    if messages().is_empty() && stream_text().is_empty() {
                        div { class: "text-sm text-zinc-500", "Пока нет сообщений" }
                    }

                    for message in messages().iter() {
                        MessageView { message: message.clone() }
                    }

                    if generation_active() {
                        div { class: "max-w-[85%] mr-auto rounded-xl border border-cyan-500/20 bg-zinc-900/80 px-4 py-3 text-sm text-zinc-100 shadow-[0_0_0_1px_rgba(34,211,238,0.08)]",
                            div { class: "mb-2 text-[10px] uppercase tracking-wider text-cyan-400 font-semibold", "Ассистент" }
                            div { class: "prose prose-invert prose-sm max-w-none",
                                MarkdownMessage { content: if stream_text().is_empty() { "…".to_string() } else { stream_text() } }
                            }
                        }
                    }
                }

                // Chat input area
                div { class: "p-3",
                    div { class: "rounded-2xl border border-white/10 bg-zinc-900/55 p-3 flex flex-col gap-3 shadow-[0_16px_32px_rgba(0,0,0,0.25)]",
                        ThinkingBanner {
                            status: input_status,
                            status_class: input_status_class,
                            text: input_status_text,
                            running: input_status_running,
                        }

                        textarea {
                            id: "chat-input-textarea",
                            class: "{input_box_class}",
                            style: "height: 52px;",
                            placeholder: "Напишите сообщение...",
                            rows: "2",
                            value: "{input}",
                            oninput: move |e| input.set(e.value()),
                            onkeydown: {
                            let orch = orch.clone();
                            let manager = manager.clone();
                            move |evt| {
                                let key = evt.key().to_string();
                                let modifiers = evt.data().modifiers();
                                let shift_pressed = modifiers.contains(dioxus::html::Modifiers::SHIFT);
                                let ctrl_pressed = modifiers.contains(dioxus::html::Modifiers::CONTROL)
                                    || modifiers.contains(dioxus::html::Modifiers::META);

                                if key == "Enter" && !generation_active() && (shift_pressed || ctrl_pressed) {
                                    evt.prevent_default();
                                    let current = input();
                                    input.set(format!("{current}\n"));
                                    return;
                                }

                                if key == "Enter" && !generation_active() && !shift_pressed && !ctrl_pressed {
                                    evt.prevent_default();
                                    let prompt = input().trim().to_string();
                                    if prompt.is_empty() {
                                        return;
                                    }

                                    let manager = manager.clone();
                                    let selected = selected_conversation_id();
                                    let mut selected_conversation_id = selected_conversation_id;
                                    let mut generation_active = generation_active;
                                    let mut generation_paused = generation_paused;
                                    let mut is_thinking = is_thinking;
                                    let mut thinking_text = thinking_text;
                                    let mut input = input;
                                    let mut reload_tick = reload_tick;
                                    let mut stream_text = stream_text;
                                    let orch_for_task = orch.clone();
                                    let active_folder = active_folder_id();

                                    spawn(async move {
                                        let reader = AiPrefsReader;
                                        let provider_cfg = match provider_config(&reader) {
                                            Ok(cfg) => cfg,
                                            Err(e) => {
                                                tracing::error!(error = %e, "Failed to get provider config");
                                                return;
                                            }
                                        };

                                        let model = model_for_provider(&reader, provider_cfg.kind());

                                        let conversation_id = if let Some(id) = selected {
                                            id
                                        } else {
                                            let mut conversation = Conversation::new(
                                                conversation_title_from_prompt(&prompt),
                                                provider_cfg.kind(),
                                                model.clone(),
                                            );
                                            conversation.folder_id = active_folder;
                                            let id = conversation.id.clone();
                                            if let Err(e) = ConversationStore.upsert(&conversation) {
                                                tracing::error!(error = %e, "Failed to create conversation");
                                                orch_for_task.error(format!("Не удалось создать диалог: {e}"));
                                                return;
                                            }
                                            selected_conversation_id.set(Some(id.clone()));
                                            id
                                        };

                                        if let Err(e) = manager
                                            .start(
                                                conversation_id,
                                                provider_cfg,
                                                model,
                                                orch_for_task.ai_tools(),
                                                prompt,
                                            )
                                            .await
                                        {
                                            tracing::error!(error = %e, "Failed to start generation");
                                            orch_for_task.error(format!("Не удалось запустить генерацию: {e}"));
                                            return;
                                        }

                                        input.set(String::new());
                                        stream_text.set(String::new());
                                        thinking_text.set(String::new());
                                        is_thinking.set(false);
                                        generation_paused.set(false);
                                        generation_active.set(true);
                                        reload_tick.set(reload_tick() + 1);
                                    });
                                    return;
                                }

                                if (key == " " || key == "Space") && generation_active() {
                                    evt.prevent_default();
                                    if let Some(id) = selected_conversation_id() {
                                        let manager = manager.clone();
                                        let mut generation_paused = generation_paused;
                                        spawn(async move {
                                            if generation_paused() {
                                                let _ = manager.resume(&id).await;
                                                generation_paused.set(false);
                                            } else {
                                                let _ = manager.pause(&id).await;
                                                generation_paused.set(true);
                                            }
                                        });
                                    }
                                    return;
                                }

                                if key == "Escape" && generation_active() {
                                    evt.prevent_default();
                                    if let Some(id) = selected_conversation_id() {
                                        let manager = manager.clone();
                                        let mut generation_active = generation_active;
                                        let mut generation_paused = generation_paused;
                                        let mut is_thinking = is_thinking;
                                        let mut thinking_text = thinking_text;
                                        let mut reload_tick = reload_tick;
                                        spawn(async move {
                                            let _ = manager.stop(&id).await;
                                            thinking_text.set(String::new());
                                            is_thinking.set(false);
                                            generation_paused.set(false);
                                            generation_active.set(false);
                                            reload_tick.set(reload_tick() + 1);
                                        });
                                    }
                                }
                            }
                            }
                        }

                        div { class: "flex items-center justify-end gap-1.5",
                            button {
                                class: "h-8 w-8 rounded-md bg-white/10 hover:bg-white/15 transition-colors cursor-pointer disabled:opacity-50 flex items-center justify-center",
                                disabled: !generation_active(),
                                title: if generation_paused() { "Возобновить" } else { "Пауза" },
                                onclick: {
                                    let manager = manager.clone();
                                    move |_| {
                                        if let Some(id) = selected_conversation_id() {
                                            let manager = manager.clone();
                                            let mut generation_paused = generation_paused;
                                            spawn(async move {
                                                if generation_paused() {
                                                    let _ = manager.resume(&id).await;
                                                    generation_paused.set(false);
                                                } else {
                                                    let _ = manager.pause(&id).await;
                                                    generation_paused.set(true);
                                                }
                                            });
                                        }
                                    }
                                },
                                if generation_paused() {
                                    Icon { icon: MdPlayArrow, size: 16 }
                                } else {
                                    Icon { icon: MdPause, size: 16 }
                                }
                            }

                            if generation_active() {
                                button {
                                    class: "h-8 w-8 rounded-md bg-red-500/15 text-red-300 hover:bg-red-500/25 transition-colors cursor-pointer flex items-center justify-center",
                                    title: "Остановить",
                                    onclick: {
                                        let manager = manager.clone();
                                        move |_| {
                                            if let Some(id) = selected_conversation_id() {
                                                let manager = manager.clone();
                                                let mut generation_active = generation_active;
                                                let mut generation_paused = generation_paused;
                                                let mut reload_tick = reload_tick;
                                                spawn(async move {
                                                    let _ = manager.stop(&id).await;
                                                    generation_paused.set(false);
                                                    generation_active.set(false);
                                                    reload_tick.set(reload_tick() + 1);
                                                });
                                            }
                                        }
                                    },
                                    Icon { icon: MdStop, size: 16, color: "#fca5a5" }
                                }
                            } else {
                                button {
                                    class: "h-8 w-8 rounded-md bg-zinc-100 text-zinc-900 hover:bg-white transition-colors cursor-pointer disabled:opacity-50 flex items-center justify-center",
                                    disabled: input().trim().is_empty(),
                                    title: "Отправить",
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
                                            let mut generation_paused = generation_paused;
                                            let mut is_thinking = is_thinking;
                                            let mut thinking_text = thinking_text;
                                            let mut input = input;
                                            let mut reload_tick = reload_tick;
                                            let mut stream_text = stream_text;
                                            let orch_for_task = orch.clone();
                                            let active_folder = active_folder_id();

                                            spawn(async move {
                                                let reader = AiPrefsReader;
                                                let provider_cfg = match provider_config(&reader) {
                                                    Ok(cfg) => cfg,
                                                    Err(e) => {
                                                        tracing::error!(error = %e, "Failed to get provider config");
                                                        return;
                                                    }
                                                };

                                                let model = model_for_provider(&reader, provider_cfg.kind());

                                                let conversation_id = if let Some(id) = selected {
                                                    id
                                                } else {
                                                    let mut conversation = Conversation::new(
                                                        conversation_title_from_prompt(&prompt),
                                                        provider_cfg.kind(),
                                                        model.clone(),
                                                    );
                                                    conversation.folder_id = active_folder;
                                                    let id = conversation.id.clone();
                                                    if let Err(e) = ConversationStore.upsert(&conversation) {
                                                        tracing::error!(error = %e, "Failed to create conversation");
                                                        return;
                                                    }
                                                    selected_conversation_id.set(Some(id.clone()));
                                                    id
                                                };

                                                if let Err(e) = manager
                                                    .start(
                                                        conversation_id,
                                                        provider_cfg,
                                                        model,
                                                        orch_for_task.ai_tools(),
                                                        prompt,
                                                    )
                                                    .await
                                                {
                                                    tracing::error!(error = %e, "Failed to start generation");
                                                    return;
                                                }

                                                input.set(String::new());
                                                stream_text.set(String::new());
                                                thinking_text.set(String::new());
                                                is_thinking.set(false);
                                                generation_paused.set(false);
                                                generation_active.set(true);
                                                reload_tick.set(reload_tick() + 1);
                                            });
                                        }
                                    },
                                    Icon { icon: MdSend, size: 16, color: "#111827" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ThinkingBanner(
    status: &'static str,
    status_class: &'static str,
    text: String,
    running: bool,
) -> Element {
    rsx! {
        div { class: "w-full rounded-xl bg-zinc-900/80 px-3 py-2 flex items-center gap-3",
            if running {
                div { class: "w-4 h-4 rounded-full border-2 border-zinc-600 border-t-cyan-400 animate-spin shrink-0" }
            } else {
                div { class: "w-2.5 h-2.5 rounded-full bg-zinc-500 shrink-0" }
            }
            div { class: "min-w-0 flex-1 flex items-center gap-3",
                div { class: "shrink-0 px-2 py-1 rounded-full border text-[10px] font-semibold uppercase tracking-[0.12em] {status_class}",
                    "{status}"
                }
                div { class: "text-sm text-zinc-300 truncate", "{text}" }
            }
        }
    }
}

#[component]
fn MessageView(message: Message) -> Element {
    let bubble_class = match message.role {
        Role::User => "ml-auto bg-cyan-500/10 border border-cyan-500/40 text-zinc-100",
        Role::Assistant => "mr-auto text-zinc-100",
        Role::Tool => {
            "mr-auto bg-violet-500/5 border border-violet-500/20 text-violet-200"
        },
        Role::System => {
            "mr-auto bg-amber-500/5 border border-amber-500/20 text-amber-100"
        },
    };

    let reader = AiPrefsReader;
    let provider = reader.provider().unwrap_or(ProviderKind::Ollama);
    let model_name = model_for_provider(&reader, provider);

    rsx! {
        div { class: "max-w-[85%] rounded-xl px-6 py-3 text-sm {bubble_class}",
            if message.role == Role::User {
                div { class: "text-[10px] uppercase tracking-wider text-cyan-400 mb-2 font-semibold", "Вы" }
            } else if message.role == Role::Assistant {
                div { class: "text-[10px] uppercase tracking-wider text-zinc-500 mb-2 font-semibold", "Ассистент / {model_name}" }
            } else if message.role == Role::System {
                div { class: "text-[10px] uppercase tracking-wider text-amber-400 mb-2 font-semibold", "Система / {model_name}" }
            } else if message.role == Role::Tool {
                div { class: "text-[10px] uppercase tracking-wider text-violet-400 mb-2 font-semibold", "Инструмент / {model_name}" }
            }

            if message.role == Role::Tool {
                ToolMessageContent { message: message.clone() }
            } else {
                MarkdownMessage { content: message.content.clone() }
            }
        }
    }
}

#[component]
fn ToolMessageContent(message: Message) -> Element {
    let (tool_name, event_kind, details) = tool_display_data(&message);
    let has_details = !details.is_empty();

    let badge_class = if event_kind == "result" {
        "bg-fuchsia-500/15 border border-fuchsia-400/30 text-fuchsia-200"
    } else {
        "bg-sky-500/10 border border-sky-400/25 text-sky-200"
    };
    let badge_label = if event_kind == "result" {
        "результат"
    } else {
        "вызов"
    };

    rsx! {
        div { class: "relative group",
            div { class: "inline-flex items-center gap-2 rounded-lg px-2.5 py-1.5 {badge_class}",
                span { class: "text-sm", if event_kind == "result" { "◉" } else { "⚙" } }
                span { class: "font-mono text-[11px] uppercase tracking-[0.16em]", "{badge_label}" }
                span { class: "font-mono text-xs text-zinc-100", "{tool_name}" }
            }

            if has_details {
                div { class: "pointer-events-none absolute left-0 top-[calc(100%+8px)] z-30 hidden min-w-[300px] max-w-[520px] group-hover:block",
                    div { class: "rounded-xl border border-violet-400/30 bg-zinc-950/95 px-2 py-2 shadow-[0_18px_40px_rgba(0,0,0,0.45)]",
                        table { class: "w-full text-xs border-separate border-spacing-y-1",
                            tbody {
                                for (key, value) in details.iter() {
                                    tr {
                                        td { class: "align-top pr-3 text-violet-300/90 font-mono whitespace-nowrap", "{key}" }
                                        td { class: "align-top text-zinc-200 break-all", "{value}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
