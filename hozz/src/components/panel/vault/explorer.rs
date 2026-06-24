use std::sync::Arc;

use dioxus::prelude::*;
use serde_json::{Map, Value as Json};
use shared::{
    app::orchestrator::Orchestrator,
    apps::vault::{SecretItem, SecretType, TokenInfo},
    db::vault::VaultConfig,
    infra::SecretManager,
};

use super::{
    breadcrumb::VaultBreadcrumbs, dirs::VaultDirectoryList, quick::VaultQuickAccess,
    session::VaultSessionBar,
};
use crate::components::{
    card::PanelCard, input::SearchInput, item::SecretEntry, modal::ModalOverlay,
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
                                    if let Ok(data) = orch.secret(&mount, &path).await {
                                        active_secret.set(Some(data));
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
                on_close: move |()| active_secret.set(None),

                div { class: "divide-y divide-white/5",
                    for (k, v) in data {
                        SecretEntry { key_name: k, value: v.as_str().unwrap_or("binary data").to_string() }
                    }
                }
            }
        }
    }
}
