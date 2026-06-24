use dioxus::prelude::*;
use dioxus_free_icons::icons::md_action_icons::MdLogout;
use shared::apps::vault::TokenInfo;

use crate::{
    components::{
        button::{ActionButton, ButtonVariant},
        card::PanelCard,
    },
    utils::Icon,
};

#[component]
pub fn VaultSessionBar(info: TokenInfo, on_logout: EventHandler<MouseEvent>) -> Element {
    rsx! {
        PanelCard { class: "flex flex-wrap items-center justify-between p-4 gap-4",
            div { class: "flex items-center gap-3",
                // Индикатор статуса с легким свечением
                div { class: "w-2 h-2 bg-emerald-500 rounded-full shadow-[0_0_8px_rgba(16,185,129,0.5)]" }
                div { class: "flex flex-col",
                    span { class: "text-sm font-medium text-zinc-200", "Соединение активно" }
                    span { class: "text-[11px] text-zinc-500 mt-0.5", "Политики: {info.policies_str()}" }
                }
            }
            div { class: "flex items-center gap-6",
                div { class: "flex flex-col items-end",
                    span { class: "text-[10px] font-semibold text-zinc-500 uppercase tracking-widest", "TTL Сессии" }
                    span { class: "text-sm font-mono text-zinc-300 mt-0.5", "{info.ttl} сек." }
                }
                ActionButton {
                    variant: ButtonVariant::Danger,
                    onclick: move |e| on_logout.call(e),
                    Icon { icon: MdLogout, size: 14 }
                    "Завершить"
                }
            }
        }
    }
}
