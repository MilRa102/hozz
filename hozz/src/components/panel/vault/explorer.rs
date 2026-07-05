use std::sync::Arc;

use dioxus::{logger::tracing, prelude::*};
use dioxus_free_icons::icons::ld_icons::LdShare2;
use serde_json::{Map, Value as Json};
use shared::apps::{
    LoggingLayer, Orchestrator,
    vault::{SecretItem, SecretManager, SecretType, TokenInfo, VaultConfig},
};

use super::{
    breadcrumb::VaultBreadcrumbs, dirs::VaultDirectoryList, quick::VaultQuickAccess,
    session::VaultSessionBar,
};
use crate::{
    components::{
        card::PanelCard, input::SearchInput, item::SecretEntry, modal::ModalOverlay,
    },
    utils::{Icon, to_clipboard},
};

#[component]
pub fn VaultExplorer(
    cfg: VaultConfig,
    info: TokenInfo,
    on_logout: EventHandler<MouseEvent>,
) -> Element {
    let arch = use_context::<Arc<Orchestrator>>();

    // --- State ---
    let mut selected_mount = use_signal(String::new);
    let mut cursor = use_signal(String::new);
    let mut active_secret = use_signal(|| None::<Map<String, Json>>);
    let mut active_meta = use_signal(|| None::<Map<String, Json>>);
    let mut is_loading_secret = use_signal(|| false);
    let mut search_query = use_signal(String::new);
    let mut frequent_refresh = use_signal(|| 0);

    // --- Resources ---
    let frequent_secrets = use_resource(move || {
        let _ = frequent_refresh();
        async move { consume_context::<Arc<Orchestrator>>().frequent_visits() }
    });

    let resources = use_resource(move || {
        let mount = selected_mount();
        let path = cursor();
        let arch = consume_context::<Arc<Orchestrator>>();
        async move {
            if mount.is_empty() {
                arch.mounts().await.map(|m| {
                    m.into_iter()
                        .map(SecretItem::new_folder)
                        .collect()
                })
            } else {
                arch.secrets(&mount, &path).await
            }
        }
    });

    // --- Actions ---
    let back_evt = move |_| {
        let path = cursor();
        if cursor.is_empty() {
            selected_mount.set(String::new());
        } else if let Some(parent) = path.rsplitn(2, '/').last() {
            cursor.set(if path.contains('/') {
                parent.to_string()
            } else {
                String::new()
            });
        }
    };

    let mut jump_to_secret = move |mount: String, full_path: String| {
        selected_mount.set(mount.clone());

        let mut parts: Vec<&str> = full_path.split('/').collect();
        let _name = parts.pop().unwrap_or("");
        cursor.set(parts.join("/"));
        search_query.set(String::new());

        spawn(async move {
            is_loading_secret.set(true);
            let arch = consume_context::<Arc<Orchestrator>>();
            if let Ok(data) = arch.secret(&mount, &full_path).await {
                active_secret.set(Some(data));
            }
            is_loading_secret.set(false);
        });
    };

    // Подготавливаем данные для списка
    let (items, is_loading, error) = match &*resources.value().read() {
        Some(Ok(items)) => {
            let query = search_query().to_lowercase();
            let filtered: Vec<SecretItem> = items
                .iter()
                .filter(|i| i.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
            (filtered, false, None)
        },
        Some(Err(e)) => (vec![], false, Some(e.to_string())),
        None => (vec![], true, None),
    };

    let secret_share = move || {
        let arch = consume_context::<Arc<Orchestrator>>();
        let mount = selected_mount();
        let path = cursor();

        spawn(async move {
            match arch.secret_wrap(&mount, &path).await {
                Ok(wrapped) => {
                    let token = wrapped.info.token;
                    match to_clipboard(&token) {
                        Ok(_) => arch.ok("Секрет скопирован в буфер обмена."),
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to copy secret token to clipboard")
                        },
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "Failed to wrap secret for sharing");
                    arch.error("Не удалось получить токен доступа к секрету.");
                },
            }
        });
    };

    // --- View ---
    rsx! {
        div { class: "h-full flex flex-col p-6 gap-4",

            // 1. Панель сессии
            VaultSessionBar { info: info.clone(), on_logout: move |e| on_logout.call(e) }

            // 2. Блок быстрого доступа
            if let Some(visits) = frequent_secrets.value().read().as_ref() {
                if !visits.is_empty() {
                    VaultQuickAccess { visits: visits.clone(), on_jump: move |(m, p)| jump_to_secret(m, p) }
                }
            }

            // 3. Основной проводник (PanelCard)
            PanelCard { class: "flex-1 flex flex-col min-h-0 overflow-hidden",
                // Тулбар
                div { class: "flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-4 border-b border-white/10 bg-black/20 rounded-t-lg shrink-0",
                    VaultBreadcrumbs {
                        selected_mount: selected_mount(),
                        cursor: cursor(),
                        on_reset: move |_| {
                            selected_mount.set(String::new());
                            cursor.set(String::new());
                        }
                    }
                    SearchInput {
                        signal: search_query,
                        placeholder: if selected_mount().is_empty() { "Поиск точки монтирования...".to_string() } else { "Поиск секрета...".to_string() }
                    }
                }

                // Листинг
                div { class: "flex-1 overflow-hidden",
                    VaultDirectoryList {
                        items, is_loading, error,
                        is_mount_level: selected_mount().is_empty(),
                        has_cursor: !cursor().is_empty(),
                        on_back: back_evt,
                        on_item_click: move |item: SecretItem| {
                            search_query.set(String::new());
                            let orch = arch.clone();
                            if selected_mount().is_empty() {
                                selected_mount.set(item.path.clone());
                            } else if item.secret_type == SecretType::Folder {
                                let new_path = if cursor().is_empty() { item.name.clone() } else { format!("{}/{}", cursor(), item.name) };
                                cursor.set(new_path);
                            } else {
                                let mount = selected_mount().clone();
                                let path = format!("{}/{}", cursor(), item.name);
                                spawn(async move {
                                    is_loading_secret.set(true);
                                    orch.track_visit(&mount, &path);
                                    frequent_refresh.with_mut(|v| *v += 1);

                                    let secret_fut = orch.secret(&mount, &path);
                                    let meta_fut = orch.secret_meta(&mount, &path);

                                    let (secret_res, meta_res) = tokio::join!(secret_fut, meta_fut);

                                    if let Ok(data) = secret_res {
                                        active_secret.set(Some(data));
                                    }
                                    if let Ok(meta) = meta_res {
                                        active_meta.set(Some(meta));
                                    }

                                    is_loading_secret.set(false);
                                });
                            }
                        }
                    }
                }
            }
        }

        // 4. Модальное окно просмотра секрета
        if let Some(data) = active_secret() {
            ModalOverlay {
                title: "Свойства секрета".to_string(),
                footer_text: "Значения зашифрованы и изолированы текущей сессией".to_string(),
                on_close: move |()| {
                    active_secret.set(None);
                    active_meta.set(None);
                },
                share_button: rsx! {
                    button {
                        class: "p-1.5 text-zinc-500 hover:text-zinc-100 hover:bg-zinc-800 rounded-md transition-colors cursor-copy",
                        onclick: move |_| secret_share(),
                        title: "Поделиться секретом",
                        Icon { icon: LdShare2, size: 14 }
                    }
                },

                div {
                    if let Some(meta) = active_meta()
                        && let Some(Json::Object(custom_meta)) = meta.get("custom_metadata")
                        && !custom_meta.is_empty() {
                            div { class: "text-[10px] font-semibold text-zinc-500 uppercase tracking-wider mb-3", "Метаданные секрета" }
                            div { class: "flex flex-wrap gap-2.5",
                                for (key, val) in custom_meta.iter() {
                                    div {
                                        class: "flex items-stretch rounded border border-zinc-700/50 overflow-hidden text-xs font-mono max-w-full",

                                        div { class: "flex items-center px-2 py-1 bg-black text-zinc-500 font-semibold shrink-0 border-r border-zinc-700/50",
                                            "{key}"
                                        }

                                        div {
                                            class: "flex items-center px-2 py-1 bg-zinc-800/40 text-zinc-300 truncate cursor-default hover:bg-zinc-800/60 transition-colors",
                                            title: "{val.as_str().unwrap_or_default()}",
                                            "{val.as_str().unwrap_or_default()}"
                                        }
                                    }
                                }
                            }
                    }

                    div { class: "divide-y divide-white/5 max-h-[55vh] overflow-y-auto pr-1 mt-5",
                        for (key_name, v) in data {
                            SecretEntry {
                                key_name,
                                value: v.as_str().unwrap_or("binary data").to_string()
                            }
                        }
                    }
                }
            }
        }
    }
}
