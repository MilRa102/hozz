use dioxus::prelude::*;
use dioxus_icons::lucide::House;

#[component]
pub fn VaultBreadcrumbs(
    selected_mount: String,
    cursor: String,
    on_reset: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 text-sm",
            button {
                class: "p-1.5 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800/50 rounded-md transition-all cursor-pointer",
                onclick: move |e| on_reset.call(e),
                House { size: "16px" }
            }
            if !selected_mount.is_empty() {
                span { class: "text-zinc-700", "/" }
                span { class: "text-zinc-200 font-medium", "{selected_mount}" }
            }
            for segment in cursor.split('/').filter(|s| !s.is_empty()) {
                span { class: "text-zinc-700", "/" }
                span { class: "text-zinc-400", "{segment}" }
            }
        }
    }
}
