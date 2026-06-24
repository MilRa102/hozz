use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    md_action_icons::MdInfoOutline, md_navigation_icons::MdClose,
};

use crate::utils::Icon;

#[component]
pub fn ModalOverlay(
    title: String,
    children: Element,
    on_close: EventHandler<()>,
    #[props(default = String::new())] footer_text: String,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-[100] flex items-center justify-center p-4 sm:p-6",
            // Темный Backdrop
            div { class: "absolute inset-0 bg-black/60 backdrop-blur-sm", onclick: move |_| on_close.call(()) }

            // Контейнер модального окна
            div { class: "relative w-full max-w-2xl bg-zinc-950 border border-zinc-800 rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-full animate-in fade-in zoom-in-95 duration-200",

                div { class: "px-6 py-4 border-b border-zinc-800 flex justify-between items-center bg-zinc-950 shrink-0",
                    h3 { class: "text-sm font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "p-1.5 text-zinc-500 hover:text-zinc-100 hover:bg-zinc-800 rounded-md transition-colors",
                        onclick: move |_| on_close.call(()),
                        Icon { icon: MdClose, size: 18 }
                    }
                }

                div { class: "p-6 overflow-y-auto",
                    {children}
                }

                if !footer_text.is_empty() {
                    div { class: "px-6 py-3 bg-zinc-900/50 border-t border-zinc-800 shrink-0",
                        p { class: "text-[11px] text-zinc-500 flex items-center justify-center gap-1.5",
                            Icon { icon: MdInfoOutline, size: 13 }
                            "{footer_text}"
                        }
                    }
                }
            }
        }
    }
}
