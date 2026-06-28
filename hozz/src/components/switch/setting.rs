use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    ld_icons::LdRecycle,
    md_action_icons::{MdAdminPanelSettings, MdAutorenew},
};
use prefs::{Requirement, SettingMeta};

use crate::components::switch::control::SettingControl;
use crate::utils::Icon;

#[component]
pub(crate) fn SettingRow(
    meta: SettingMeta,
    value: String,
    onchange: EventHandler<String>,
) -> Element {
    let SettingMeta {
        title,
        description,
        requirements,
        ..
    } = meta.clone();

    rsx! {
        div { class: "w-full flex items-center justify-between p-3 hover:bg-white/5 outline-none rounded-lg transition-colors group text-left",
            div { class: "flex flex-col pr-6",
                span { class: "text-sm font-medium text-zinc-200 transition-colors", "{title}" }
                span { class: "text-xs text-zinc-500 mt-0.5 leading-relaxed", "{description}" }
            }

            div { class: "flex flex-col items-end gap-3 shrink-0",
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

                SettingControl {
                    meta: meta.clone(),
                    value: value.clone(),
                    onchange,
                }
            }
        }
    }
}
