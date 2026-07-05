use dioxus::prelude::*;
use dioxus_free_icons::icons::md_navigation_icons::MdClose;

use crate::utils::Icon;

#[derive(Props, Clone, PartialEq)]
pub struct ModalProps {
    /// Modal window title
    pub title: String,

    /// Optional description below the heading
    #[props(default)]
    pub description: Option<String>,

    /// The content to be included (text, inputs, charts)
    pub children: Element,

    /// The handler for closing the modal (required)
    pub on_close: EventHandler<()>,

    /// The handler for applying the action (optional)
    #[props(default)]
    pub on_apply: Option<EventHandler<()>>,

    /// The text for the action button (default is "Apply")
    #[props(default = "Apply".to_string())]
    pub apply_text: String,

    /// The text for the cancel button (default is "Cancel")
    #[props(default = "Cancel".to_string())]
    pub cancel_text: String,
}

#[component]
pub fn ModalDetails(props: ModalProps) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/60 backdrop-blur-sm",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "w-full max-w-xl bg-zinc-950 border border-zinc-800 rounded-xl shadow-2xl flex flex-col cursor-default",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "flex items-start justify-between px-6 py-5 border-b border-zinc-800/50",
                    div { class: "flex flex-col gap-1",
                        h3 { class: "text-lg font-medium text-zinc-100", "{props.title}" }
                        if let Some(desc) = &props.description {
                            p { class: "text-sm text-zinc-400", "{desc}" }
                        }
                    }
                    button {
                        class: "shrink-0 p-1 text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50 rounded-md transition-colors focus:outline-none cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        Icon { icon: MdClose, size: 20 }
                    }
                }

                div { class: "px-6 py-5 text-zinc-300 text-sm leading-relaxed",
                    {props.children}
                }

                div { class: "flex items-center justify-end gap-3 px-6 py-4 bg-zinc-900/30 border-t border-zinc-800/50 rounded-b-xl",

                    button {
                        class: "px-4 py-2 text-sm font-medium text-zinc-400 hover:text-zinc-200 transition-colors focus:outline-none cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        "{props.cancel_text}"
                    }

                    if let Some(on_apply) = props.on_apply {
                        button {
                            class: "px-4 py-2 text-sm font-medium bg-zinc-100 text-zinc-950 hover:bg-white rounded-lg shadow-sm transition-colors focus:outline-none cursor-pointer",
                            onclick: move |_| on_apply.call(()),
                            "{props.apply_text}"
                        }
                    }
                }
            }
        }
    }
}
