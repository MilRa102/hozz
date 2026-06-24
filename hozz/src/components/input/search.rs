use dioxus::prelude::*;
use dioxus_free_icons::icons::{ld_icons::LdSearch, md_navigation_icons::MdClose};

use crate::utils::Icon;

#[component]
pub fn SearchInput(
    #[props(default = "sm:w-72")] class: &'static str,
    mut signal: Signal<String>,
    placeholder: String,
) -> Element {
    rsx! {
        div { class: "relative w-full shrink-0 group {class}",
            div { class: "absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-zinc-500 group-focus-within:text-zinc-300 transition-colors",
                Icon { icon: LdSearch, size: 14 }
            }
            input {
                class: "w-full bg-black border border-white/10 text-zinc-100 text-sm rounded-md pl-9 pr-8 py-1.5 focus:outline-none focus:ring-1 focus:ring-zinc-600 focus:border-zinc-600 placeholder-zinc-600 transition-colors",
                placeholder: "{placeholder}",
                value: "{signal}",
                oninput: move |e| signal.set(e.value())
            }
            if !signal().is_empty() {
                button {
                    class: "absolute inset-y-0 right-0 pr-2.5 flex items-center text-zinc-500 hover:text-zinc-300 cursor-pointer transition-colors",
                    onclick: move |_| signal.set(String::new()),
                    Icon { icon: MdClose, size: 14 }
                }
            }
        }
    }
}
