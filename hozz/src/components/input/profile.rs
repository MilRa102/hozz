use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_free_icons::icons::md_content_icons::{MdAdd, MdContentPaste};
use shared::{
    app::orchestrator::Orchestrator,
    infra::{ProfileManager, log::LoggingLayer},
};

use crate::utils::Icon;

#[component]
pub fn AddProfile() -> Element {
    let mut input = use_signal(String::new);

    let paste_from_clipboard = move |_| {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                input.set(text);
            }
        } else {
            let arch = use_context::<Arc<Orchestrator>>();
            arch.error("Не удалось получить данные из буфера обмена");
        }
    };

    let on_apply = move |_| {
        let arch = use_context::<Arc<Orchestrator>>();
        let url = input();
        if url.is_empty() {
            arch.warning("Введите URL подписки");
            return;
        }

        spawn(async move {
            match arch.add_profile(&url).await {
                Ok(()) => {
                    arch.ok("Подписка успешно добавлена!");
                    input.set(String::new());
                },
                Err(_) => arch.error("Не удалось применить подписку"),
            }
        });
    };

    rsx! {
        div { class: "border border-white/10 rounded-lg p-1.5 shadow-sm bg-zinc-900/30",
            div { class: "flex gap-1.5",
                input {
                    // Глубокий черный фон для поля ввода
                    class: "flex-1 bg-black border border-white/5 rounded-md px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-zinc-700 transition-all",
                    placeholder: "URL подписки",
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                }

                button {
                    class: "px-3 py-2 border border-transparent rounded-md text-zinc-400 hover:bg-white/5 hover:text-zinc-200 transition-colors cursor-copy",
                    onclick: paste_from_clipboard,
                    title: "Вставить из буфера",
                    Icon { icon: MdContentPaste, size: 18 }
                }

                button {
                    class: "px-3 py-2 bg-zinc-100 hover:bg-white text-black rounded-md text-sm font-semibold transition-colors focus:outline-none active:scale-95 cursor-pointer",
                    title: "Добавить",
                    onclick: on_apply,
                    Icon { icon: MdAdd, size: 18, color: "black" }
                }
            }
        }
    }
}
