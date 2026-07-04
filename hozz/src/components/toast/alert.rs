use std::{sync::Arc, time::Duration};

use dioxus::prelude::*;
use dioxus_free_icons::icons::{
    md_action_icons::{MdDone, MdInfo},
    md_alert_icons::{MdError, MdWarning},
    md_navigation_icons::MdClose,
};
use shared::apps::{Alert, Orchestrator};
use tokio::time::sleep;

use crate::utils::Icon;

#[component]
pub(crate) fn Toaster() -> Element {
    let arch = use_context::<Arc<Orchestrator>>();
    let mut alerts = use_signal(Vec::<Alert>::new);

    use_future(move || {
        let arch = arch.clone();
        async move {
            let mut rx = arch.state.events.subscribe();
            while let Ok(alert) = rx.recv().await {
                alerts.with_mut(|v| v.push(alert));
            }
        }
    });

    rsx! {
        div { class: "fixed bottom-6 right-6 z-[100] flex flex-col items-end pointer-events-none gap-3 w-full max-w-sm",
            for alert in alerts() {
                div { class: "pointer-events-auto w-full", key: "{alert.id()}",
                    ToastItem {
                        alert: alert.clone(),
                        on_remove: move |id| {
                            alerts.with_mut(|v| v.retain(|a| a.id().to_string() != id));
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn ToastItem(alert: Alert, on_remove: EventHandler<String>) -> Element {
    let id = alert.id();

    use_future(move || async move {
        sleep(Duration::from_secs(5)).await;
        on_remove.call(id.to_string());
    });

    // Используем hex-цвета из палитры Tailwind для идеального совпадения с цветом бордера
    let (border_color, icon_elem) = match &alert {
        Alert::Error { .. } => (
            "border-l-red-500",
            rsx!(Icon {
                icon: MdError,
                color: "#ef4444"
            }), // red-500
        ),
        Alert::Ok { .. } => (
            "border-l-emerald-500",
            rsx!(Icon {
                icon: MdDone,
                color: "#10b981"
            }), // emerald-500
        ),
        Alert::Warning { .. } => (
            "border-l-amber-500",
            rsx!(Icon {
                icon: MdWarning,
                color: "#f59e0b"
            }), // amber-500
        ),
        Alert::Info { .. } => (
            "border-l-blue-500",
            rsx!(Icon {
                icon: MdInfo,
                color: "#3b82f6"
            }), // blue-500
        ),
    };

    rsx! {
        // Плотный фон, базовая обводка, цветной левый акцент и правильная тень
        div { class: "flex items-start gap-3 p-4 bg-zinc-900 border border-zinc-800 border-l-4 {border_color} rounded-lg shadow-xl shadow-black/20",
            div { class: "shrink-0 mt-0.5 text-lg", {icon_elem} }

            span { class: "flex-1 text-sm font-medium text-zinc-200 leading-relaxed",
                "{alert.message()}"
            }

            button {
                class: "shrink-0 text-zinc-500 hover:text-zinc-300 transition-colors focus:outline-none cursor-pointer",
                onclick: move |_| on_remove.call(id.to_string()),
                Icon { icon: MdClose, color: "#a1a1aa" } // zinc-400
            }
        }
    }
}
