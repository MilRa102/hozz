use std::{collections::HashMap, sync::Arc};

use dioxus::{logger::tracing, prelude::*};
use dioxus_free_icons::icons::md_action_icons::MdAnchor;
use prefs::Category;
use shared::{
    app::orchestrator::Orchestrator,
    infra::{LoggingLayer, PrefsManager},
};

use crate::{
    components::{
        input::SearchInput,
        pet::{ZeroEmpty, ZeroMasked},
        switch::SettingRow,
    },
    utils::Icon,
};

// Для удобства перебора категорий в UI
const CATEGORIES: &[Category] = &[
    Category::System,
    Category::Network,
    Category::Modules,
];

#[component]
pub fn SettingsView() -> Element {
    let orch = consume_context::<Arc<Orchestrator>>();

    // Состояние поиска
    let mut search_query = use_signal(String::new);
    let mut active_category = use_signal(|| Category::System);
    let mut states = use_signal(HashMap::<String, bool>::new);

    let search_text = search_query();
    let is_searching = !search_text.trim().is_empty();

    let displayed_settings = if is_searching {
        orch.registry.search(&search_text)
    } else {
        orch.registry.by_category(active_category())
    };

    let arch = orch.clone();
    let meta_is_active = move |setting_id: String| {
        states()
            .get(&setting_id)
            .copied()
            .unwrap_or_else(|| arch.get_into_bool(&setting_id))
    };

    let popular_tags = orch.registry.all_tags();

    rsx! {
        div { class: "flex h-full w-full bg-transparent overflow-hidden text-zinc-200 animate-fade-in",

            // 👈 ЛЕВЫЙ САЙДБАР (Навигация)
            div { class: "w-56 border-r border-white/5 flex flex-col p-6 gap-6",
                div { class: "text-lg font-semibold tracking-tight", "Настройки" }

                div { class: "flex flex-col gap-1",
                    for category in CATEGORIES {
                        button {
                            class: "text-left px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200",
                            class: if active_category() == *category {
                                "bg-white/10 text-white"
                            } else {
                                "text-zinc-500 hover:bg-white/5 hover:text-zinc-300"
                            },
                            onclick: move |_| {
                                active_category.set(*category);
                                search_query.set(String::new()); // Сбрасываем поиск при клике на категорию
                            },
                            "{category.as_str()}"
                        }
                    }
                }
            }

            // 👉 ПРАВАЯ ЧАСТЬ (Контент)
            div { class: "flex-1 flex flex-col h-full relative",

                // Верхняя панель: Умный поиск и Быстрый доступ
                div { class: "p-6 border-b border-white/5 flex flex-col gap-4 sticky top-0 bg-black/40 backdrop-blur-md z-10",

                    // Поисковая строка
                    SearchInput {
                        class: "",
                        signal: search_query,
                        placeholder: "Поиск по ключевым словам или названиям настроек",
                    }

                    // Теги быстрого доступа
                    div { class: "flex flex-wrap items-center gap-1",
                        span { class: "text-[10px] font-medium text-zinc-500 uppercase tracking-widest shrink-0", "Популярное" }
                        for tag in popular_tags {
                            button {
                                class: "px-2.5 py-1 text-[11px] bg-zinc-900/50 border border-zinc-800 text-zinc-400 rounded-full hover:bg-zinc-800 hover:text-zinc-200 hover:border-zinc-700 transition-all flex items-center gap-1.5 cursor-pointer",
                                onclick: move |_| {
                                    // Убираем скобки и устанавливаем поиск
                                    let clean_tag = tag.replace("[", "").replace("]", "");
                                    search_query.set(clean_tag);
                                },
                                Icon { icon: MdAnchor, size: 12, class: "opacity-70" }
                                span { class: "font-mono truncate max-w-[120px]", "{tag}" }
                            }
                        }
                    }
                }

                // Скроллящийся холст с настройками
                div { class: "flex-1 overflow-y-auto p-8 pb-32",
                    div { class: "max-w-3xl mx-auto flex flex-col gap-6",

                        if !is_searching {
                            // --- ОТОБРАЖЕНИЕ КАТЕГОРИИ ---
                            div { class: "animate-fade-in",
                                h3 { class: "text-[11px] font-mono uppercase tracking-widest text-zinc-500 mb-4 ml-1",
                                    "{active_category().as_str()}"
                                }

                                if displayed_settings.is_empty() {
                                    div { class: "py-16 flex flex-col items-center justify-center text-center border border-dashed border-white/10 rounded-2xl bg-white/[0.01]",
                                        p { class: "text-zinc-400 text-sm font-medium",
                                            ZeroMasked {
                                                title: "А ВАМ ТОЧНО МОЖНО?",
                                                description: "Здесь пусто или вам это не разрешено видеть. Обратитесь к администратору."
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "bg-white/[0.02] border border-white/5 rounded-2xl flex flex-col shadow-sm divide-y divide-white/5",
                                        for meta in displayed_settings {

                                            SettingRow {
                                                key: "{meta.id}",
                                                title: meta.title,
                                                desc: meta.description,
                                                requirements: meta.requirements.to_vec(),
                                                is_active: meta_is_active(meta.id.to_string()),
                                                ontoggle: {
                                                    let orch = orch.clone();
                                                    move |new: bool| {
                                                        let orch = orch.clone();
                                                        let id = meta.id;
                                                        states.write().insert(id.to_string(), new);
                                                        spawn(async move {
                                                            if let Err(e) = orch.toggle_preference(id).await {
                                                                tracing::error!("Ошибка при переключении {}: {}", id, e);
                                                                orch.warning(e.to_string());

                                                                states.write().insert(id.to_string(), !new);
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // --- ОТОБРАЖЕНИЕ ПОИСКА ---
                            div { class: "animate-fade-in",
                                h3 { class: "text-[11px] font-mono uppercase tracking-widest text-amber-500/80 mb-4 ml-1 flex items-center gap-2",
                                    "Результаты поиска"
                                    span { class: "text-zinc-600", "• {displayed_settings.len()} найдено" }
                                }

                                if displayed_settings.is_empty() {
                                    div { class: "py-16 flex flex-col items-center justify-center text-center border border-dashed border-white/10 rounded-2xl bg-white/[0.01]",
                                        p { class: "text-zinc-400 text-sm font-medium",
                                            ZeroEmpty {
                                                title: "НИЧЕГО НЕ НАЙДЕНО",
                                                description: "По вашему запросу ничего не найдено."
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "bg-white/[0.02] border border-white/5 rounded-2xl flex flex-col shadow-sm divide-y divide-white/5",
                                        for meta in displayed_settings {
                                            SettingRow {
                                                key: "{meta.id}",
                                                title: meta.title,
                                                desc: meta.description,
                                                requirements: meta.requirements.to_vec(),
                                                is_active: orch.get_into_bool(meta.id),
                                                ontoggle: {
                                                    let orch = orch.clone();
                                                    move |new: bool| {
                                                        let orch = orch.clone();
                                                        let id = meta.id;
                                                        states.write().insert(id.to_string(), new);
                                                        spawn(async move {
                                                            if let Err(e) = orch.toggle_preference(id).await {
                                                                tracing::error!("Ошибка при переключении {}: {}", id, e);
                                                                orch.warning(e.to_string());

                                                                states.write().insert(id.to_string(), !new);
                                                            }
                                                        });
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
        }
    }
}
