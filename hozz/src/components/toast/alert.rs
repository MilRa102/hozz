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
        // Перенесли в правый нижний угол (bottom-6 right-6) и изменили выравнивание
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

    // Избавляемся от интерполяции строк, отдаем Tailwind'у полные классы.
    // Сделали светлые карточки с акцентными иконками и левым бордером (типично для GitLab/VS Code)
    let (border_color, icon_elem) = match &alert {
        Alert::Error { .. } => (
            "border-red-500/20",
            rsx!(Icon {
                icon: MdError,
                color: "red",
            }),
        ),
        Alert::Ok { .. } => (
            "border-green-500/20",
            rsx!(Icon {
                icon: MdDone,
                color: "green",
            }),
        ),
        Alert::Warning { .. } => (
            "border-amber-500/20",
            rsx!(Icon {
                icon: MdWarning,
                color: "orange",
            }),
        ),
        Alert::Info { .. } => (
            "border-blue-500/20",
            rsx!(Icon {
                icon: MdInfo,
                color: "blue",
            }),
        ),
    };

    rsx! {
        // Белый фон, тонкая тень, цветная левая граница
        div { class: "flex items-start gap-3 p-4 bg-zinc-950 border-l-4 border-y border-r {border_color} rounded-xl shadow-lg opacity-70",
            div { class: "shrink-0 mt-0.5 text-lg", {icon_elem} }

            span { class: "flex-1 text-sm text-zinc-300 leading-snug",
                "{alert.message()}"
            }

            button {
                class: "shrink-0 text-zinc-400 hover:text-zinc-600 transition-colors focus:outline-none",
                onclick: move |_| on_remove.call(id.to_string()),
                Icon { icon: MdClose }
            }
        }
    }
}
