use std::sync::Arc;

use dioxus::{desktop::use_window, prelude::*};
use dioxus_free_icons::icons::{
    md_content_icons::MdRemove, md_navigation_icons::MdClose,
};
use shared::app::orchestrator::Orchestrator;

use crate::utils::Icon;

#[component]
pub(crate) fn TitleBar() -> Element {
    let window = use_window();
    let window_minimized = window.clone();
    let window_close = window.clone();
    // Получаем доступ к оркестратору для логики сворачивания в трей
    let arch = use_context::<Arc<Orchestrator>>();

    rsx! {
        div {
            // Линейный стиль: черный фон, очень тонкая нижняя рамка, запрет выделения текста
            class: "flex justify-end items-center w-full h-9 bg-black border-b border-white/5 select-none shrink-0",

            // Захват окна для перетаскивания (сработает везде, где не остановлено всплытие)
            onmousedown: move |_| window.drag(),

            // Правая часть: элементы управления
            div {
                class: "flex h-full",

                // Кнопка Свернуть (стандартное сворачивание в панель задач ОС)
                button {
                    class: "px-4 h-full flex items-center justify-center text-zinc-500 hover:bg-white/10 hover:text-zinc-300 transition-colors",
                    onmousedown: move |evt| evt.stop_propagation(),
                    onclick: move |_| window_minimized.set_minimized(true),
                    Icon { icon: MdRemove, size: 16 }
                }

                // Кнопка Закрыть -> Спрятать в трей
                button {
                    class: "px-4 h-full flex items-center justify-center text-zinc-500 hover:bg-rose-500 hover:text-white transition-colors",
                    onmousedown: move |evt| evt.stop_propagation(),
                    onclick: move |_| {
                        window_close.set_visible(false);
                        arch.set_active(false);
                    },
                    Icon { icon: MdClose, size: 16 }
                }
            }
        }
    }
}
