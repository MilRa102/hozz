use dioxus::prelude::*;

#[component]
pub(crate) fn SettingSelect(
    options: Vec<&'static str>,
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
        div { class: "flex bg-zinc-800 rounded-lg p-1",
            for opt in options {
                button {
                    class: "px-3 py-1 text-xs rounded-md transition-all cursor-pointer {selected_style(opt)}",
                    onclick: move |_| onselect.call(opt.to_string()),
                    "{opt}"
                }
            }
        }
    }
}
