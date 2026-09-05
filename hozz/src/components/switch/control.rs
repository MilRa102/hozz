use std::sync::Arc;

use ai::{AiPrefsReader, ProviderKind, list_ollama_models};
use config::CONF;
use dioxus::{logger::tracing, prelude::*};
use dioxus_icons::lucide::RefreshCw;
use prefs::{SettingMeta, SettingType};
use shared::{ai::AiRegistry, apps::{LoggingLayer, Orchestrator}};

use crate::components::input::{SettingSelect, SettingSelectVertical, SettingSwitch};

#[component]
pub(crate) fn SettingControl(
    meta: SettingMeta,
    value: String,
    provider: String,
    onchange: EventHandler<String>,
) -> Element {
    if meta.id == "ai.model" {
        return rsx! {
            ModelPicker { current_value: value, provider, onchange }
        };
    }

    if meta.id == "ai.api_key.tavily" {
        return rsx! {
            TavilyApiKeyControl { current_value: value, onchange }
        };
    }

    match meta.setting_type {
        SettingType::Toggle => rsx! {
            SettingSwitch {
                is_active: value.parse().unwrap_or(false),
                ontoggle: move |v: bool| onchange.call(v.to_string())
            }
        },
        SettingType::Select(options) => rsx! {
            SettingSelect {
                options: options.iter().map(|opt| (*opt).to_string()).collect(),
                selected: value,
                onselect: move |v| onchange.call(v)
            }
        },
        SettingType::TextInput => rsx! {
            input {
                class: "bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-white/20 transition-colors w-56",
                value: "{value}",
                oninput: move |evt| onchange.call(evt.value().clone()),
            }
        },
        SettingType::NumberInput { min, max } => rsx! {
            input {
                class: "bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-white/20 transition-colors w-40",
                r#type: "number",
                min: "{min}",
                max: "{max}",
                value: "{value}",
                oninput: move |evt| onchange.call(evt.value().clone()),
            }
        },
    }
}

#[component]
fn TavilyApiKeyControl(
    current_value: String,
    onchange: EventHandler<String>,
) -> Element {
    let orch = use_context::<Arc<Orchestrator>>();
    let mut refreshing = use_signal(|| false);

    rsx! {
        div { class: "flex items-center gap-2",
            input {
                class: "bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-white/20 transition-colors w-56",
                value: "{current_value}",
                oninput: move |evt| onchange.call(evt.value().clone()),
            }
            button {
                class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-zinc-700 bg-zinc-950 text-zinc-400 transition-colors hover:border-zinc-500 hover:text-white disabled:opacity-60 disabled:cursor-not-allowed",
                title: if refreshing() { "Обновление инструментов..." } else { "Обновить инструменты Tavily" },
                disabled: refreshing(),
                onclick: {
                    let orch = orch.clone();
                    move |_| {
                        if refreshing() {
                            return;
                        }

                        refreshing.set(true);
                        let _ = orch.refresh_tool_server();
                        let has_tavily = AiPrefsReader.tavily_api_key().is_some();

                        if has_tavily {
                            orch.ok("AI-инструменты обновлены: Tavily доступен");
                        } else {
                            orch.info("AI-инструменты обновлены: Tavily отключён (ключ не задан)");
                        }

                        refreshing.set(false);
                    }
                },
                RefreshCw { size: "17px" }
            }
        }
    }
}

#[component]
fn ModelPicker(
    current_value: String,
    provider: String,
    onchange: EventHandler<String>,
) -> Element {
    let prefs = AiPrefsReader;
    let provider = provider.parse().unwrap_or(ProviderKind::Gemini);
    let base_url = prefs.ollama_base_url();
    let mut refresh_tick = use_signal(|| 0usize);
    let ollama_models = use_resource(move || {
        let _tick = refresh_tick();
        let base_url = base_url.clone();
        async move {
            if provider != ProviderKind::Ollama {
                return Vec::<String>::new();
            }
            match list_ollama_models(&base_url).await {
                Ok(models) => models,
                Err(err) => {
                    tracing::warn!("Ollama model discovery failed: {err}");
                    Vec::new()
                }
            }
        }
    });

    let choices = match provider {
        ProviderKind::Gemini => CONF.ai.gemini_models.clone(),
        ProviderKind::Copilot => CONF.ai.copilot_models.clone(),
        ProviderKind::Ollama => ollama_models.value().read().clone().unwrap_or_default(),
    };

    let selected = if choices.iter().any(|item| item == &current_value) {
        current_value.clone()
    } else if current_value.trim().is_empty() && !choices.is_empty() {
        choices[0].clone()
    } else {
        current_value
    };

    let loading = provider == ProviderKind::Ollama && ollama_models.value().read().is_none();
    let has_error = provider == ProviderKind::Ollama && ollama_models.value().read().as_ref().is_some_and(|models| models.is_empty());

    rsx! {
        div { class: "flex flex-col items-end gap-2 min-w-[16rem]",
            if choices.is_empty() {
                div { class: "flex items-center gap-2",
                    div { class: "flex h-9 w-64 items-center rounded-lg border border-zinc-800 bg-zinc-950 px-3 text-xs text-zinc-500",
                        if provider == ProviderKind::Ollama {
                            "Ollama недоступен или локальных моделей нет"
                        } else {
                            "Нет доступных моделей для этого провайдера"
                        }
                    }
                    if provider == ProviderKind::Ollama {
                        button {
                            class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-zinc-700 bg-zinc-950 text-zinc-400 transition-colors hover:border-zinc-500 hover:text-white",
                            title: "Обновить список моделей Ollama",
                            onclick: move |_| refresh_tick += 1,
                            RefreshCw { size: "17px" }
                        }
                    }
                }
            } else {
                div { class: "flex items-center gap-2",
                    SettingSelectVertical {
                        options: choices.clone(),
                        selected: selected.clone(),
                        onselect: move |v| onchange.call(v)
                    }
                    if provider == ProviderKind::Ollama {
                        button {
                            class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-zinc-700 bg-zinc-950 text-zinc-400 transition-colors hover:border-zinc-500 hover:text-white",
                            title: "Обновить список моделей Ollama",
                            onclick: move |_| refresh_tick += 1,
                            RefreshCw { size: "17px" }
                        }
                    }
                }
            }
            if provider == ProviderKind::Ollama && (loading || has_error || choices.is_empty()) {
                span { class: "text-xs text-zinc-500",
                    if loading {
                        "Загрузка моделей..."
                    } else if has_error {
                        "Не удалось получить список Ollama"
                    } else {
                        "Модели не найдены"
                    }
                }
            }
        }
    }
}
