use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    md_action_icons::{MdLanguage, MdLock, MdSettings}, md_communication_icons::MdChat, md_content_icons::MdInventory, md_hardware_icons::{MdMemory, MdSecurity}, md_navigation_icons::MdMenu,
};
use shared::apps::{
    Orchestrator, PrefsManager,
    prefs::{
        ChatCapability, ContainerCapability, ResourceCapability,
        VaultCapability,
    },
};

use crate::{route::Route, utils::Icon};

#[component]
pub fn Navbar() -> Element {
    let arch = use_context::<Arc<Orchestrator>>();

    let version = env!("CARGO_PKG_VERSION");
    let is_admin = Orchestrator::is_admin();
    let mut is_expanded = use_signal(|| false);

    let sidebar_width = if is_expanded() { "w-64" } else { "w-[72px]" };

    rsx! {
        div { class: "flex h-screen w-full bg-black text-zinc-300 font-sans overflow-hidden",
            // 1. ЛЕВЫЙ САЙДБАР
            aside { class: "{sidebar_width} flex flex-col shrink-0 transition-all duration-300 ease-in-out",
                // Логотип / Заголовок
                div { class: "h-14 flex items-center px-4 gap-3 shrink-0 overflow-hidden",
                    div { class: "w-8 h-8 rounded-md bg-white text-black flex items-center justify-center font-bold text-xs shadow-sm shrink-0 cursor-default",
                        "H"
                    }
                    if is_expanded() {
                        span { class: "font-semibold text-zinc-100 tracking-wide whitespace-nowrap animate-in fade-in duration-300", "Hozz" }
                    }
                }

                // Списки навигации (сгруппированные)
                nav { class: "flex-1 overflow-y-auto overflow-x-hidden py-4 px-3 space-y-1",

                    // ГРУППА: СЕТЬ
                    if is_expanded() {
                        div { class: "text-[10px] font-bold text-zinc-500 mt-2 mb-2 px-2 uppercase tracking-wider whitespace-nowrap", "Сеть" }
                    } else {
                        div { class: "h-px bg-white/10 my-4 mx-2" }
                    }
                    NavItem { to: Route::ProxyDashboard {}, icon: rsx!(Icon { icon: MdLanguage }), label: "Прокси", is_expanded: is_expanded() }
                    NavItem { to: Route::Home {}, icon: rsx!(Icon { icon: MdSecurity }), label: "Политики", is_expanded: is_expanded() }
                    if is_admin || arch.preference_is_active::<ChatCapability>() {
                        NavItem {
                            to: Route::ChatPage {},
                            icon: rsx!(Icon { icon: MdChat }),
                            label: "Чат",
                            is_expanded: is_expanded(),
                        }
                    }

                    // ГРУППА: ИНФРАСТРУКТУРА
                    if is_admin || arch.preference_is_active::<VaultCapability>() || arch.preference_is_active::<ContainerCapability>() {
                        if is_expanded() {
                            div { class: "text-[10px] font-bold text-zinc-500 mt-2 mb-2 px-2 uppercase tracking-wider whitespace-nowrap", "Инфраструктура" }
                        } else {
                            div { class: "h-px bg-white/10 my-4 mx-2" }
                        }
                    }
                    if is_admin || arch.preference_is_active::<VaultCapability>() {
                        NavItem { to: Route::VaultPage {}, icon: rsx!(Icon { icon: MdLock }), label: "Хранилище", is_expanded: is_expanded() }
                    }
                    if is_admin || arch.preference_is_active::<ContainerCapability>() {
                        NavItem { to: Route::DockerContainers {}, icon: rsx!(Icon { icon: MdInventory }), label: "Контейнеры", is_expanded: is_expanded() }
                    }

                    // ГРУППА: СИСТЕМА
                    if is_admin || arch.preference_is_active::<ResourceCapability>() {
                        if is_expanded() {
                            div { class: "text-[10px] font-bold text-zinc-500 mt-2 mb-2 px-2 uppercase tracking-wider whitespace-nowrap", "Система" }
                        } else {
                            div { class: "h-px bg-white/10 my-4 mx-2" }
                        }
                        NavItem { to: Route::SystemResources {}, icon: rsx!(Icon { icon: MdMemory }), label: "Ресурсы", is_expanded: is_expanded() }
                    }

                    // ГРУППА: ПРОДВИНУТЫЕ
                    if is_expanded() {
                        div { class: "text-[10px] font-bold text-zinc-500 mt-2 mb-2 px-2 uppercase tracking-wider whitespace-nowrap", "Продвинутые" }
                    } else {
                        div { class: "h-px bg-white/10 my-4 mx-2" }
                    }
                    NavItem { to: Route::SettingsView {}, icon: rsx!(Icon { icon: MdSettings }), label: "Настройки", is_expanded: is_expanded() }
                }
            }

            // 2. ПРАВАЯ РАБОЧАЯ ОБЛАСТЬ
            div { class: "flex-1 flex flex-col min-w-0",

                // Шапка
                header { class: "h-14 flex items-center justify-between px-4 shrink-0",
                    div { class: "flex items-center gap-2 text-sm",
                        // Кнопка сворачивания сайдбара
                        button {
                            class: "p-2 hover:bg-white/10 rounded-md transition-colors text-zinc-500 hover:text-zinc-300 cursor-pointer",
                            onclick: move |_| is_expanded.set(!is_expanded()),
                            title: if is_expanded() { "Свернуть меню" } else { "Развернуть меню" },
                            Icon { icon: MdMenu, size: 20 }
                        }
                        span { class: "font-medium text-zinc-400 hidden sm:block cursor-default", if is_expanded() { "Свернуть меню" } else { "Развернуть меню" } }
                    }

                    div { class: "flex items-center gap-3",
                        span {
                            class: "px-2.5 py-0.5 rounded-full text-[10px] font-bold tracking-wide uppercase border bg-white/5 shadow-sm text-zinc-400 border-white/10 cursor-help",
                            title: if is_admin { "Вам доступны дополнительные возможности. Версия: {version}" } else { "Версия: {version}" },
                            if is_admin { "Администратор" } else { "Пользователь" }
                        }
                        span { class: "px-2.5 py-0.5 rounded-full text-[10px] font-medium tracking-wide uppercase border bg-zinc-950 shadow-sm text-zinc-500 border-white/10 cursor-help",
                               title: "Бета: почти готово, но ещё дышит. Версия для смелых пользователей. Баги уже не страшные, но могут удивить. Идеально, чтобы попробовать новое первым и сказать своё слово.",
                            "Beta"
                        }
                    }
                }

                // Рабочая область (Canvas)
                main { class: "flex-1 overflow-hidden pb-4 pr-4 pl-2",
                    div { class: "w-full h-full bg-zinc-950 border border-white/10 rounded-xl overflow-hidden flex flex-col relative",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}

// Обновленный элемент меню
#[component]
fn NavItem(to: Route, icon: Element, label: String, is_expanded: bool) -> Element {
    let route: Route = use_route();
    let is_active = route == to;

    let (active_class, active_color) = if is_active {
        (
            "bg-zinc-800/50 text-zinc-50 border border-white/5 font-medium",
            "text-zinc-50",
        )
    } else {
        (
            "border-transparent text-zinc-400 hover:bg-white/5 hover:text-zinc-200",
            "text-zinc-500",
        )
    };

    // Если сайдбар свернут, убираем боковые отступы (px-3) и центрируем (justify-center)
    let layout_class = if is_expanded {
        "px-3 justify-start"
    } else {
        "justify-center"
    };

    rsx! {
        Link {
            to,
            // Native tooltip — критически важен, когда меню свернуто!
            title: "{label}",
            class: "flex items-center gap-3 py-2.5 rounded-md text-sm transition-all duration-300 overflow-hidden {layout_class} {active_class}",

            div { class: "shrink-0 flex items-center justify-center {active_color}",
                {icon}
            }

            if is_expanded {
                span { class: "whitespace-nowrap", "{label}" }
            }
        }
    }
}
