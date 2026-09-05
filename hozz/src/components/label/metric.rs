use dioxus::prelude::*;

/// Displays traffic
///
/// # Arguments
/// * `icon` - Traffic icon
/// * `value` - Traffic value
/// * `label` - Traffic label
///
/// # Return
/// * `Element` - A fragment ready for rendering
#[component]
pub(crate) fn MetricLabel(icon: Element, value: String, label: String) -> Element {
    rsx! {
        div { class: "flex items-center gap-2",
            div { class: "text-zinc-500", {icon} }
            div { class: "flex flex-col",
                p { class: "text-sm font-mono font-medium text-zinc-200 leading-none", "{value}" }
                p { class: "text-[10px] text-zinc-500 uppercase tracking-wider font-bold mt-0.5", "{label}" }
            }
        }
    }
}
