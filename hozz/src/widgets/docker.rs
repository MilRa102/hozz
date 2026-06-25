use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    io_icons::{IoPlay, IoStop},
    ld_icons::LdArrowLeft,
    md_action_icons::MdAutorenew,
    md_alert_icons::MdError,
    md_navigation_icons::{MdUnfoldLess, MdUnfoldMore},
};
use futures::StreamExt;
use shared::{HumanTime, apps::docker, utils::format_size};

use crate::{route::Route, utils::Icon};

#[component]
pub fn ContainerWidget(
    container: docker::Container,
    on_action: EventHandler<()>,
) -> Element {
    let container_signal = use_signal(|| container.clone());
    let is_running = container_signal.read().state == docker::ContainerStatus::RUNNING;

    // Семантические цвета для статусов
    let (status_bg, status_dot, status_text) = if is_running {
        (
            "bg-emerald-500/10 border-emerald-500/20 text-emerald-400",
            "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]",
            "Работает",
        )
    } else {
        (
            "bg-zinc-500/10 border-zinc-500/20 text-zinc-400",
            "bg-zinc-500",
            "Остановлен",
        )
    };

    rsx! {
        // Карточка: полупрозрачная, при наведении граница немного светлеет (интерактивность)
        div { class: "bg-zinc-900/30 border border-zinc-800/80 rounded-xl shadow-sm backdrop-blur-sm p-4 flex flex-col hover:border-zinc-700 hover:bg-zinc-900/50 transition-all duration-200 group",

            div { class: "flex justify-between items-start mb-4 gap-4",
                Link {
                    to: Route::DockerContainer {
                        id: container.id.clone(),
                    },
                    class: "font-semibold text-zinc-200 group-hover:text-zinc-50 transition-colors truncate text-lg tracking-tight",
                    "{container.names.trim_start_matches('/')}"
                }

                div { class: "flex items-center gap-1.5 px-2 py-1 rounded border {status_bg} text-[10px] uppercase font-bold tracking-widest shrink-0",
                    div { class: "w-1.5 h-1.5 rounded-full {status_dot}" }
                    "{status_text}"
                }
            }

            div { class: "flex flex-col gap-3 mb-5",
                div { class: "flex flex-col gap-0.5",
                    span { class: "text-[10px] text-zinc-500 font-medium uppercase tracking-widest", "Порты" }
                    span { class: "text-xs font-mono text-zinc-300 truncate",
                        if container.ports.is_empty() { "—" } else { "{container.ports}" }
                    }
                }
                div { class: "flex flex-col gap-0.5",
                    span { class: "text-[10px] text-zinc-500 font-medium uppercase tracking-widest", "Создан" }
                    span { class: "text-xs text-zinc-400", "{HumanTime::from(container.created)}" }
                }
            }

            div { class: "mt-auto pt-3 border-t border-zinc-800/50 flex justify-between items-center",
                span { class: "text-[10px] text-zinc-600 font-mono truncate select-all", "{container.id[..12].to_string()}" }

                ControlButtons {
                    container: container_signal.read().clone(),
                    on_action: on_action,
                }
            }
        }
    }
}

#[component]
pub fn DockerContainer(id: String) -> Element {
    let mut container_res = use_resource(move || {
        let id = id.clone();
        async move { docker::Container::inspect(id).await }
    });

    // По умолчанию панель деталей открыта на широких экранах
    let mut show_details = use_signal(|| true);
    let show_details_open = *show_details.read();

    let content = match &*container_res.read() {
        Some(Some(container)) => {
            rsx! {
                div { class: "h-full flex flex-col bg-zinc-950 text-zinc-100",

                    // 1. ШАПКА
                    div { class: "shrink-0 bg-zinc-950/80 backdrop-blur-md border-b border-zinc-800 px-6 py-4 flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 z-10",
                        div { class: "flex flex-col gap-1.5",
                            Link {
                                to: Route::DockerContainers {},
                                class: "text-zinc-500 hover:text-zinc-300 text-[10px] font-semibold uppercase tracking-widest mb-1 inline-flex items-center gap-1 transition-colors",
                                Icon { icon: LdArrowLeft, size: 12 }
                                "Все контейнеры"
                            }
                            div { class: "flex items-baseline gap-3",
                                h1 { class: "text-2xl font-bold text-zinc-50 tracking-tight",
                                    "{container.names.trim_start_matches('/')}"
                                }
                                span { class: "text-xs font-mono text-zinc-500", "{container.id[..12].to_string()}" }
                            }
                        }

                        div { class: "flex items-center gap-4",
                            StatusBadge { container: container.clone() }

                            div { class: "w-px h-6 bg-zinc-800 hidden sm:block" }

                            ControlButtons {
                                container: container.clone(),
                                on_action: move |_| container_res.restart(),
                            }

                            button {
                                class: "px-3 py-1.5 bg-zinc-900/50 border border-zinc-800 rounded-lg text-xs font-medium text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-all flex items-center gap-2 active:scale-[0.98] cursor-pointer",
                                onclick: move |_| show_details.set(!show_details_open),
                                if show_details_open {
                                    Icon { icon: MdUnfoldLess, size: 14 }
                                    "Скрыть Инспектор"
                                } else {
                                    Icon { icon: MdUnfoldMore, size: 14 }
                                    "Показать Инспектор"
                                }
                            }
                        }
                    }

                    // 2. РАБОЧАЯ ОБЛАСТЬ
                    div { class: "flex-1 overflow-hidden p-4 lg:p-6 flex flex-col lg:flex-row gap-6",

                        div { class: "flex-1 min-w-0 flex flex-col",
                            LogViewer { container: container.clone() }
                        }

                        if show_details_open {
                            div { class: "w-full lg:w-[400px] shrink-0 flex flex-col animate-in slide-in-from-right-8 duration-300",
                                div { class: "bg-zinc-900/30 border border-zinc-800/80 rounded-xl shadow-sm flex flex-col h-full overflow-hidden backdrop-blur-sm",
                                    div { class: "px-4 py-3 border-b border-zinc-800/50 shrink-0",
                                        h2 { class: "text-xs font-semibold text-zinc-300 uppercase tracking-widest", "Свойства контейнера" }
                                    }

                                    div { class: "flex-1 overflow-y-auto p-2 divide-y divide-zinc-800/50",
                                        InfoRow { title: "Создан".to_string(), value: container.created }
                                        InfoRow { title: "Образ".to_string(), value: container.image.clone() }
                                        InfoRow { title: "Команда".to_string(), value: container.command.clone() }
                                        InfoRow { title: "Порты".to_string(), value: container.ports.clone() }
                                        InfoRow { title: "RW Размер".to_string(), value: format_size(container.size_rw) }
                                        InfoRow { title: "RootFS".to_string(), value: format_size(container.size_root_fs) }

                                        if !container.labels.is_empty() {
                                            div { class: "pt-4 pb-2 px-3 text-[10px] font-bold text-zinc-500 uppercase tracking-wider mt-2", "Метки (Labels)" }
                                            for (key, value) in &container.labels {
                                                if !value.is_empty() {
                                                    InfoRow { title: key.clone(), value: value.clone() }
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
        },
        Some(None) => rsx! {
            div { class: "h-full flex items-center justify-center bg-zinc-950",
                div { class: "bg-zinc-900/30 border border-zinc-800 p-8 rounded-xl text-center flex flex-col items-center",
                    Icon { icon: MdError, size: 40, class: "text-zinc-600 mb-4" }
                    h2 { class: "text-lg font-semibold text-zinc-200", "Контейнер не найден" }
                    p { class: "text-sm text-zinc-500 mt-1", "Возможно, он был удален или ID указан неверно." }
                    Link {
                        to: Route::DockerContainers {},
                        class: "mt-6 px-4 py-2 bg-zinc-100 text-zinc-900 text-sm font-medium rounded-lg hover:bg-white transition-all active:scale-[0.98] shadow-sm",
                        "Вернуться к списку"
                    }
                }
            }
        },
        None => rsx! {
            div { class: "h-full flex items-center justify-center bg-zinc-900 gap-3 text-zinc-500",
                div { class: "w-5 h-5 border-2 border-zinc-300 border-t-blue-500 rounded-full animate-spin" }
                "Сбор информации о контейнере..."
            }
        },
    };

    rsx! { {content} }
}

#[component]
fn InfoRow(title: String, value: String) -> Element {
    rsx! {
        div { class: "py-2.5 px-3 flex flex-col gap-1 hover:bg-zinc-800/30 transition-colors group rounded-md",
            span { class: "text-[10px] font-medium text-zinc-500 uppercase tracking-widest group-hover:text-zinc-400 transition-colors",
                "{title}"
            }
            span { class: "text-[13px] font-mono text-zinc-300 break-all",
                "{value}"
            }
        }
    }
}

#[component]
fn StatusBadge(container: docker::Container) -> Element {
    let is_running = container.state == docker::ContainerStatus::RUNNING;
    let (status_bg, status_dot, status_text) = if is_running {
        (
            "bg-emerald-500/10 border-emerald-500/20 text-emerald-400",
            "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]",
            "Работает",
        )
    } else if container.state == docker::ContainerStatus::EXITED {
        (
            "bg-red-500/10 border-red-500/20 text-red-400",
            "bg-red-500",
            "Остановлен",
        )
    } else {
        (
            "bg-amber-500/10 border-amber-500/20 text-amber-400",
            "bg-amber-500",
            "Ожидание",
        )
    };

    rsx! {
        div { class: "flex items-center gap-2",
            div { class: "flex items-center gap-1.5 px-2.5 py-1 rounded-md border {status_bg} text-[10px] uppercase font-bold tracking-widest",
                div { class: "w-1.5 h-1.5 rounded-full {status_dot}" }
                "{status_text}"
            }
            span { class: "px-2.5 py-1 rounded-md border border-zinc-800 bg-zinc-950/50 text-[10px] font-mono text-zinc-400 font-medium whitespace-nowrap",
                "{container.status}"
            }
        }
    }
}

#[allow(clippy::clone_on_copy)]
#[allow(clippy::await_holding_invalid_type)]
#[component]
pub fn ControlButtons(
    container: docker::Container,
    on_action: EventHandler<()>,
) -> Element {
    let container = use_signal(|| container.clone());
    let mut is_loading = use_signal(|| false);
    let is_running = container.read().state == docker::ContainerStatus::RUNNING;

    rsx! {
        div { class: "flex items-center gap-1",

            // Кнопка Старт / Стоп
            if is_running {
                button {
                    class: "p-1.5 text-zinc-500 hover:text-red-400 hover:bg-red-950/40 rounded-md transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: *is_loading.read(),
                    title: "Остановить",
                    onclick: move |_| {
                        is_loading.set(true);
                        spawn(async move {
                            if container.read().stop().await { on_action.call(()); }
                        });
                    },
                    if *is_loading.read() {
                        div { class: "w-4 h-4 border-2 border-zinc-300 border-t-rose-500 rounded-full animate-spin" }
                    } else {
                        Icon { icon: IoStop, size: 16 }
                    }
                }
            } else {
                button {
                    class: "p-1.5 text-zinc-500 hover:text-emerald-400 hover:bg-emerald-950/40 rounded-md transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: *is_loading.read(),
                    title: "Запустить",
                    onclick: move |_| {
                        is_loading.set(true);
                        spawn(async move {
                            if container.read().start().await { on_action.call(()); }
                            is_loading.set(false); // Включаем обратно, если запуск не удался
                        });
                    },
                    if *is_loading.read() {
                        div { class: "w-4 h-4 border-2 border-zinc-300 border-t-emerald-500 rounded-full animate-spin" }
                    } else {
                        Icon { icon: IoPlay, size: 16 }
                    }
                }
            }

            // Кнопка Рестарт
            button {
                class: "p-1.5 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded-md transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                disabled: *is_loading.read(),
                title: "Перезапустить",
                onclick: move |_| {
                    is_loading.set(true);
                    spawn(async move {
                        if container.read().restart().await { on_action.call(()); }
                        is_loading.set(false);
                    });
                },
                if *is_loading.read() {
                    div { class: "w-4 h-4 border-2 border-zinc-300 border-t-blue-500 rounded-full animate-spin" }
                } else {
                    Icon { icon: MdAutorenew, size: 16 }
                }
            }
        }
    }
}

#[component]
pub fn LogViewer(container: docker::Container) -> Element {
    let mut log_lines = use_signal(Vec::<String>::new);

    let _log_worker = use_coroutine(move |_: UnboundedReceiver<()>| {
        let container = container.clone();
        async move {
            let mut stream = container.logs();
            while let Some(line) = stream.next().await {
                log_lines.with_mut(|lines| {
                    lines.push(line);
                    if lines.len() > 300 {
                        lines.remove(0);
                    }
                });
            }
        }
    });

    rsx! {
        div { class: "flex flex-col h-full bg-[#0A0A0A] rounded-xl shadow-sm border border-zinc-800/80 overflow-hidden",

            div { class: "flex justify-between items-center px-4 py-2.5 bg-zinc-900/50 border-b border-zinc-800/80 shrink-0 select-none",
                span { class: "text-[10px] font-medium text-zinc-500 tracking-widest uppercase", "Live Logs" }
                div { class: "flex items-center gap-2",
                    div { class: "w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse shadow-[0_0_8px_rgba(16,185,129,0.5)]" }
                    span { class: "text-[10px] font-semibold text-emerald-500 uppercase tracking-widest", "Stream" }
                }
            }

            div { class: "flex-1 overflow-y-auto p-4 font-mono text-[11px] leading-relaxed",
                for line in log_lines.read().iter() {
                    // Более приглушенный текст логов, подсветка строки при наведении
                    div { class: "text-zinc-400 break-all hover:bg-zinc-800/40 hover:text-zinc-200 px-1 rounded-sm border-l-2 border-transparent hover:border-zinc-600 transition-colors",
                        "{line}"
                    }
                }
                if log_lines.read().is_empty() {
                    div { class: "text-zinc-600 italic", "Ожидание потока логов..." }
                }
            }
        }
    }
}
