use dioxus::prelude::*;
use dioxus_free_icons::icons::md_navigation_icons::MdArrowDropDown;

use crate::utils::Icon;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct DropdownProps {
    #[props(default)]
    pub title: String,
    #[props(default)]
    pub label: String,
    #[props(default)]
    pub open: Signal<bool>,
    pub children: Element,
}

#[component]
pub(crate) fn DropdownList(props: DropdownProps) -> Element {
    let mut opened = props.open;
    let transform = if opened() { "rotate-180" } else { "" };
    let label = if props.label.is_empty() {
        String::new()
    } else {
        format!("—\u{00A0}{}", props.label)
    };

    rsx! {
        div { class: "border border-white/10 rounded-xl overflow-hidden shadow-sm",
            // Заголовок аккордеона
            div {
                class: "flex justify-between items-center px-4 py-3 bg-zinc-900/50 hover:bg-white/5 cursor-pointer transition-colors select-none",
                onclick: move |_| opened.set(!opened()),
                div { class: "flex items-center gap-2",
                    span { class: "font-semibold text-sm text-zinc-200", "{props.title}" }
                    span { class: "text-xs text-zinc-500 font-medium", "{label}" }
                }
                div { class: "text-zinc-500 transition-transform duration-200 {transform}",
                    Icon { icon: MdArrowDropDown, size: 20 }
                }
            }

            if opened() {
                div { class: "divide-y divide-white/5 border-t border-white/5",
                    {props.children}
                }
            }
        }
    }
}
