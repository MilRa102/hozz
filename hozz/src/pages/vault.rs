use std::sync::Arc;

use dioxus::prelude::*;
use shared::{
    apps::{
        LoggingLayer, Orchestrator,
        vault::{SecretManager, TokenInfo},
    },
    db::vault::VaultConfig,
};

use crate::components::panel::{VaultExplorer, VaultSignIn};

#[derive(Debug, Clone, Default, PartialEq)]
enum VaultState {
    #[default]
    Checking,
    Unauthenticated,
    Authenticated(VaultConfig, TokenInfo),
}

#[component]
pub fn VaultPage() -> Element {
    let mut state = use_signal(VaultState::default);

    use_effect(move || {
        spawn(async move {
            let arch = use_context::<Arc<Orchestrator>>();
            if let Some(cfg) = arch.vaults.fetch() {
                if let Ok(info) = arch.session().await {
                    state.set(VaultState::Authenticated(cfg, info));
                } else {
                    arch.error("Авторизация в Vault не удалась");
                    state.set(VaultState::Unauthenticated);
                }
            } else {
                state.set(VaultState::Unauthenticated);
            }
        });
    });

    rsx! {
        div { class: "h-full w-full bg-zinc-950 text-zinc-100 flex flex-col font-sans selection:bg-zinc-800",
            match state() {
                VaultState::Checking => rsx! {
                    div { class: "flex-1 flex items-center justify-center gap-3 text-zinc-400",
                        div { class: "w-5 h-5 border-2 border-zinc-800 border-t-zinc-300 rounded-full animate-spin" }
                        span { class: "text-sm font-medium", "Установка соединения с Vault..." }
                    }
                },
                VaultState::Unauthenticated => rsx! {
                    VaultSignIn {
                        on_sign: move |(cfg, info)| state.set(VaultState::Authenticated(cfg, info))
                    }
                },
                VaultState::Authenticated(cfg, info) => rsx! {
                    VaultExplorer {
                        cfg,
                        info,
                        on_logout: move |_| {
                            let orch = use_context::<Arc<Orchestrator>>();
                            let _ = orch.vaults.cleanup();
                            state.set(VaultState::Unauthenticated);
                        }
                    }
                }
            }
        }
    }
}
