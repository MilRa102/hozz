use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    io_icons::IoFlash,
    md_action_icons::MdDelete,
    md_notification_icons::{MdPower, MdPowerOff, MdSync},
};
use shared::app::profile::{Profile, Source};

use crate::utils::Icon;

#[component]
pub(crate) fn ProfileItem(
    profile: Profile,
    is_updating: bool,
    onupdate: EventHandler<String>,
    onhealth: EventHandler<String>,
    ondelete: EventHandler<String>,
    ontoggle: EventHandler<String>,
) -> Element {
    let profile_update_id = profile.id.clone();
    let profile_health_id = profile.id.clone();
    let profile_delete_id = profile.id.clone();
    let profile_toggle_id = profile.id.clone();

    let Source::Remote(url) = &profile.source;
    let url_without_proto = url.replace("https://", "").replace("http://", "");
    let domain = url_without_proto
        .split('/')
        .next()
        .unwrap_or(&url_without_proto)
        .to_string();

    let (updating_animate, updating_opacity) = if is_updating {
        ("animate_pulse", "opacity-100 translate-x-0")
    } else {
        ("", "opacity-0 group-hover:opacity-100")
    };

    let is_enabled = profile.enabled;
    let container_state_style: &str = if is_enabled {
        "opacity-100"
    } else {
        "opacity-50 grayscale-[40%] transition-all duration-300"
    };

    rsx! {
        div { class: "group flex items-center justify-between px-4 py-3 hover:bg-white/[0.02] transition-colors duration-200 cursor-default {container_state_style}",

            div { class: "flex items-center gap-3 {updating_animate}",
                if !is_enabled {
                    Icon { icon: MdPowerOff, size: 18 }
                }
                div { class: "flex flex-col",
                    div { class: "flex items-center gap-2",
                        span {
                            class: "text-sm transition-colors",
                            class: if is_enabled { "text-zinc-200 font-medium" } else { "text-zinc-500 font-normal" },
                            "{domain}"
                        }
                    }
                    span { class: "text-xs text-zinc-500 truncate max-w-[280px]", "{profile.id}" }
                }
            }

            div { class: "flex items-center gap-0.5 transition-all duration-200 translate-x-2 group-hover:translate-x-0 {updating_opacity}",

                button {
                    class: "p-1.5 rounded-md text-zinc-200 hover:text-zinc-200 hover:bg-white/10 transition-colors cursor-pointer",
                    class: if is_updating { "opacity-50 cursor-not-allowed hover:bg-transparent hover:text-zinc-500" } else { "" },
                    disabled: is_updating,
                    title: "Обновить профиль",
                    onclick: move |_| onupdate.call(profile_update_id.clone()),

                    if is_updating {
                        div { class: "w-5 h-5 border-2 border-zinc-800 border-t-zinc-300 rounded-full animate-spin" }
                    } else {
                        Icon { icon: MdSync, size: 16 }
                    }
                }

                button {
                    class: "p-1.5 rounded-md text-zinc-500 hover:text-amber-400 hover:bg-amber-400/10 transition-colors cursor-pointer",
                    class: if is_updating { "opacity-50 cursor-not-allowed hover:bg-transparent hover:text-zinc-500" } else { "" },
                    disabled: is_updating,
                    title: "Проверить задержку",
                    onclick: move |_| onhealth.call(profile_health_id.clone()),

                    if is_updating {
                        div { class: "w-5 h-5 border-2 border-zinc-800 border-t-zinc-300 rounded-full animate-spin" }
                    } else {
                        Icon { icon: IoFlash, size: 16 }
                    }
                }

                div { class: "w-[1px] h-4 bg-white/10 mx-1.5" }

                button {
                    class: "p-1.5 rounded-md text-zinc-200 hover:text-zinc-200 hover:bg-blue-400/10 transition-colors cursor-pointer",
                    class: if is_updating { "opacity-50 cursor-not-allowed hover:bg-transparent hover:text-zinc-500" } else { "" },
                    title: if is_enabled { "Отключить профиль" } else { "Подключить профиль" },
                    onclick: move |_| ontoggle.call(profile_toggle_id.clone()),

                    if is_updating {
                        div { class: "w-5 h-5 border-2 border-zinc-800 border-t-zinc-300 rounded-full animate-spin" }
                    } else {
                        if is_enabled {
                            Icon { icon: MdPowerOff, size: 16 }
                        } else {
                            Icon { icon: MdPower, size: 16 }
                        }
                    }
                }

                button {
                    class: "p-1.5 rounded-md text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer",
                    class: if is_updating { "opacity-50 cursor-not-allowed hover:bg-transparent hover:text-zinc-500" } else { "" },
                    disabled: is_updating,
                    title: "Удалить профиль",
                    onclick: move |_| ondelete.call(profile_delete_id.clone()),
                    Icon { icon: MdDelete, size: 16 }
                }
            }
        }
    }
}
