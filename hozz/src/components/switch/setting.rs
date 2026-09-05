use dioxus::prelude::*;
use dioxus_icons::lucide::{CircleQuestionMark, Recycle, RefreshCw, Settings};
use prefs::{Requirement, SettingMeta};

use crate::components::{modal::ModalDetails, switch::control::SettingControl};

#[component]
pub(crate) fn SettingRow(
    meta: SettingMeta,
    value: String,
    provider: String,
    onchange: EventHandler<String>,
) -> Element {
    let SettingMeta {
        title,
        description,
        requirements,
        ..
    } = meta.clone();

    let mut show_modal = use_signal(|| false);

    rsx! {
        div { class: "w-full flex items-center justify-between p-3 hover:bg-white/5 outline-none rounded-lg transition-colors group text-left",
            div { class: "flex flex-col pr-6",
                span { class: "flex gap-2 text-sm font-medium text-zinc-200 transition-colors",
                    if !requirements.is_empty() {
                        button {
                            class: "flex items-center gap-1.5 text-xs text-zinc-400 hover:text-zinc-200 transition-colors cursor-help",
                            onclick: move |_| show_modal.set(true),
                            CircleQuestionMark { size: "16px" }
                        }
                    }
                    "{title}"
                }
                span { class: "text-xs text-zinc-500 mt-0.5 leading-relaxed", "{description}" }
            }

            div { class: "flex flex-col items-end gap-3 shrink-0",
                SettingControl {
                    meta: meta.clone(),
                    value: value.clone(),
                    provider,
                    onchange,
                }
            }
        }

        if show_modal() {
            ModalDetails {
                title: "Изменение конфигурации",
                description: format!("Применение настройки «{}» потребует дополнительных системных действий.", title),
                apply_text: "Понятно",
                cancel_text: "Отмена",
                on_close: move |_| show_modal.set(false),
                on_apply: move |_| show_modal.set(false),

                div { class: "flex flex-col gap-3 text-sm text-zinc-400",
                    for req in requirements.iter() {
                        match req {
                            Requirement::Admin => rsx! {
                                div { class: "flex items-center gap-2",
                                    Settings { size: "16px", class: "text-green-500" }
                                    span { "Необходимо обладать правами администратора приложения для применения настройки" }
                                }
                            },
                            Requirement::CoreReload => rsx! {
                                div { class: "flex items-center gap-2",
                                    RefreshCw { size: "16px", class: "text-orange-500" }
                                    span { "Для применения настройки, ядро приложения будет перезапущено." }
                                }
                            },
                            Requirement::Restart => rsx! {
                                div { class: "flex items-center gap-2",
                                    Recycle { size: "16px", class: "text-white" }
                                    span { "Чтобы изменения вступили в силу, закройте приложение и откройте его снова." }
                                }
                            },
                        }
                    }

                }
            }
        }
    }
}
