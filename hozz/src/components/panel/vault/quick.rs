use dioxus::prelude::*;
use dioxus_free_icons::icons::md_action_icons::MdLockOutline;
use shared::apps::vault::SecretVisit;

use crate::utils::Icon;

#[component]
pub fn VaultQuickAccess(
    visits: Vec<SecretVisit>,
    on_jump: EventHandler<(String, String)>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3",
            span { class: "text-[10px] font-medium text-zinc-500 uppercase tracking-widest shrink-0", "Недавние" }
            div { class: "flex flex-wrap gap-2",
                for visit in visits {
                    button {
                        // Темные pill-кнопки
                        class: "px-2.5 py-1 text-[11px] bg-zinc-900/50 border border-zinc-800 text-zinc-400 rounded-full hover:bg-zinc-800 hover:text-zinc-200 hover:border-zinc-700 transition-all flex items-center gap-1.5 cursor-pointer",
                        onclick: {
                            let m = visit.mount.clone();
                            let p = visit.path.clone();
                            move |_| on_jump.call((m.clone(), p.clone()))
                        },
                        Icon { icon: MdLockOutline, size: 12, class: "opacity-70" }
                        span { class: "font-mono truncate max-w-[120px]", "{visit.path.split('/').last().unwrap_or(&visit.path)}" }
                    }
                }
            }
        }
    }
}
