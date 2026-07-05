use crate::utils::Icon;
use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    md_action_icons::{MdDone, MdInfo},
    md_alert_icons::{MdError, MdWarning},
};

/// Importance levels of the information block
#[derive(Clone, Copy, PartialEq, Default)]
pub enum CalloutIntent {
    #[default]
    Info,
    Warning,
    Danger,
    Success,
}

#[derive(Props, Clone, PartialEq)]
pub struct CalloutProps {
    /// Importance level (determines colors and icon)
    #[props(default)]
    pub intent: CalloutIntent,

    /// Optional title for the callout block. If not provided, only the content will be displayed.
    #[props(default)]
    pub title: Option<String>,

    /// Content of any complexity (multi-line text, lists, tags)
    pub children: Element,
}

#[component]
pub fn Callout(props: CalloutProps) -> Element {
    let (bg_color, border_color, title_color, icon) = match props.intent {
        CalloutIntent::Info => (
            "bg-blue-500/5",
            "border-blue-500/20",
            "text-blue-400",
            rsx!(Icon {
                icon: MdInfo,
                color: "#3b82f6",
                size: 18
            }),
        ),
        CalloutIntent::Warning => (
            "bg-amber-500/5",
            "border-amber-500/20",
            "text-amber-500",
            rsx!(Icon {
                icon: MdWarning,
                color: "#f59e0b",
                size: 18
            }),
        ),
        CalloutIntent::Danger => (
            "bg-red-500/5",
            "border-red-500/20",
            "text-red-400",
            rsx!(Icon {
                icon: MdError,
                color: "#ef4444",
                size: 18
            }),
        ),
        CalloutIntent::Success => (
            "bg-emerald-500/5",
            "border-emerald-500/20",
            "text-emerald-400",
            rsx!(Icon {
                icon: MdDone,
                color: "#10b981",
                size: 18
            }),
        ),
    };

    rsx! {
        div { class: "flex gap-3 p-4 rounded-lg border {bg_color} {border_color}",
            div { class: "mt-0.5",
                {icon}
            }

            div { class: "flex flex-col gap-1.5 w-full",
                if let Some(t) = props.title {
                    span { class: "text-sm font-semibold tracking-wide {title_color}",
                        "{t}"
                    }
                }

                div { class: "text-sm text-zinc-300 leading-relaxed",
                    {props.children}
                }
            }
        }
    }
}
