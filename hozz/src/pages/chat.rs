use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use ai::{
    AiPrefsReader, Conversation, ConversationStore, GenerationManager, Message,
    MessageStore, ProviderConfig, ProviderKind, Role, ToolCallStatus,
};
use dioxus::{document::eval, logger::tracing, prelude::*};
use dioxus_free_icons::icons::{md_action_icons::MdDone, md_alert_icons::MdError};
use dioxus_icons::lucide::{
    CircleStop, Loader, MessageCircle, ReceiptText, Send, Trash2,
};
use shared::{
    ai::AiRegistry,
    apps::{LoggingLayer, Orchestrator},
};

use crate::{components::message::MarkdownMessage, utils::Icon};

/// Преобразует JSON-значение в строку, чтобы его можно было показать в карточке деталей инструмента.
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

/// Извлекает название tool и его аргументы/результат из raw/content payload, которые приходят от генератора.
fn extract_tool_details(raw: &str, content: &str) -> (String, Vec<(String, String)>) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(function) = val.get("function") {
            let name = function
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("tool")
                .to_string();

            let mut details = Vec::new();
            if let Some(args) = function.get("arguments") {
                if let Some(obj) = args.as_object() {
                    for (k, v) in obj {
                        details.push((k.clone(), value_to_text(v)));
                    }
                } else {
                    details.push(("arguments".to_string(), value_to_text(args)));
                }
            }
            return (name, details);
        }

        if let Some(result) = val.get("result") {
            let name = val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("tool")
                .to_string();

            let mut details = Vec::new();
            if let Some(obj) = result.as_object() {
                for (k, v) in obj {
                    details.push((k.clone(), value_to_text(v)));
                }
            } else {
                details.push(("result".to_string(), value_to_text(result)));
            }
            return (name, details);
        }
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        let name = val
            .get("name")
            .or_else(|| val.get("tool"))
            .and_then(|v| v.as_str())
            .unwrap_or("tool")
            .to_string();

        let mut details = Vec::new();
        if let Some(obj) = val.as_object() {
            for (k, v) in obj {
                if k != "name" && k != "tool" {
                    details.push((k.clone(), value_to_text(v)));
                }
            }
        }
        return (name, details);
    }

    (
        "tool".to_string(),
        vec![("payload".to_string(), content.to_string())],
    )
}

/// Одно обращение к инструменту, полученное слиянием его call+result-сообщений
/// (сопоставленных по `tool_call_id` в `raw`), чтобы UI показывал один бейдж на вызов.
#[derive(Clone, PartialEq)]
struct ToolGroup {
    name: String,
    status: ToolCallStatus,
    args: Vec<(String, String)>,
    result: Vec<(String, String)>,
}

/// Элемент таймлайна чата: обычное сообщение или смерженный бейдж вызова инструмента.
#[derive(Clone)]
enum TimelineItem {
    Regular(Message),
    Tool(ToolGroup),
}

/// Считывает `tool_call_id` из `raw`, если он там есть.
fn tool_call_id_from_raw(raw: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()?
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

/// Схлопывает вызов+результат каждого инструмента в один `ToolGroup`, сохраняя порядок остальных сообщений.
///
/// Многие провайдеры (например Ollama) не возвращают `tool_call_id`, поэтому при его
/// отсутствии результат сопоставляется с самым старым ещё не завершённым вызовом (FIFO) —
/// корректно, пока инструменты выполняются последовательно, а не параллельно.
fn build_timeline(messages: &[Message]) -> Vec<TimelineItem> {
    let mut timeline: Vec<TimelineItem> = Vec::with_capacity(messages.len());
    let mut group_slots: HashMap<String, usize> = HashMap::new();
    let mut pending: VecDeque<usize> = VecDeque::new();

    for msg in messages {
        if msg.role != Role::Tool {
            timeline.push(TimelineItem::Regular(msg.clone()));
            continue;
        }

        let tool_call_id = tool_call_id_from_raw(&msg.raw);
        let is_result = msg
            .raw
            .parse::<serde_json::Value>()
            .map(|v| v.get("result").is_some())
            .unwrap_or(false);

        if is_result {
            let status = if msg.raw.contains("\"status\":\"error\"") {
                ToolCallStatus::Error
            } else {
                ToolCallStatus::Success
            };
            let (name, result) = extract_tool_details(&msg.raw, &msg.content);

            let matched_idx = tool_call_id
                .as_ref()
                .and_then(|id| group_slots.remove(id))
                .or_else(|| {
                    while let Some(idx) = pending.pop_front() {
                        if let Some(TimelineItem::Tool(group)) = timeline.get(idx)
                            && group.status == ToolCallStatus::Running
                        {
                            return Some(idx);
                        }
                    }
                    None
                });

            if let Some(idx) = matched_idx
                && let Some(TimelineItem::Tool(group)) = timeline.get_mut(idx)
            {
                group.status = status;
                group.result = result;
                continue;
            }

            timeline.push(TimelineItem::Tool(ToolGroup {
                name,
                status,
                args: Vec::new(),
                result,
            }));
        } else {
            let (name, args) = extract_tool_details(&msg.raw, &msg.content);
            timeline.push(TimelineItem::Tool(ToolGroup {
                name,
                status: ToolCallStatus::Running,
                args,
                result: Vec::new(),
            }));
            let idx = timeline.len() - 1;
            pending.push_back(idx);
            if let Some(id) = tool_call_id {
                group_slots.insert(id, idx);
            }
        }
    }

    timeline
}

/// Возвращает цветовую схему для бейджа провайдера в списке диалогов.
fn provider_badge_class(kind: &ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Ollama => "bg-orange-500/10 text-orange-400 border-orange-500/20",
        ProviderKind::Gemini => "bg-blue-500/10 text-blue-400 border-blue-500/20",
        ProviderKind::Copilot => "bg-purple-500/10 text-purple-400 border-purple-500/20",
    }
}

/// Возвращает цветовую схему бейджа вызова инструмента по его текущему статусу.
fn tool_badge_class(status: ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Running => {
            "bg-violet-500/10 text-violet-400 border-violet-500/30"
        },
        ToolCallStatus::Success => {
            "bg-emerald-500/10 text-emerald-400 border-emerald-500/30"
        },
        ToolCallStatus::Error => "bg-rose-500/10 text-rose-400 border-rose-500/30",
    }
}

/// Компактный слепок live-статусов вызовов инструментов из `GenerationSnapshot`,
/// чтобы обнаружить изменение и обновить список сообщений раньше конца генерации.
fn tool_calls_signature(tool_calls: &[ai::ToolCallView]) -> String {
    tool_calls
        .iter()
        .map(|call| {
            let status = match call.status {
                ToolCallStatus::Running => 'r',
                ToolCallStatus::Success => 's',
                ToolCallStatus::Error => 'e',
            };
            format!("{}:{status}", call.id)
        })
        .collect::<Vec<_>>()
        .join(",")
}

mod sidebar {

    use super::*;

    /// Боковая панель с перечнем диалогов и кнопкой создания нового.
    #[component]
    pub(super) fn ChatSidebar(
        conversations: Vec<Conversation>,
        selected_conversation_id: Option<String>,
        on_select_conversation: Callback<String>,
        on_create_conversation: Callback<()>,
        on_delete_conversation: Callback<String>,
    ) -> Element {
        rsx! {
            div { class: "flex w-72 flex-col border-r border-zinc-800/80 bg-zinc-900/40 backdrop-blur-xl",
                div { class: "flex items-center justify-between border-b border-zinc-800/80 p-4",
                    h2 { class: "font-semibold text-sm tracking-wide text-zinc-200", "Диалоги" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        onclick: move |_| on_create_conversation.call(()),
                            title: "Новый чат",
                            MessageCircle { size: "18px"}
                    }
                }

                div { class: "flex-1 overflow-y-auto p-3 space-y-2 custom-scrollbar",
                    for conversation in conversations.iter() {
                        ChatConversationItem {
                            conversation: conversation.clone(),
                            is_selected: selected_conversation_id.as_ref() == Some(&conversation.id),
                            on_select_conversation,
                            on_delete_conversation,
                        }
                    }
                }
            }
        }
    }

    /// Один элемент списка диалогов. В будущем можно выделить отдельную карточку с дополнительной информацией.
    #[component]
    fn ChatConversationItem(
        conversation: Conversation,
        is_selected: bool,
        on_select_conversation: Callback<String>,
        on_delete_conversation: Callback<String>,
    ) -> Element {
        let badge_style = provider_badge_class(&conversation.provider);
        let selected_class = if is_selected {
            "bg-violet-500/10 border-violet-500/30 text-violet-200 shadow-sm"
        } else {
            "border-transparent text-zinc-400 hover:bg-zinc-900/80 hover:text-zinc-200"
        };

        rsx! {
            div {
                key: "{conversation.id}",
                class: format!(
                    "group flex items-center justify-between rounded-xl px-3 py-2.5 gap-2 text-xs transition-all cursor-pointer border {}",
                    selected_class
                ),
                onclick: {
                    let conversation_id = conversation.id.clone();
                    move |_| on_select_conversation.call(conversation_id.clone())
                },

                button {
                    class: "p-1.5 rounded-md text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer opacity-0 group-hover:opacity-100",
                    title: "Удалить",
                    onclick: {
                        let conversation_id = conversation.id.clone();
                        move |evt| {
                            evt.stop_propagation();
                            on_delete_conversation.call(conversation_id.clone());
                        }
                    },
                    Trash2 { size: "16px" }
                }

                span { class: "truncate font-medium pr-2", "{conversation.title}" }
                span { class: "rounded-md border px-1.5 py-0.5 font-mono text-[10px] uppercase font-semibold {badge_style}",
                    "{conversation.provider}"
                }
            }
        }
    }
}

mod chat_area {
    use super::*;

    /// Шапка чата с названием, моделью и статусом генерации.
    #[component]
    pub(super) fn ChatHeader(
        conversation: Conversation,
        is_thinking: bool,
        generation_active: bool,
    ) -> Element {
        rsx! {
            div { class: "flex items-center justify-between px-6 py-4 pb-2 bg-zinc-900/20 backdrop-blur-md",
                div { class: "flex items-center gap-3",
                    h3 { class: "font-medium text-sm text-zinc-100", "{conversation.title}" }
                    span {
                        class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold tracking-wide uppercase border bg-white/5 shadow-sm text-zinc-400 border-white/10 cursor-help",
                        "{conversation.provider} • {conversation.model}"
                    }
                }
                BadgeThinking {
                    is_thinking,
                    generation_active,
                }
            }
        }
    }

    /// Список сообщений и стримингового текста. Здесь же можно позже вынести логику рендера сообщений в отдельный helper.
    #[component]
    pub(super) fn MessageList(
        messages: Vec<Message>,
        stream_text: String,
        generation_active: bool,
    ) -> Element {
        let timeline = build_timeline(&messages);

        rsx! {
            div {
                id: "chat-scroll-container",
                class: "flex-1 overflow-y-auto p-6 space-y-6 custom-scrollbar",
                for (idx , item) in timeline.iter().enumerate() {
                    div {
                        key: "{idx}",
                        class: format!(
                            "flex flex-col {}",
                            match item {
                                TimelineItem::Regular(msg) if msg.role == Role::User => "items-end",
                                _ => "items-start",
                            }
                        ),
                        match item {
                            TimelineItem::Tool(group) => rsx! {
                                ToolCallBadge { group: group.clone() }
                            },
                            TimelineItem::Regular(msg) => rsx! {
                                div {
                                    class: format!(
                                        "max-w-[85%] text-sm leading-relaxed {}",
                                        if msg.role == Role::User {
                                            "rounded-2xl border bg-violet-500/10 border-violet-500/30 px-4 py-3 text-zinc-100"
                                        } else {
                                            "px-0 py-0 text-zinc-200"
                                        }
                                    ),
                                    MarkdownMessage { content: msg.content.clone() }
                                }
                            },
                        }
                    }
                }

                if generation_active && !stream_text.is_empty() {
                    div { class: "flex flex-col items-start",
                        div { class: "max-w-[85%] px-0 py-0 text-sm leading-relaxed text-zinc-100",
                            MarkdownMessage { content: stream_text }
                        }
                    }
                }
            }
        }
    }

    /// Поле ввода и группа кнопок отправки/паузы/остановки.
    #[component]
    pub(super) fn ChatComposer(
        input_text: String,
        generation_active: bool,
        on_input: Callback<String>,
        on_submit: Callback<()>,
        on_stop: Callback<()>,
        on_refresh_tools: Callback<()>,
    ) -> Element {
        rsx! {
            div { class: "px-4 py-3 bg-transparent",
                div { class: "relative flex flex-col rounded-2xl border border-white/10 bg-zinc-900/50 shadow-sm focus-within:border-violet-500/50 transition-all",
                    textarea {
                        class: "w-full resize-none bg-transparent p-4 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none custom-scrollbar min-h-[56px] max-h-[200px]",
                        placeholder: "Напишите сообщение...",
                        value: "{input_text}",
                        oninput: move |evt| on_input.call(evt.value()),
                        onkeydown: move |evt| {
                            let key = evt.key();
                            if key == Key::Enter && !evt.modifiers().shift() && !evt.modifiers().ctrl() {
                                evt.prevent_default();
                                on_submit.call(());
                            } else if key == Key::Escape && generation_active {
                                evt.prevent_default();
                                on_stop.call(());
                            }
                        },
                    }

                    div { class: "flex items-center justify-between px-4 py-2.5 bg-transparent rounded-b-2xl",
                        div { class: "text-xs font-mono text-zinc-500", "Enter для отправки • Shift+Enter для переноса • Esc для остановки" }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "flex items-center justify-center rounded-xl border border-white/10 bg-zinc-800/80 p-2 text-zinc-300 hover:bg-zinc-700 hover:text-white transition-all cursor-pointer",
                                title: "Обновить инструменты",
                                onclick: move |_| on_refresh_tools.call(()),
                                span { class: "text-lg leading-none", "↻" }
                            }
                            if generation_active {
                                button {
                                    class: "flex items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10 p-2 text-red-400 hover:bg-red-500/20 transition-all cursor-pointer shadow-lg shadow-red-500/20",
                                    onclick: move |_| on_stop.call(()),
                                    title: "Остановить",
                                    CircleStop { size: "20px", style: "background-color: #ff5e5e" }
                                }
                            } else {
                                button {
                                    class: "flex items-center justify-center rounded-xl bg-violet-600 p-2 text-white hover:bg-violet-500 transition-all disabled:opacity-50 shadow-lg shadow-violet-600/20 cursor-pointer hover:shadow-violet-600/40",
                                    disabled: input_text.trim().is_empty(),
                                    onclick: move |_| on_submit.call(()),
                                    Send { size: "24px" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Пустое состояние для случая, когда диалог ещё не выбран.
    /// TODO: в будущем можно объединить его с общим состоянием загрузки/ошибок, чтобы не держать отдельные ветки UI.
    #[component]
    pub(super) fn EmptyChatState() -> Element {
        rsx! {
            div { class: "flex flex-1 flex-col items-center justify-center text-zinc-500 space-y-3",
                MessageCircle { size: "30px" }
                p { class: "text-sm font-medium", "Выберите или создайте новый диалог" }
            }
        }
    }

    #[component]
    pub(super) fn BadgeThinking(is_thinking: bool, generation_active: bool) -> Element {
        if !generation_active {
            return rsx! {};
        }

        // Возвращаем полные строки классов для Tailwind
        let (classes, icon, text) = if is_thinking {
            (
                // Используем твои цвета, но добавляем прозрачность (/20) для фона и рамки (/30)
                "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[10px] font-bold tracking-wide uppercase border shadow-sm bg-[#47568f]/20 text-[#a8bcf8] border-[#47568f]/30",
                rsx!(Loader { size: "13px" }),
                "Размышляет",
            )
        } else {
            (
                "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[10px] font-bold tracking-wide uppercase border shadow-sm bg-[#5c3d76]/20 text-[#d5b8f7] border-[#5c3d76]/30",
                rsx!(ReceiptText { size: "13px" }),
                "Отвечает",
            )
        };

        rsx! {
            span {
                class: "{classes}",
                {icon}
                "{text}"
            }
        }
    }

    /// Бейдж одного вызова инструмента: лоадер пока идёт выполнение, иконка статуса после результата.
    #[component]
    pub(super) fn ToolCallBadge(group: ToolGroup) -> Element {
        let badge_class = tool_badge_class(group.status);
        let mut details = group.args.clone();
        details.extend(group.result.clone());
        let has_details = !details.is_empty();

        let (status_icon, status_label) = match group.status {
            ToolCallStatus::Running => (rsx!(Loader { size: "13px" }), "вызов"),
            ToolCallStatus::Success => (
                rsx!(Icon {
                    icon: MdDone,
                    size: 13,
                    color: "#34d399"
                }),
                "готово",
            ),
            ToolCallStatus::Error => (
                rsx!(Icon {
                    icon: MdError,
                    size: 13,
                    color: "#fb7185"
                }),
                "ошибка",
            ),
        };

        rsx! {
            div { class: "relative group",
                div { class: "inline-flex items-center gap-2 rounded-lg border px-2.5 py-1.5 {badge_class}",
                    span { class: "flex items-center", {status_icon} }
                    span { class: "font-mono text-[11px] uppercase tracking-[0.16em]", "{status_label}" }
                    span { class: "font-mono text-xs text-zinc-100 font-semibold", "{group.name}" }
                }

                if has_details {
                    div { class: "pointer-events-none absolute left-0 top-[calc(100%+8px)] z-30 hidden min-w-[300px] max-w-[520px] group-hover:block",
                        div { class: "rounded-xl border border-violet-400/30 bg-zinc-950/95 p-3 shadow-[0_18px_40px_rgba(0,0,0,0.6)] backdrop-blur-md",
                            table { class: "w-full text-xs border-separate border-spacing-y-1",
                                tbody {
                                    for (key , value) in details.iter() {
                                        tr {
                                            td { class: "align-top pr-3 text-violet-300 font-mono whitespace-nowrap", "{key}" }
                                            td { class: "align-top text-zinc-300 font-mono break-all", "{value}" }
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
}

#[component]
pub fn ChatPage() -> Element {
    let orch = consume_context::<Arc<Orchestrator>>();

    let mut selected_conversation_id = use_signal(|| Option::<String>::None);
    let mut input_text = use_signal(String::new);
    let mut stream_text = use_signal(String::new);
    let mut is_thinking = use_signal(|| false);
    let mut generation_active = use_signal(|| false);

    let mut reload_tick = use_signal(|| 0);
    let mounted = use_hook(|| Arc::new(AtomicBool::new(true)));
    let mounted_for_drop = mounted.clone();

    use_drop(move || {
        mounted_for_drop.store(false, Ordering::SeqCst);
    });

    let conversations = use_memo(move || {
        let _ = reload_tick();
        ConversationStore.list().unwrap_or_default()
    });

    let selected_conversation = use_memo(move || {
        let _ = reload_tick();
        selected_conversation_id()
            .and_then(|id| ConversationStore.find(&id).ok().flatten())
    });

    let current_messages = use_memo(move || {
        let _ = reload_tick();
        if let Some(id) = selected_conversation_id() {
            let mut messages = MessageStore.list(&id).unwrap_or_default();
            messages.sort_by_key(|msg| msg.timestamp);
            messages
        } else {
            vec![]
        }
    });

    // Обработчик авто-скролла вниз при стриминге.
    // TODO: позже можно вынести эту логику в маленький helper, чтобы не дублировать поведение для разных состояний UI.
    use_effect(move || {
        let _ = stream_text();
        let _ = current_messages();
        spawn(async move {
            let _ = eval(
                r#"
                const el = document.getElementById('chat-scroll-container');
                if (el) { el.scrollTop = el.scrollHeight; }
            "#,
            )
            .await;
        });
    });

    let provider_config = move || -> ProviderConfig {
        let prefs = AiPrefsReader;
        let provider = prefs.effective_provider(ProviderKind::Ollama);

        match provider {
            ProviderKind::Ollama => ProviderConfig::Ollama {
                base_url: prefs.ollama_base_url(),
            },
            ProviderKind::Gemini => ProviderConfig::Gemini {
                api_key: prefs.gemini_api_key().unwrap_or_default(),
            },
            ProviderKind::Copilot => ProviderConfig::Copilot {
                api_key: prefs.copilot_api_key().unwrap_or_default(),
            },
        }
    };

    // Группа обработчиков состояния чата.
    // TODO: в следующем проходе их можно собрать в один маленький helper/struct, чтобы упростить чтение шаблона.
    let mut handle_select_conversation = move |conversation_id: String| {
        selected_conversation_id.set(Some(conversation_id));
        generation_active.set(false);
        is_thinking.set(false);
        stream_text.set(String::new());
    };

    let mut handle_create_conversation = move || {
        let prefs = AiPrefsReader;
        let provider = prefs.effective_provider(ProviderKind::Ollama);
        let model = prefs.effective_model(provider, Some("llama3.1"));
        let new_conv = Conversation::new("Новый чат", provider, model);
        if ConversationStore.upsert(&new_conv).is_ok() {
            selected_conversation_id.set(Some(new_conv.id));
            generation_active.set(false);
            is_thinking.set(false);
            stream_text.set(String::new());
            reload_tick.write();
        }
    };

    let orch_for_delete = orch.clone();
    let mut handle_delete_conversation = move |conversation_id: String| {
        if let Err(err) = ConversationStore.remove(&conversation_id) {
            orch_for_delete.error(format!("Не удалось удалить диалог: {err}"));
        } else {
            if selected_conversation_id().as_deref() == Some(conversation_id.as_str()) {
                selected_conversation_id.set(None);
                generation_active.set(false);
                is_thinking.set(false);
                stream_text.set(String::new());
            }
            reload_tick.write();
        }
    };

    // Оптимистичная отправка сообщения.
    let mounted_for_send = mounted.clone();
    let orch_for_send = orch.clone();
    let orch_for_request = orch.clone();
    let handle_send: Rc<RefCell<dyn FnMut()>> = Rc::new(RefCell::new(move || {
        let prompt = input_text();
        if prompt.trim().is_empty() {
            return;
        }

        let conv_id = match selected_conversation_id() {
            Some(id) => id,
            None => return,
        };

        let prefs = AiPrefsReader;
        let provider = prefs.effective_provider(ProviderKind::Ollama);
        let model = prefs.effective_model(provider, Some("llama3.1"));

        if let Some(conv) = ConversationStore.find(&conv_id).ok().flatten() {
            let mut conv = conv;
            conv.provider = provider;
            conv.model = model.clone();
            let _ = ConversationStore.upsert(&conv);
        }

        // 1. Мгновенно очищаем инпут и включаем статусы ожидания
        input_text.set(String::new());
        generation_active.set(true);
        is_thinking.set(true);
        stream_text.set(String::new());

        // 2. Мгновенно сохраняем сообщение пользователя локально
        let user_msg = Message::user(prompt.clone());
        if let Err(e) = MessageStore.append(&conv_id, &user_msg) {
            orch_for_send.error(format!("Не удалось сохранить сообщение: {e}"));
            return;
        }

        reload_tick.write();

        if let Some(conv) = ConversationStore.find(&conv_id).ok().flatten() {
            let should_title = conv.title == "Новый чат";
            if should_title {
                let title_config = provider_config();
                let title_model = model.clone();
                let title_prompt = prompt.clone();
                let title_conv_id = conv_id.clone();
                let title_store = ConversationStore;
                let title_messages = MessageStore;
                spawn(async move {
                    let manager = consume_context::<Arc<GenerationManager>>();
                    let _ = manager;
                    let title_result =
                        ai::generate_title(&title_config, &title_model, &title_prompt)
                            .await;
                    if let Ok(title) = title_result
                        && let Ok(Some(mut conv)) = title_store.find(&title_conv_id)
                        && conv.title == "Новый чат"
                    {
                        conv.title = title;
                        conv.updated_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64;
                        let _ = title_store.upsert(&conv);
                    }

                    if let Ok(msgs) = title_messages.list(&title_conv_id)
                        && msgs
                            .iter()
                            .filter(|msg| msg.role == Role::User)
                            .count()
                            > 1
                        && let Ok(Some(mut conv)) = title_store.find(&title_conv_id)
                        && (conv.title == "Новый чат" || conv.title == "...")
                    {
                        conv.updated_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64;
                        let _ = title_store.upsert(&conv);
                    }
                });
            }
        }

        // 3. Запускаем фоновую генерацию
        let config_cl = provider_config();
        let sys_prompt_cl = String::new();
        let model_cl = model;
        let request = ai::GenerationRequest {
            config: config_cl,
            model: model_cl,
            system_prompt: sys_prompt_cl,
            tools: Some(orch_for_request.tool_server()),
            max_tool_turns: 4,
        };
        let mounted_for_spawn = mounted_for_send.clone();
        spawn(async move {
            let manager_cl = consume_context::<Arc<GenerationManager>>();
            if !mounted_for_spawn.load(Ordering::SeqCst) {
                return;
            }

            if let Err(err) = manager_cl
                .start(
                    conv_id.clone(),
                    Arc::new(MessageStore),
                    Arc::new(ConversationStore),
                    request,
                )
                .await
            {
                tracing::error!("Ошибка запуска генерации: {err}");
                if mounted_for_spawn.load(Ordering::SeqCst) {
                    generation_active.set(false);
                    is_thinking.set(false);
                    stream_text.set(String::new());
                }
                return;
            }

            if let Some(mut rx) = manager_cl.subscribe(&conv_id).await {
                let initial_snapshot = rx.borrow().clone();
                let mut last_tool_signature =
                    tool_calls_signature(&initial_snapshot.tool_calls);
                if mounted_for_spawn.load(Ordering::SeqCst) {
                    if !initial_snapshot.text.is_empty() {
                        is_thinking.set(false);
                        stream_text.set(initial_snapshot.text.clone());
                    } else if !initial_snapshot.thinking.is_empty() {
                        is_thinking.set(true);
                    }

                    if initial_snapshot.finished {
                        generation_active.set(false);
                        is_thinking.set(false);
                        stream_text.set(String::new());
                        reload_tick.write();
                        return;
                    }
                }

                loop {
                    if !mounted_for_spawn.load(Ordering::SeqCst)
                        || selected_conversation_id() != Some(conv_id.clone())
                    {
                        break;
                    }

                    if rx.changed().await.is_err() {
                        break;
                    }

                    let snapshot = rx.borrow().clone();
                    if mounted_for_spawn.load(Ordering::SeqCst) {
                        if !snapshot.text.is_empty() {
                            is_thinking.set(false);
                            stream_text.set(snapshot.text);
                        } else if !snapshot.thinking.is_empty() {
                            is_thinking.set(true);
                        }

                        // Обновляем список сообщений сразу, как только меняется статус
                        // вызова инструмента, не дожидаясь конца генерации.
                        let tool_signature = tool_calls_signature(&snapshot.tool_calls);
                        if tool_signature != last_tool_signature {
                            last_tool_signature = tool_signature;
                            reload_tick.write();
                        }

                        if snapshot.finished {
                            generation_active.set(false);
                            is_thinking.set(false);
                            stream_text.set(String::new());
                            reload_tick.write();
                            break;
                        }
                    }
                }
            } else if mounted_for_spawn.load(Ordering::SeqCst) {
                generation_active.set(false);
            }
        });
    }));

    let handle_stop: Rc<RefCell<dyn FnMut()>> = Rc::new(RefCell::new(move || {
        if let Some(conv_id) = selected_conversation_id() {
            spawn(async move {
                let manager_cl = consume_context::<Arc<GenerationManager>>();
                let _ = manager_cl.stop(&conv_id).await;
            });
        }
    }));

    let orch_for_refresh = orch.clone();
    let handle_refresh_tools = move || {
        let _ = orch_for_refresh.refresh_tool_server();
        let has_tavily = AiPrefsReader.tavily_api_key().is_some();

        if has_tavily {
            orch_for_refresh.info("AI-инструменты обновлены: Tavily доступен");
        } else {
            orch_for_refresh.info("AI-инструменты обновлены: Tavily отключён (ключ не задан)");
        }
        reload_tick.write();
    };

    let handle_send_for_button = Rc::clone(&handle_send);
    let handle_stop_for_button = Rc::clone(&handle_stop);
    let mut handle_refresh_tools_for_button = handle_refresh_tools;

    rsx! {
        div { class: "flex h-full w-full bg-zinc-950 text-zinc-100 font-sans overflow-hidden",
            // Боковая панель теперь вынесена в отдельный компонент для лучшей читаемости.
            sidebar::ChatSidebar {
                conversations: conversations(),
                selected_conversation_id: selected_conversation_id(),
                on_select_conversation: move |conversation_id| {
                    handle_select_conversation(conversation_id);
                },
                on_create_conversation: move |_| {
                    handle_create_conversation();
                },
                on_delete_conversation: move |conversation_id| {
                    handle_delete_conversation(conversation_id);
                },
            }

            // Основная область чата теперь тоже разбита на небольшие смысловые блоки.
            div { class: "flex flex-1 flex-col h-full bg-zinc-950 relative",
                if let Some(conv) = selected_conversation() {
                    chat_area::ChatHeader {
                        conversation: conv.clone(),
                        is_thinking: is_thinking(),
                        generation_active: generation_active(),
                    }

                    chat_area::MessageList {
                        messages: current_messages(),
                        stream_text: stream_text(),
                        generation_active: generation_active(),
                    }

                    chat_area::ChatComposer {
                        input_text: input_text(),
                        generation_active: generation_active(),
                        on_input: move |value| input_text.set(value),
                        on_submit: move |_| {
                            handle_send_for_button.borrow_mut()();
                        },
                        on_stop: move |_| {
                            handle_stop_for_button.borrow_mut()();
                        },
                        on_refresh_tools: move |_| {
                            handle_refresh_tools_for_button();
                        },
                    }
                } else {
                    chat_area::EmptyChatState {}
                }
            }
        }
    }
}
