use dioxus::prelude::*;
use prefs::{SettingMeta, SettingType};

use crate::components::input::{SettingSelect, SettingSwitch};

#[component]
pub(crate) fn SettingControl(
    meta: SettingMeta,
    value: String,
    onchange: EventHandler<String>,
) -> Element {
    match meta.setting_type {
        SettingType::Toggle => rsx! {
            SettingSwitch {
                is_active: value.parse().unwrap_or(false),
                ontoggle: move |v: bool| onchange.call(v.to_string())
            }
        },
        SettingType::Select(options) => rsx! {
            SettingSelect {
                options: options.to_vec(),
                selected: value,
                onselect: move |v| onchange.call(v)
            }
        },
        SettingType::TextInput => rsx! {
            input {
                class: "bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-white/20 transition-colors w-56",
                value: "{value}",
                oninput: move |evt| onchange.call(evt.value().clone()),
            }
        },
        SettingType::NumberInput { min, max } => rsx! {
            input {
                class: "bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-white outline-none focus:border-white/20 transition-colors w-40",
                r#type: "number",
                min: "{min}",
                max: "{max}",
                value: "{value}",
                oninput: move |evt| onchange.call(evt.value().clone()),
            }
        },
    }
}
