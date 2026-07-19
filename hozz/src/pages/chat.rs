use std::sync::Arc;

use ai::{
    AiPrefsReader, Conversation, ConversationStore, Folder, FolderStore,
    GenerationManager, Message, MessageStore, ProviderConfig, ProviderKind,
    Role,
};
use dioxus::{document::eval, prelude::*};
use dioxus_free_icons::icons::{
    md_av_icons::{MdPause, MdPlayArrow, MdStop}, md_content_icons::{MdInventory, MdSend}, md_navigation_icons::MdMenu,
};
use shared::apps::{LoggingLayer, Orchestrator};

use crate::{components::message::MarkdownMessage, utils::Icon};

fn thinking_preview(input: &str) -> Option<String> {
    let first_non_empty = input.lines().map(str::trim).find(|line| !line.is_empty())?;

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
    let short = short.split_whitespace().take(8).collect::<Vec<_>>().join(" ");

    if short.is_empty() {
        None
    } else {
        Some(short)
    }
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
            let loaded = MessageStore.list(&conversation_id).unwrap_or_default();
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
                        is_thinking.set(!snapshot.finished && !snapshot.thinking.trim().is_empty());
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

    let show_activity_banner = generation_active() && !generation_paused();
    let activity_status = if is_thinking() {
        "Думает"
    } else {
        "Пишет ответ"
    };
    let activity_text = if is_thinking() {
        thinking_preview(&thinking_text()).unwrap_or_else(|| "Формирует план ответа".to_string())
    } else {
        thinking_preview(&thinking_text())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "Формулирует итоговый ответ".to_string())
    };
    let activity_banner_class = if show_activity_banner {
        "max-h-24 opacity-100 translate-y-0 mb-2"
    } else {
        "max-h-0 opacity-0 -translate-y-1 mb-0 pointer-events-none"
    };
    let activity_status_class = if is_thinking() {
        "text-cyan-300 bg-cyan-400/10 border-cyan-400/20"
    } else {
        "text-emerald-300 bg-emerald-400/10 border-emerald-400/20"
    };

    let input_box_class = if generation_active() {
        "w-full min-h-[88px] resize-y bg-black border border-emerald-400/80 rounded-md px-3 py-2 text-sm shadow-[0_0_0_1px_rgba(52,211,153,0.35),0_0_18px_rgba(16,185,129,0.22)] animate-pulse"
    } else {
        "w-full min-h-[88px] resize-y bg-black border border-white/10 rounded-md px-3 py-2 text-sm"
    };

    let chevron_class = if create_menu_open() {
        "transform rotate-90 transition-transform duration-200"
    } else {
        "transform rotate-0 transition-transform duration-200"
    };

    rsx! {
        div {
            class: "h-full w-full flex overflow-hidden bg-zinc-950 text-zinc-200",
            onclick: move |_| {
                context_menu.set(None);
                create_menu_open.set(false);
            },

            div { class: "w-[272px] border-r border-white/10 flex flex-col",
                div { class: "p-3 border-b border-white/10 flex flex-col gap-2",
                    div { class: "relative",
                        div { class: "h-8 inline-flex items-center rounded-md border border-white/10 bg-zinc-900/70 overflow-hidden",
                            button {
                                class: "w-8 h-8 flex items-center justify-center hover:bg-white/5 transition-colors cursor-pointer",
                                title: "Новый диалог",
                                onclick: {
                                    let orch = orch.clone();
                                    move |evt| {
                                        evt.stop_propagation();
                                        let reader = AiPrefsReader;
                                        let provider = reader.provider().unwrap_or(ProviderKind::Gemini);
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
                                Icon { icon: MdSend, size: 14, color: "#e4e4e7" }
                            }
                            button {
                                class: "w-8 h-8 border-l border-white/10 flex items-center justify-center hover:bg-white/5 transition-colors cursor-pointer",
                                title: "Меню создания",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    create_menu_open.set(!create_menu_open());
                                },
                                Icon { icon: MdMenu, size: 14, class: chevron_class }
                            }
                        }

                        if create_menu_open() {
                            div {
                                class: "absolute z-20 top-10 left-0 rounded-md border border-white/10 bg-zinc-950/98 shadow-xl p-1 flex items-center gap-1",
                                onclick: move |evt| evt.stop_propagation(),
                                button {
                                    class: "w-8 h-8 rounded hover:bg-white/5 transition-colors cursor-pointer flex items-center justify-center",
                                    title: "Новый диалог",
                                    onclick: {
                                        let orch = orch.clone();
                                        move |_| {
                                            let reader = AiPrefsReader;
                                            let provider = reader.provider().unwrap_or(ProviderKind::Gemini);
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
                                    Icon { icon: MdSend, size: 14 }
                                    
                                }
                                button {
                                    class: "w-8 h-8 rounded hover:bg-white/5 transition-colors cursor-pointer flex items-center justify-center",
                                    title: "Новая папка",
                                    onclick: move |_| {
                                        creating_folder.set(true);
                                        create_menu_open.set(false);
                                    },
                                    Icon { icon: MdInventory, size: 14 }
                                }
                            }
                        }
                    }

                    if creating_folder() {
                        div { class: "flex gap-1",
                            input {
                                class: "flex-1 h-8 bg-black border border-white/10 rounded px-2 text-xs",
                                placeholder: "Имя папки",
                                value: "{new_folder_name}",
                                oninput: move |e| new_folder_name.set(e.value())
                            }
                            button {
                                class: "h-8 px-3 rounded bg-white/10 text-xs hover:bg-white/15 transition-colors cursor-pointer",
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
                                        creating_folder.set(false);
                                        new_folder_name.set(String::new());
                                        reload_tick.set(reload_tick() + 1);
                                    }
                                },
                                "Создать"
                            }
                        }
                    }
                }

                div { class: "flex-1 overflow-y-auto p-3",
                    div { class: "space-y-1.5",
                        for folder in folders_list.iter() {
                            div {
                                class: "relative rounded-md border px-2 py-1.5 flex items-center gap-2 {row_class(active_folder.as_deref() == Some(folder.id.as_str()))}",
                                oncontextmenu: {
                                    let folder_id = folder.id.clone();
                                    move |evt| {
                                        evt.prevent_default();
                                        evt.stop_propagation();
                                        context_menu.set(Some(ContextMenuTarget::Folder(folder_id.clone())));
                                    }
                                },
                                button {
                                    class: "flex-1 text-left min-w-0",
                                    onclick: {
                                        let folder_id = folder.id.clone();
                                        move |_| {
                                            active_folder_id.set(Some(folder_id.clone()));
                                            selected_conversation_id.set(None);
                                        }
                                    },
                                    div { class: "text-xs text-zinc-100 truncate", "{folder.name}" }
                                }

                                if context_menu().as_ref() == Some(&ContextMenuTarget::Folder(folder.id.clone())) {
                                    div {
                                        class: "absolute top-8 right-1 z-20 min-w-[132px] border border-white/10 bg-zinc-950 rounded-md p-1 shadow-xl",
                                        button {
                                            class: "w-full text-left px-2 py-1.5 rounded text-xs text-pink-300 hover:text-pink-200 hover:bg-pink-500/10 transition-colors cursor-pointer",
                                            onclick: {
                                                let folder_id = folder.id.clone();
                                                let orch = orch.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    if let Err(error) = FolderStore.remove(&folder_id) {
                                                        orch.error(format!("Не удалось удалить папку: {error}"));
                                                        return;
                                                    }
                                                    if active_folder_id().as_deref() == Some(folder_id.as_str()) {
                                                        active_folder_id.set(None);
                                                    }
                                                    context_menu.set(None);
                                                    reload_tick.set(reload_tick() + 1);
                                                }
                                            },
                                            "Удалить папку"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "my-3 h-px bg-white/10" }

                    div { class: "space-y-1.5",
                        if chats_for_view.is_empty() {
                            div { class: "px-2 py-1.5 text-xs text-zinc-600", "Диалогов нет" }
                        }
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
                                        class: "flex-1 text-left min-w-0",
                                        onclick: {
                                            let id = conversation.id.clone();
                                            move |_| selected_conversation_id.set(Some(id.clone()))
                                        },
                                        div { class: "text-xs text-zinc-100 truncate", "{conversation.title}" }
                                        div { class: "text-[10px] text-zinc-500 truncate", "{conversation.model}" }
                                    }
                                }

                                if context_menu().as_ref() == Some(&ContextMenuTarget::Conversation(conversation.id.clone())) {
                                    div {
                                        class: "absolute top-8 right-1 z-20 min-w-[138px] border border-white/10 bg-zinc-950 rounded-md p-1 shadow-xl",
                                        button {
                                            class: "w-full text-left px-2 py-1.5 rounded text-xs text-pink-300 hover:text-pink-200 hover:bg-pink-500/10 transition-colors cursor-pointer",
                                            onclick: {
                                                let id = conversation.id.clone();
                                                let orch = orch.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    if let Err(error) = ConversationStore.remove(&id) {
                                                        orch.error(format!("Не удалось удалить диалог: {error}"));
                                                        return;
                                                    }
                                                    if selected_conversation_id().as_deref() == Some(id.as_str()) {
                                                        selected_conversation_id.set(None);
                                                    }
                                                    context_menu.set(None);
                                                    reload_tick.set(reload_tick() + 1);
                                                }
                                            },
                                            "Удалить диалог"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if active_folder.is_some() {
                    div { class: "p-3 border-t border-white/10",
                        button {
                            class: "w-full h-8 rounded-md border border-white/10 text-xs hover:bg-white/5 transition-colors cursor-pointer",
                            onclick: move |_| active_folder_id.set(None),
                            "Назад"
                        }
                    }
                }
            }

            div { class: "flex-1 flex flex-col min-w-0",
                div { class: "h-12 px-4 border-b border-white/10 flex items-center",
                    div { class: "flex-1" }
                    div { class: "text-sm text-zinc-300 text-center truncate max-w-[70%]", "{title}" }
                    div { class: "flex-1 flex justify-end",
                        if generation_active() {
                            div { class: "text-[11px] text-emerald-400 animate-pulse", "Генерация" }
                        }
                    }
                }

                div { class: "flex-1 overflow-y-auto p-5 space-y-3",
                    if messages().is_empty() && stream_text().is_empty() {
                        div { class: "text-sm text-zinc-500", "Пока нет сообщений" }
                    }

                    for message in messages().iter() {
                        MessageView { message: message.clone() }
                    }

                    if generation_active() && !stream_text().is_empty() {
                        div { class: "max-w-[85%] mr-auto rounded-xl px-4 py-3 text-sm bg-zinc-900 border border-emerald-500/30 text-zinc-100",
                            MarkdownMessage { content: stream_text() }
                        }
                    }
                }

                div { class: "border-t border-white/10 p-3 flex flex-col gap-2",
                    div { class: "overflow-hidden transition-all duration-200 ease-out {activity_banner_class}",
                        ThinkingBanner {
                            status: activity_status,
                            status_class: activity_status_class,
                            text: activity_text,
                        }
                    }

                    textarea {
                        class: "{input_box_class}",
                        placeholder: "Напишите сообщение...",
                        value: "{input}",
                        oninput: move |e| input.set(e.value()),
                        onkeydown: {
                            let orch = orch.clone();
                            let manager = manager.clone();
                            move |evt| {
                                let key = evt.key().to_string();

                                if key == "Enter" && !generation_active() {
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
                                            Err(error) => {
                                                orch_for_task.error(error.to_string());
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
                                                Err(error) => {
                                                    orch_for_task.error(error.to_string());
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

#[component]
fn ThinkingBanner(status: &'static str, status_class: &'static str, text: String) -> Element {
    rsx! {
        div { class: "w-full rounded-2xl border border-white/10 bg-zinc-900/80 px-4 py-3 flex items-center gap-3 shadow-[0_10px_30px_rgba(0,0,0,0.18)]",
            div { class: "w-4 h-4 rounded-full border-2 border-zinc-600 border-t-cyan-400 animate-spin shrink-0" }
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
        Role::Tool => "mr-auto bg-violet-500/5 border border-violet-500/20 text-violet-200",
        Role::System => "mr-auto bg-amber-500/5 border border-amber-500/20 text-amber-100",
    };

    rsx! {
        div { class: "max-w-[85%] rounded-xl px-6 py-3 text-sm {bubble_class}",
            div { class: "text-[10px] uppercase tracking-wider text-zinc-500 mb-2 font-semibold", 
                "{message.role:?}" 
            }
            
            if message.role == Role::Tool {
                div { class: "font-mono text-xs", "🛠 {message.content}" }
            } else {
                MarkdownMessage { content: message.content.clone() }
            }
        }
    }
}