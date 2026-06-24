use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_free_icons::icons::io_icons::{IoArrowDown, IoArrowUp};
use shared::{app::orchestrator::Orchestrator, infra::GroupManager};

use crate::{
    components::{
        dropdown::{DropdownList, NodeDropdownItems, ProfileDropdownItems},
        input::AddProfile,
        label::MetricLabel,
    },
    pages::app::AppState,
};

#[component]
pub fn ProxyDashboard() -> Element {
    let mut state = use_context::<AppState>();
    let wait_pending = use_signal(|| false);

    let traffic_up = state.up;
    let traffic_down = state.down;
    let connected = state.is_connected;
    let groups = (state.groups)();
    let active_ip = state.active_ip;
    let active_profile = state.active_profile;

    // Инвертированные "темные" стили для бейджей статуса
    let (status_bg, status_dot, status_text, btn_class, btn_text) = if wait_pending() {
        (
            "bg-amber-500/10 border-amber-500/20",
            "bg-amber-400 animate-pulse shadow-[0_0_8px_rgba(251,191,36,0.5)]",
            "Подключение...",
            "bg-zinc-800 text-zinc-500 cursor-not-allowed border-transparent",
            "Подождите",
        )
    } else if connected() {
        (
            "bg-emerald-500/10 border-emerald-500/20",
            "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]",
            "Прокси активен",
            "bg-rose-500/10 text-rose-400 hover:bg-rose-500/20 border-transparent",
            "Отключить",
        )
    } else {
        (
            "bg-zinc-900 border-white/5",
            "bg-zinc-600",
            "Отключено",
            "bg-zinc-100 text-black hover:bg-white border-transparent focus:ring-zinc-500 focus:ring-offset-zinc-950",
            "Подключить",
        )
    };

    rsx! {
        div { class: "p-6 h-full flex flex-col gap-6 overflow-hidden",

            // 1. HEADER: Панель управления и статуса
            div { class: "shrink-0 flex items-center justify-between p-5 rounded-xl border transition-colors duration-500 {status_bg}",
                // Левая часть: Статус и IP
                div { class: "flex items-center gap-6",
                    div { class: "flex items-center gap-3",
                        div { class: "w-2.5 h-2.5 rounded-full {status_dot}" }
                        div { class: "flex flex-col",
                            span { class: "text-sm font-semibold text-zinc-100 tracking-wide", "{status_text}" }
                            span { class: "text-xs text-zinc-400", "Профиль: {active_profile()}" }
                        }
                    }
                    div { class: "h-8 w-px bg-white/10" } // Разделитель
                    div { class: "flex flex-col",
                        span { class: "text-[10px] text-zinc-500 uppercase tracking-wider font-bold mb-0.5", "Внешний IP" }
                        span { class: "text-sm font-mono font-medium text-zinc-200", "{active_ip()}" }
                    }
                }

                // Правая часть: Трафик и Кнопка
                div { class: "flex items-center gap-8",
                    if connected() {
                        div { class: "flex gap-6",
                            MetricLabel { icon: IoArrowDown, value: traffic_up(), label: "DL" }
                            MetricLabel { icon: IoArrowUp, value: traffic_down(), label: "UL" }
                        }
                    }

                    button {
                        class: "px-6 py-2 rounded-md text-sm font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-offset-2 active:scale-95 cursor-pointer {btn_class}",
                        disabled: wait_pending(),
                        onclick: move |_| state.toggle_proxy(wait_pending),
                        "{btn_text}"
                    }
                }
            }

            // 2. BODY: Grid-лейаут
            div { class: "flex-1 grid grid-cols-1 lg:grid-cols-12 gap-6 min-h-0",

                // Левая колонка (Подписки)
                div { class: "lg:col-span-5 flex flex-col gap-4 min-h-0",
                    div { class: "shrink-0", AddProfile {} }

                    div { class: "flex-1 border border-white/10 rounded-xl flex flex-col min-h-0 overflow-hidden",
                        div { class: "px-4 py-3 border-b border-white/5 bg-zinc-900/50 shrink-0",
                            span { class: "text-xs font-semibold text-zinc-400 uppercase tracking-wider", "Профили" }
                        }
                        div { class: "flex-1 overflow-y-auto divide-y divide-white/5",
                            ProfileDropdownItems {}
                        }
                    }
                }

                // Правая колонка (Группы и узлы)
                div { class: "lg:col-span-7 flex flex-col gap-4 min-h-0",
                    div { class: "flex-1 overflow-y-auto pr-2 space-y-4",
                        {
                            groups.iter().map(|group| {
                                let group_name = group.name.clone();
                                let nodes = group.nodes.clone();
                                rsx! {
                                    DropdownList {
                                        key: "{group_name}",
                                        title: group_name.clone(),
                                        label: group.selected.clone(),
                                        open: use_signal(|| false),

                                        NodeDropdownItems {
                                            nodes,
                                            select: move |node_name: String| {
                                                let group = group_name.clone();
                                                spawn(async move {
                                                    consume_context::<Arc<Orchestrator>>()
                                                        .select_active_in_group(&group, &node_name).await;
                                                    state.select_profile(node_name);
                                                });
                                            }
                                        }
                                    }
                                }
                            })
                        }
                    }
                }
            }
        }
    }
}
