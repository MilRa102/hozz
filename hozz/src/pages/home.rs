use dioxus::prelude::*;

use crate::widgets::rule::RulesTable;

#[component]
pub fn Home() -> Element {
    rsx! {
        // Убираем собственный фон, так как Navbar уже задает bg-zinc-950
        div { class: "h-full p-6 overflow-y-auto space-y-6",

            // Компактный заголовок страницы
            div { class: "flex items-center justify-start",
                div { class: "flex items-center gap-3",
                    h1 { class: "text-2xl font-semibold text-zinc-100 tracking-tight", "Политики приложения" }
                }
            }

            // Блок политик (на всю ширину)
            RulesTable {}
        }
    }
}
