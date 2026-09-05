use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowLeft, Database, FolderClosed, Key};
use shared::apps::vault::{SecretItem, SecretType};

#[derive(Props, Clone, PartialEq)]
pub struct VaultDirectoryListProps {
    items: Vec<SecretItem>,
    is_loading: bool,
    error: Option<String>,
    is_mount_level: bool,
    has_cursor: bool,
    on_item_click: EventHandler<SecretItem>,
    on_back: EventHandler<MouseEvent>,
}

#[component]
pub fn VaultDirectoryList(props: VaultDirectoryListProps) -> Element {
    rsx! {
        div { class: "h-full overflow-y-auto divide-y divide-zinc-800/50",
            if props.has_cursor {
                div {
                    class: "flex items-center px-4 py-2.5 text-zinc-500 hover:bg-zinc-800/30 cursor-pointer transition-colors",
                    onclick: move |e| props.on_back.call(e),
                    ArrowLeft { size: "14px", class: "text-zinc-500" }
                    span { class: "ml-3 font-mono text-sm", ".." }
                }
            }

            if props.is_loading {
                div { class: "flex items-center gap-3 p-4 text-zinc-500 text-sm",
                    div { class: "w-4 h-4 border-2 border-zinc-800 border-t-zinc-400 rounded-full animate-spin" }
                    "Загрузка данных..."
                }
            } else if let Some(err) = &props.error {
                div { class: "p-4 text-red-400 bg-red-950/30 border-b border-red-900/50",
                    p { class: "text-sm font-medium", "Ошибка доступа" }
                    p { class: "text-xs mt-1 opacity-80", "{err}" }
                }
            } else if props.items.is_empty() {
                div { class: "p-4 text-zinc-500 text-sm", "Директория пуста" }
            } else {
                for item in props.items {
                    div {
                        class: "flex items-center px-4 py-2.5 hover:bg-zinc-800/30 cursor-pointer transition-colors group",
                        onclick: {
                            let item_clone = item.clone();
                            move |_| props.on_item_click.call(item_clone.clone())
                        },
                        div { class: "w-6 flex justify-center shrink-0",
                            if props.is_mount_level {
                                Database { size: "15px", class: "text-zinc-500 group-hover:text-zinc-300 transition-colors" }
                            } else if item.secret_type == SecretType::Folder {
                                FolderClosed { size: "15px", class: "text-zinc-500 group-hover:text-zinc-300 transition-colors" }
                            } else {
                                Key { size: "15px", class: "text-zinc-600 group-hover:text-zinc-400 transition-colors" }
                            }
                        }
                        span { class: "ml-3 text-sm text-zinc-300 group-hover:text-zinc-100 transition-colors", "{item.name}" }
                    }
                }
            }
        }
    }
}
