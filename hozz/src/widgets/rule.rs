use std::{cmp::Reverse, sync::Arc, time::Duration};

use dioxus::{logger::tracing, prelude::*};
use dioxus_free_icons::icons::{
    md_content_icons::MdContentCopy, md_navigation_icons::MdCheck,
};
use shared::{
    apps::{LoggingLayer, Orchestrator},
    core::models::rule::{Direction, Rule},
    db::SledManager,
};
use tokio::task::spawn_blocking;

use crate::{
    components::{input::SearchInput, pet::ZeroEmpty},
    utils::{Icon, to_clipboard},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
enum VisibilityMode {
    #[default]
    All,
    Active,
    Inactive,
    Ignored,
}

impl VisibilityMode {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Inactive,
            Self::Inactive => Self::Ignored,
            Self::Ignored => Self::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "Все правила",
            Self::Active => "Только активные",
            Self::Inactive => "Отключенные",
            Self::Ignored => "Скрытые",
        }
    }
    // Enterprise цвета для бейджей фильтра
    fn classes(self) -> &'static str {
        match self {
            Self::All => "bg-zinc-900 border-white/10 text-zinc-300 hover:bg-zinc-800",
            Self::Active => "bg-emerald-500/10 border-emerald-500/20 text-emerald-400",
            Self::Inactive => {
                "bg-zinc-800/50 border-white/5 text-zinc-500 hover:bg-zinc-800"
            },
            Self::Ignored => "bg-amber-500/10 border-amber-500/20 text-amber-500",
        }
    }
}

#[derive(Clone, Default, PartialEq)]
struct TableData {
    items: Vec<Rule>,
    total_count: usize,
    safe_page: usize,
    total_pages: usize,
    start_idx: usize,
    end_idx: usize,
}

#[component]
pub fn RulesTable() -> Element {
    const PER_PAGE: usize = 10;

    let arch = use_context::<Arc<Orchestrator>>();

    let search = use_signal(String::new);
    let mut visibility_mode = use_signal(VisibilityMode::default);
    let mut current_page = use_signal(|| 0usize);

    let mut processed_rules = use_resource(move || {
        let search_query = search();
        let mode = visibility_mode();
        let page = current_page();
        let arch = arch.clone();

        async move {
            spawn_blocking(move || {
                let rules = arch.rules.all().unwrap_or_default();

                let mut filtered: Vec<_> = rules
                    .into_iter()
                    .filter(|r| {
                        let match_search = r.name.to_lowercase().contains(&search_query);
                        let match_status = match mode {
                            VisibilityMode::All => !r.is_ignored(),
                            VisibilityMode::Active => r.is_active() && !r.is_ignored(),
                            VisibilityMode::Inactive => !r.is_active() && !r.is_ignored(),
                            VisibilityMode::Ignored => r.is_ignored(),
                        };
                        match_search && match_status
                    })
                    .collect();

                filtered.sort_by_key(|b| Reverse(b.amt));

                let total_count = filtered.len();
                let total_pages = if total_count == 0 {
                    1
                } else {
                    total_count.div_ceil(PER_PAGE)
                };
                let safe_page = std::cmp::min(page, total_pages.saturating_sub(1));

                let start_idx_raw = safe_page * PER_PAGE;
                let end_idx = std::cmp::min(start_idx_raw + PER_PAGE, total_count);

                let items = filtered[start_idx_raw..end_idx].to_vec();
                let start_idx = if total_count == 0 {
                    0
                } else {
                    start_idx_raw + 1
                };

                TableData {
                    items,
                    total_count,
                    safe_page,
                    total_pages,
                    start_idx,
                    end_idx,
                }
            })
            .await
            .unwrap_or_default()
        }
    });

    let data_opt = processed_rules.read().clone();

    rsx! {
        div { class: "bg-black/20 border border-white/10 rounded-xl shadow-sm flex flex-col overflow-hidden",

            // Тулбар таблицы
            div { class: "p-4 border-b border-white/10 flex flex-col sm:flex-row gap-4 justify-between items-center bg-white/5 shrink-0",
                h2 { class: "text-sm font-semibold text-zinc-200 shrink-0", "Сетевые политики" }

                div { class: "flex items-center gap-3 w-full sm:w-auto",
                    SearchInput {
                        signal: search,
                        placeholder: "Поиск по домену/процессу.."
                    }

                    // Кнопка фильтра
                    button {
                        class: "px-3 py-1.5 border rounded-md text-sm font-medium transition-colors whitespace-nowrap focus:outline-none cursor-pointer {visibility_mode().classes()}",
                        onclick: move |_| {
                            visibility_mode.set(visibility_mode().next());
                            current_page.set(0);
                        },
                        "{visibility_mode().label()}"
                    }
                }
            }

            // Таблица данных
            div { class: "overflow-x-auto",
                table { class: "w-full text-left border-collapse",
                    thead { class: "bg-black/40 border-b border-white/5 text-xs uppercase font-semibold text-zinc-500 tracking-wider",
                        tr {
                            th { class: "px-5 py-3", "Цель / Домен" }
                            th { class: "px-5 py-3 w-24", "Запросов" }
                            th { class: "px-5 py-3 w-32", "Действие" }
                            th { class: "px-5 py-3 w-32", "Статус" }
                            th { class: "px-5 py-3 w-32 text-right", "Управление" }
                        }
                    }
                    tbody { class: "divide-y divide-white/5 text-sm",
                        if let Some(data) = data_opt.as_ref() {
                            if data.items.is_empty() {
                                    ZeroEmpty {
                                        title: "ДОМЕН ИЛИ ПРИЛОЖЕНИЕ НЕ НАЙДЕНО",
                                        description: "Пожалуйста, проверьте правильность введенного адреса или приложения"
                                    }
                            } else {
                                for item in &data.items {
                                    RuleTableRow {
                                        key: "{item.id}",
                                        item: item.clone(),
                                        on_change: move |()| processed_rules.restart(),
                                    }
                                }
                            }
                        } else {
                            tr {
                                td { class: "px-5 py-12 text-center text-zinc-500 text-sm",
                                    "Обработка политик..."
                                }
                            }
                        }
                    }
                }
            }

            // Подвал (Пагинация)
            if let Some(data) = data_opt {
                div { class: "px-5 py-3 border-t border-white/10 bg-white/5 flex flex-col sm:flex-row items-center justify-between gap-4 shrink-0",
                    div { class: "text-sm text-zinc-500 font-medium",
                        "Показано "
                        span { class: "text-zinc-200", "{data.start_idx}-{data.end_idx}" }
                        " из "
                        span { class: "text-zinc-200", "{data.total_count}" }
                    }
                    div { class: "flex items-center gap-2",
                        button {
                            class: "px-3 py-1.5 border border-white/10 rounded-md text-sm font-medium text-zinc-300 bg-zinc-900 hover:bg-zinc-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-1 focus:ring-zinc-700",
                            disabled: data.safe_page == 0,
                            onclick: move |_| current_page.with_mut(|p| *p = p.saturating_sub(1)),
                            "Назад"
                        }
                        span { class: "text-sm text-zinc-500 font-medium px-2",
                            "{data.safe_page + 1} / {data.total_pages}"
                        }
                        button {
                            class: "px-3 py-1.5 border border-white/10 rounded-md text-sm font-medium text-zinc-300 bg-zinc-900 hover:bg-zinc-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-1 focus:ring-zinc-700",
                            disabled: data.safe_page + 1 >= data.total_pages,
                            onclick: move |_| current_page.with_mut(|p| *p += 1),
                            "Вперед"
                        }
                    }
                }
            }
        }
    }
}

// Строка таблицы вместо старой карточки
#[component]
fn RuleTableRow(item: Rule, on_change: EventHandler<()>) -> Element {
    let orch = use_context::<Arc<Orchestrator>>();

    // Оформление бейджа Действия (Direct/Reject/Auto)
    let (dir_bg, dir_text) = match item.direction {
        Direction::Direct => (
            "bg-emerald-500/10 border-emerald-500/20",
            "text-emerald-400",
        ),
        Direction::Reject => (
            "bg-rose-500/10 border-rose-500/20",
            "text-rose-400",
        ),
        Direction::Auto => (
            "bg-amber-500/10 border-amber-500/20",
            "text-amber-400",
        ),
    };

    let is_active = item.is_active();
    let is_ignored = item.is_ignored();
    let mut is_copied = use_signal(|| false);

    let row_class = if is_ignored {
        "opacity-50 bg-black/20" // Игнор уводим на задний план
    } else {
        "hover:bg-white/5 transition-colors"
    };

    let (active_name, active_class) = if is_active {
        (
            "Активно",
            "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.4)]",
        )
    } else {
        ("Отключено", "bg-zinc-600")
    };

    rsx! {
        tr { class: "{row_class}",


            // 1. Цель
            td { class: "flex p-3 max-w-xs",
                button {
                    class: "inline-flex items-center justify-center p-2 mr-2 bg-transparent rounded-md text-zinc-500 hover:bg-white/10 hover:text-zinc-200 transition-colors cursor-pointer",
                    title: "Копировать правило",
                    onclick: {
                        let orch = orch.clone();
                        let i = item.clone();
                        move |_| {
                            let text_to_copy = format!("{} {}", i.name, i.amt);
                            if let Err(e) = to_clipboard(&text_to_copy) {
                                tracing::warn!(error = %e, "Failed to copy to clipboard");
                                orch.warning("Не удалось скопировать в буфер обмена");
                            } else {
                                is_copied.set(true);

                                spawn(async move {
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    is_copied.set(false);
                                });
                            }
                        }
                    },

                    if is_copied() {
                        Icon { icon: MdCheck, size: 13, color: "green" }
                    } else {
                        Icon { icon: MdContentCopy, size: 13 }
                    }
                }
                div { class: "flex flex-col",
                    span { class: "font-semibold text-zinc-200 truncate group-hover:text-white transition-colors", title: "{item.name}", "{item.name}" }
                    span { class: "text-[10px] text-zinc-500 font-mono mt-0.5 uppercase tracking-wider", "{item.target.as_ref()}" }
                }
            }

            // 2. Кол-во запросов
            td { class: "px-5 py-3 font-mono text-zinc-400 text-xs", "{item.amt}" }

            // 3. Действие
            td { class: "px-5 py-3",
                button {
                    class: "px-2 py-0.5 rounded flex items-center justify-center text-[10px] font-bold tracking-wider uppercase border {dir_bg} {dir_text} hover:opacity-80 transition-opacity cursor-pointer",
                    title: "Нажмите для смены",
                    onclick: {
                        let orch = orch.clone();
                        let mut item = item.clone();
                        item.direction = match item.direction {
                            Direction::Direct => Direction::Reject,
                            Direction::Reject => Direction::Auto,
                            Direction::Auto => Direction::Direct,
                        };
                        move |_| {
                            let _ = orch.rules.upsert(&item);
                            on_change.call(());
                        }
                    },
                    "{item.direction.as_ref()}"
                }
            }

            // 4. Статус активности
            td { class: "px-5 py-3",
                div { class: "flex items-center gap-2",
                    div { class: "w-2 h-2 rounded-full {active_class}" }
                    span { class: "text-xs font-medium text-zinc-400", "{active_name}" }
                }
            }

            // 5. Управление
            td { class: "px-5 py-3",
                div { class: "flex justify-end gap-2",
                    button {
                        class: "inline-flex items-center justify-center px-2 py-1 bg-transparent border border-white/10 rounded-md text-xs font-medium text-zinc-400 hover:bg-white/10 hover:text-zinc-100 transition-colors cursor-pointer",
                        onclick: {
                            let orch = orch.clone();
                            let mut i = item.clone();
                            move |_| {
                                i.toggle_active();
                                let _ = orch.rules.upsert(&i);
                                on_change.call(());
                            }
                        },
                        if is_active { "Откл" } else { "Вкл" }
                    }

                    button {
                        class: "inline-flex items-center justify-center px-2 py-1 bg-transparent border border-white/10 rounded-md text-xs font-medium text-zinc-400 hover:bg-white/10 hover:text-zinc-100 transition-colors cursor-pointer",
                        onclick: move |_| {
                            let mut i = item.clone();
                            i.toggle_ignore();
                            let _ = orch.rules.upsert(&i);
                            on_change.call(());
                        },
                        if is_ignored { "Вернуть" } else { "Скрыть" }
                    }
                }
            }
        }
    }
}
