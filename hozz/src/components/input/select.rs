use dioxus::prelude::*;

#[component]
pub(crate) fn SettingSelect(
    options: Vec<String>,
    selected: String,
    onselect: EventHandler<String>,
) -> Element {
    let selected_style = move |option: &str| {
        if option == selected {
            "bg-zinc-700 text-white"
        } else {
            "text-zinc-400 hover:text-white"
        }
    };

    rsx! {
        div { class: "flex flex-wrap gap-1 bg-zinc-800 rounded-lg p-1",
            for opt in options {
                button {
                    class: "px-3 py-1 text-xs rounded-md transition-all cursor-pointer {selected_style(&opt)}",
                    onclick: move |_| onselect.call(opt.clone()),
                    "{opt}"
                }
            }
        }
    }
}

#[component]
pub(crate) fn SettingSelectVertical(
    options: Vec<String>,
    selected: String,
    onselect: EventHandler<String>,
) -> Element {
    let mut is_open = use_signal(|| false);
    let selected_for_style = selected.clone();
    let selected_style = move |option: &str| {
        if option == selected_for_style {
            "bg-white/10 text-white"
        } else {
            "text-zinc-400 hover:bg-white/5 hover:text-white"
        }
    };

    rsx! {
        div { class: "relative w-64",
            button {
                class: "flex h-9 w-full items-center justify-between rounded-lg border border-zinc-700 bg-zinc-950 px-3 text-left text-xs text-zinc-200 transition-colors hover:border-zinc-500 focus:border-zinc-500 focus:outline-none",
                aria_expanded: is_open(),
                onclick: move |_| is_open.toggle(),
                span { class: "truncate", "{selected}" }
                span { class: "ml-3 shrink-0 text-zinc-500", "⌄" }
            }

            if is_open() {
                div { class: "absolute right-0 z-20 mt-1 flex max-h-56 w-full flex-col overflow-y-auto rounded-lg border border-zinc-800 bg-zinc-950 py-1 shadow-xl",
                    for opt in options {
                        button {
                            class: "w-full px-3 py-2 text-left text-xs transition-colors cursor-pointer {selected_style(&opt)}",
                            onclick: move |_| {
                                is_open.set(false);
                                onselect.call(opt.clone());
                            },
                            "{opt}"
                        }
                    }
                }
            }
        }
    }
}
