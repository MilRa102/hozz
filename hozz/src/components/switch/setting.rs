use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    ld_icons::LdRecycle,
    md_action_icons::{MdAdminPanelSettings, MdAutorenew},
};
use prefs::Requirement;

use crate::utils::Icon;

#[component]
pub(crate) fn SettingRow(
    #[props(into)] title: String,
    #[props(into)] desc: String,
    is_active: bool,
    #[props(default = vec![])] requirements: Vec<Requirement>,
    ontoggle: EventHandler<bool>,
) -> Element {
    let (bg_color, knob_color, translate) = if is_active {
        ("bg-white", "bg-black", "translate-x-4")
    } else {
        ("bg-zinc-800", "bg-zinc-400", "translate-x-0")
    };

    rsx! {
        button {
            class: "w-full flex items-center justify-between p-3 hover:bg-white/5 focus:bg-white/5 outline-none cursor-pointer rounded-lg transition-colors group text-left",
            onclick: move |_| ontoggle.call(!is_active),

            div { class: "flex flex-col pr-6",
                span { class: "text-sm font-medium text-zinc-200 group-hover:text-white transition-colors", "{title}" }
                span { class: "text-xs text-zinc-500 mt-0.5 leading-relaxed", "{desc}" }
            }

            div { class: "flex items-center gap-4 shrink-0",
                if !requirements.is_empty() {
                    div { class: "flex items-center gap-2 cursor-help",
                        for req in requirements.iter() {
                            match req {
                                Requirement::Admin => rsx! {
                                    span {
                                        title: "Требуются права администратора",
                                        Icon { icon: MdAdminPanelSettings, size: 16, color: "green" }
                                    }
                                },
                                Requirement::CoreReload => rsx! {
                                    span {
                                        title: "Ядро прокси, будет автоматически перезапущено",
                                        Icon { icon: MdAutorenew, size: 16, color: "orange" }
                                    }
                                },
                                Requirement::Restart => rsx! {
                                    span {
                                        title: "Требуется ручной перезапуск приложения",
                                        Icon { icon: LdRecycle, size: 16, color: "white" }
                                    }
                                },
                            }
                        }
                    }
                }

                div { class: "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full",
                    // Трек (Фон)
                    span { class: "absolute inset-0 rounded-full transition-colors duration-200 ease-in-out {bg_color}" }
                    // Ползунок (Кружок)
                    span { class: "absolute left-[2px] h-4 w-4 transform rounded-full shadow ring-0 transition-transform duration-200 ease-in-out {knob_color} {translate}" }
                }
            }
        }
    }
}
