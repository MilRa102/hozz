use std::time::Duration;

use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{LdCheck, LdCopy, LdEye, LdEyeOff};
use tokio::time::sleep;

use crate::utils::{Icon, to_clipboard};

#[component]
pub(crate) fn SecretEntry(key_name: String, value: String) -> Element {
    let mut is_revealed = use_signal(|| false);
    let mut is_copied = use_signal(|| false);

    let mut handle_copy = move |text: String| {
        if to_clipboard(&text).is_ok() {
            is_copied.set(true);
            spawn(async move {
                sleep(Duration::from_secs(2)).await;
                is_copied.set(false);
            });
        }
    };

    rsx! {
        div { class: "py-4 flex flex-col gap-2.5 group",
            div { class: "flex items-center justify-between",
                span { class: "text-[11px] font-semibold text-zinc-400 uppercase tracking-widest", "{key_name}" }

                div { class: "flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity",
                    button {
                        class: "p-1.5 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded-md transition-colors",
                        title: "Показать значение",
                        onclick: move |e| { e.stop_propagation(); is_revealed.set(!is_revealed()); },
                        if is_revealed() { Icon { icon: LdEyeOff, size: 14 } } else { Icon { icon: LdEye, size: 14 } }
                    }
                    button {
                        class: "p-1.5 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 rounded-md transition-colors",
                        title: if is_copied() { "Скопировано!" } else { "Копировать" },
                        onclick: move |e| { e.stop_propagation(); handle_copy(value.clone()); },
                        if is_copied() {
                            Icon { icon: LdCheck, size: 14 }
                        } else {
                            Icon { icon: LdCopy, size: 14 }
                        }
                    }
                }
            }

            div { class: "bg-zinc-900/50 border border-zinc-800/80 rounded-lg p-3.5 font-mono text-sm break-all",
                if is_revealed() {
                    span { class: "text-zinc-300 selection:bg-zinc-700 selection:text-white", "{value}" }
                } else {
                    span { class: "text-zinc-600 select-none tracking-[0.2em]", "••••••••••••••••••••••••••••••••" }
                }
            }
        }
    }
}
