use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_free_icons::icons::md_action_icons::MdLock;
use shared::{
    apps::{
        LoggingLayer, Orchestrator,
        vault::{SecretManager, TokenInfo},
    },
    db::vault::VaultConfig,
};

use crate::{components::pet::ZeroError, utils::Icon};

#[component]
pub fn VaultSignIn(on_sign: EventHandler<(VaultConfig, TokenInfo)>) -> Element {
    let mut url = use_signal(String::new);
    let mut token = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut has_error = use_signal(|| false);

    let handle_submit = move |evt: FormEvent| {
        evt.stop_propagation();
        spawn(async move {
            is_loading.set(true);
            has_error.set(false);

            let arch = use_context::<Arc<Orchestrator>>();
            let cfg = VaultConfig::new(url(), token());
            let _ = arch.vaults.update(&cfg);

            if let Ok(info) = arch.session().await {
                if arch.vaults.update(&cfg).is_err() {
                    arch.error("Не удалось сохранить конфигурацию Vault");
                    is_loading.set(false);
                    has_error.set(true);
                } else {
                    on_sign.call((cfg, info));
                }
            } else {
                is_loading.set(false);
                has_error.set(true);
                let _ = arch.vaults.cleanup();
            }
        });
    };

    let input_borders = if has_error() {
        "border-red-900/50 focus:border-red-500 focus:ring-red-500/30"
    } else {
        "border-zinc-800 focus:border-zinc-600 focus:ring-zinc-600"
    };

    rsx! {
        div { class: "flex-1 flex items-center justify-center p-6 animate-in fade-in slide-in-from-bottom-4 duration-500",
            div { class: "max-w-md w-full bg-zinc-900/30 backdrop-blur-md border border-zinc-800/80 shadow-2xl p-8 rounded-2xl",

                div { class: "flex flex-col items-center mb-8 h-32 justify-end",
                    if has_error() {
                            ZeroError {
                                title: "Доступ запрещен",
                                description: "Не пускают? Проверьте адрес и токен."
                            }
                    } else {
                        div { class: "w-12 h-12 bg-zinc-900 border border-zinc-800 rounded-xl flex items-center justify-center mb-5 shadow-sm transition-all",
                            Icon { icon: MdLock, size: 22, class: "text-zinc-100" }
                        }
                        h2 { class: "text-xl font-semibold text-zinc-50 tracking-tight", "Вход в хранилище" }
                        p { class: "text-sm text-zinc-400 mt-1.5 text-center", "Подключитесь к вашему HashiCorp Vault" }
                    }
                }

                form { onsubmit: handle_submit, class: "flex flex-col gap-5",
                    div { class: "flex flex-col gap-2",
                        label { class: "text-[11px] font-semibold text-zinc-400 uppercase tracking-wider", "Адрес сервера" }
                        input {
                            class: "w-full py-2.5 px-3 bg-zinc-950/50 border border-zinc-800 {input_borders} rounded-lg text-sm text-zinc-100 focus:outline-none focus:border-zinc-600 focus:ring-1 focus:ring-zinc-600 transition-all placeholder-zinc-600 disabled:opacity-50",
                            placeholder: "https://vault.example.com",
                            r#type: "url",
                            value: "{url}",
                            oninput: move |e| {
                                url.set(e.value().clone());
                                has_error.set(false);
                            },
                            disabled: is_loading(),
                        }
                    }

                    div { class: "flex flex-col gap-2",
                        label { class: "text-[11px] font-semibold text-zinc-400 uppercase tracking-wider", "Токен доступа" }
                        input {
                            class: "w-full py-2.5 px-3 bg-zinc-950/50 border border-zinc-800 {input_borders} rounded-lg text-sm font-mono text-zinc-100 focus:outline-none focus:border-zinc-600 focus:ring-1 focus:ring-zinc-600 transition-all placeholder-zinc-600 disabled:opacity-50",
                            placeholder: "hvs.xxxxxxxxxxxxxxxxx",
                            r#type: "password",
                            value: "{token}",
                            oninput: move |e| {
                                token.set(e.value().clone());
                                has_error.set(false);
                            },
                            disabled: is_loading(),
                        }
                    }

                    button {
                        // Главная кнопка в стиле Linear/Vercel часто инвертирована (белая на темном фоне)
                        class: "w-full py-2.5 mt-2 rounded-lg font-medium text-sm transition-all text-zinc-900 bg-zinc-100 hover:bg-white cursor-pointer disabled:opacity-70 disabled:cursor-not-allowed flex justify-center items-center gap-2 shadow-[0_0_15px_rgba(255,255,255,0.1)]",
                        r#type: "submit",
                        disabled: is_loading(),
                        if is_loading() {
                            div { class: "w-4 h-4 border-2 border-zinc-900/30 border-t-zinc-900 rounded-full animate-spin" }
                            "Подключение..."
                        } else {
                            "Авторизоваться"
                        }
                    }
                }
            }
        }
    }
}
