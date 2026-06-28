use dioxus::prelude::*;

#[component]
pub(crate) fn SettingSwitch(is_active: bool, ontoggle: EventHandler<bool>) -> Element {
    let (bg_color, knob_color, translate) = if is_active {
        ("bg-white", "bg-black", "translate-x-3")
    } else {
        ("bg-zinc-800", "bg-zinc-400", "translate-x-0")
    };

    rsx! {
        button {
            class: "relative inline-flex h-5 w-9 items-center rounded-full bg-zinc-950 p-1 outline-none focus:ring-2 focus:ring-white/20 transition-colors cursor-pointer",
            onclick: move |_| ontoggle.call(!is_active),

            span { class: "absolute inset-0 rounded-full transition-colors duration-200 ease-in-out {bg_color}" }
            span { class: "absolute inline-flex h-4 w-4 items-center justify-center rounded-full shadow ring-0 transition-transform duration-200 ease-in-out {knob_color} {translate}" }
        }
    }
}
