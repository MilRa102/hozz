use std::sync::Arc;

use dioxus::{
    desktop::{
        trayicon::{DioxusTrayIcon, init_tray_icon},
        use_window,
    },
    prelude::*,
};
use shared::{
    app::orchestrator::Orchestrator, infra::CoreController, utils::format_bytes,
};

use crate::{
    pages::app::AppState,
    utils::{
        AppIcon,
        tray::{TrayItems, setup_tray_events, sync_tray_state},
    },
};

/// Manages the system tray icon and its interactions.
///
/// This function sets up the tray icon, its menu items, and handles events triggered by these menu items.
pub(crate) fn tray_management(icon: Option<&Arc<AppIcon>>) {
    let window = use_window();
    let arch = use_context::<Arc<Orchestrator>>();
    let state = use_context::<AppState>();

    let items = use_hook(|| {
        let (items, menu) = TrayItems::build();
        let icon = icon
            .as_ref()
            .and_then(|i| i.build(DioxusTrayIcon::from_rgba));
        init_tray_icon(menu, icon);

        items
    });

    sync_tray_state(state, &items);
    setup_tray_events(arch, window, items);
}

/// Manages proxy-related operations and state updates.
///
/// This function sets up futures to track traffic metrics and group changes,
/// and initializes the orchestrator bootstrap process.
pub(crate) fn proxy_management(app: AppState) {
    let mut state = app;
    let orch = use_context::<Arc<Orchestrator>>();

    let arch = orch.clone();
    use_coroutine(move |_: UnboundedReceiver<()>| {
        let orch = arch.clone();
        async move {
            let mut visibility_rx = orch.state.visibility_rx.clone();
            let mut traffic_rx = orch.state.traffic_rx.clone();
            let mut groups_rx = orch.state.groups_rx.clone();
            let mut profiles_rx = orch.state.profiles_rx.clone();
            let mut active_profile_rx = orch.state.active_profile_rx.clone();
            let mut ip_rx = orch.state.ip_rx.clone();

            loop {
                tokio::select! {
                    // Слушаем видимость окна
                    Ok(_) = visibility_rx.changed() => {
                        let is_visible = *visibility_rx.borrow();
                        state.is_visible.set(is_visible);
                    }
                    // Слушаем трафик
                    Ok(_) = traffic_rx.changed() => {
                        let traffic = traffic_rx.borrow().clone();
                        state.up.set(format_bytes(traffic.up));
                        state.down.set(format_bytes(traffic.down));
                    }
                    // Слушаем группы
                    Ok(_) = groups_rx.changed() => {
                        let groups = groups_rx.borrow().clone();
                        state.groups.set(groups);
                    }
                    // Слушаем профили
                    Ok(_) = profiles_rx.changed() => {
                        let profiles = profiles_rx.borrow().clone();
                        state.profiles.set(profiles);
                    }
                    // Слушаем активный профиль
                    Ok(_) = active_profile_rx.changed() => {
                        let profile = active_profile_rx.borrow().clone();
                        state.active_profile.set(profile);
                    }
                    // Слушаем IP
                    Ok(_) = ip_rx.changed() => {
                        let ip = ip_rx.borrow().clone();
                        if !ip.is_empty() {
                            state.active_ip.set(ip);
                        } else {
                            state.active_ip.set("Ожидание..".to_string());
                        }
                    }
                }
            }
        }
    });

    // Bootstrap loader
    use_hook(move || {
        spawn(async move {
            orch.bootstrap().await;

            let window = use_window();
            if !window.is_visible() {
                orch.set_active(false);
            }
        });
    });
}
