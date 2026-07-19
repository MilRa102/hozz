use std::{io::Read, sync::Arc, thread, time::Duration};

use ai::GenerationManager;
use dioxus::{desktop::use_window, logger::tracing, prelude::*};
use interprocess::local_socket::traits::ListenerExt;
use shared::apps::{
    LoggingLayer, NodeManager, ORCH, Orchestrator, Profile, node::GroupNode,
    proxy::CoreController,
};
use tokio::time::sleep;

use crate::{
    MAIN_CSS, TAILWIND_CSS,
    components::{control::TitleBar, toast::Toaster},
    pages::{errors::ErrorScreen, loading::Skeleton},
    route::Route,
    utils::{AppIcon, proxy_management, tray_management},
};

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AppState {
    pub is_connected: Signal<bool>,
    pub is_visible: Signal<bool>,
    pub wait_pending: Signal<bool>,
    pub active_ip: Signal<String>,
    pub active_profile: Signal<String>,
    pub up: Signal<String>,
    pub down: Signal<String>,
    pub groups: Signal<Vec<GroupNode>>,
    pub profiles: Signal<Vec<Profile>>,
}

impl AppState {
    fn new(orch: &Arc<Orchestrator>) -> Self {
        Self {
            is_connected: use_signal(|| orch.is_connected()),
            is_visible: use_signal(|| orch.is_active()),
            active_ip: use_signal(|| "Ожидание..".to_string()),
            ..Default::default()
        }
    }

    fn bootstrap(&self) {
        let app_icon = AppIcon::new();

        tray_management(app_icon.as_ref());
        proxy_management(*self);
    }

    pub(crate) fn toggle_proxy(&mut self, mut wait_pending: Signal<bool>) {
        if wait_pending() {
            return;
        }

        let mut has_connected = self.is_connected;
        let mut active_ip = self.active_ip;
        let target = !has_connected();

        spawn(async move {
            let arch = use_context::<Arc<Orchestrator>>();
            if target {
                wait_pending.set(true);
            }
            if arch.toggle_connection(target).await.is_err() {
                arch.error(
                    "Не удалось переключить. 
                     Пожалуйста перезагрузите приложение",
                );
            }
            has_connected.set(arch.is_connected());

            if target {
                active_ip.set("Поиск адреса..".to_string());
                sleep(Duration::from_millis(500)).await;
            }

            if let Err(e) = arch.fetch_real_ip().await {
                tracing::warn!(error = %e, "Failed to fetch real ip");
            };

            wait_pending.set(false);
        });
    }

    pub(crate) fn select_profile(&mut self, group: String, name: String) {
        spawn(async move {
            let arch = consume_context::<Arc<Orchestrator>>();
            arch.select_active_in_group(&group, &name).await;
        });
    }
}

#[component]
pub(crate) fn LoaderApp() -> Element {
    let window = use_window();
    use_hook(|| {
        let ipc_listener = consume_context::<machine::sock::IpcListener>();
        let listener_arc = ipc_listener.0.clone();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        let window_clone = window.clone();
        spawn(async move {
            while rx.recv().await.is_some() {
                tracing::info!("The UI thread received a command to expand.");
                window_clone.set_minimized(false);
                window_clone.set_visible(true);
                window_clone.set_focus();
            }
        });

        #[allow(clippy::unwrap_used)]
        thread::spawn(move || {
            let listener = listener_arc.lock().unwrap();

            for stream in listener.incoming().filter_map(Result::ok) {
                let mut stream = stream;
                let mut buffer = String::new();

                if stream.read_to_string(&mut buffer).is_ok()
                    && buffer.contains("WAKE_UP")
                {
                    tracing::info!(
                        "The background thread received a WAKE_UP from the second instance."
                    );
                    let _ = tx.send(());
                }
            }
        });
    });

    let mut initialized = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let mut init_task = use_resource(move || async move {
        match Orchestrator::init().await {
            Ok(_) => initialized.set(true),
            Err(e) => error.set(Some(e.to_string())),
        }
    });

    rsx! {
        style { "{MAIN_CSS}" }
        style { "{TAILWIND_CSS}" }

        if let Some(err) = error() {
            ErrorScreen { err, on_retry: move |()| init_task.restart() }
        } else if !initialized() {
            Skeleton {}
        } else {
            App {}
        }
    }
}

#[component]
fn App() -> Element {
    let arch = use_hook(|| {
        ORCH.get()
            .expect("Orchestrator not initialized")
            .clone()
    });
    use_context_provider(|| arch.clone());

    let app_state = AppState::new(&arch);
    use_context_provider(|| app_state);
    let generation_manager = use_hook(|| Arc::new(GenerationManager::new()));
    use_context_provider(|| generation_manager.clone());
    app_state.bootstrap();

    rsx! {
        div { class: "h-screen w-screen overflow-hidden text-zinc-50 bg-black font-sans flex flex-col",

            TitleBar {}

            Router::<Route> {}

            Toaster {}
        }
    }
}
