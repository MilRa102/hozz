use dioxus::prelude::*;
use shared::apps::docker;

use crate::{components::pet::ZeroEmpty, widgets::docker::ContainerWidget};

#[component]
pub fn DockerContainers() -> Element {
    let mut containers =
        use_resource(move || async move { docker::Container::list().await });

    rsx! {
        // Заменили bg-zinc-900 на более глубокий bg-zinc-950
        div { class: "h-full bg-zinc-950 p-6 overflow-y-auto flex flex-col gap-6 text-zinc-100",

            // Заголовок страницы
            div { class: "flex items-center justify-between shrink-0",
                div { class: "flex flex-col gap-1.5",
                    h1 { class: "text-2xl font-semibold text-zinc-50 tracking-tight", "Контейнеры" }
                    p { class: "text-sm text-zinc-400", "Управление локальными Docker сервисами" }
                }

                // Обновленный бейдж счетчика: темный фон, тонкая граница
                if let Some(list) = &*containers.read() {
                    span { class: "px-3 py-1 rounded-full text-xs font-medium bg-zinc-900/50 border border-zinc-800 text-zinc-400 shadow-sm",
                        "Всего: {list.len()}"
                    }
                }
            }

            // Сетка контейнеров
            div { class: "grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-4 shrink-0",
                if let Some(list) = &*containers.read() {
                    if list.is_empty() {
                        ZeroEmpty {
                            title: "Пока здесь пусто",
                            description: "Ваше окружение не содержит активных контейнеров"
                        }
                    } else {
                        for container in list {
                            ContainerWidget {
                                key: "{container.id}",
                                container: container.clone(),
                                on_action: move || containers.restart(),
                            }
                        }
                    }
                } else {
                    div { class: "col-span-full flex justify-center py-12",
                        span { class: "text-sm text-zinc-500 flex items-center gap-3",
                            div { class: "w-4 h-4 border-2 border-zinc-800 border-t-zinc-400 rounded-full animate-spin" }
                            "Опрос Docker демона..."
                        }
                    }
                }
            }
        }
    }
}
