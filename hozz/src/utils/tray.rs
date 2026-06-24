use std::{sync::Arc, time::Duration};

use dioxus::{
    desktop::{
        DesktopContext,
        muda::{MenuId, MenuItem, PredefinedMenuItem},
        trayicon::DioxusTrayMenu,
        use_muda_event_handler,
    },
    logger::tracing,
    prelude::*,
};
use shared::{app::orchestrator::Orchestrator, infra::CoreController};

use crate::{pages::app::AppState, utils::to_clipboard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenu {
    CopyProfile,
    CopyIp,
    ToggleProxy,
    ToggleWindow,
    Restart,
    Quit,
}

impl From<TrayMenu> for MenuId {
    fn from(value: TrayMenu) -> Self {
        let id = match value {
            TrayMenu::CopyProfile => "tray_copy_profile",
            TrayMenu::CopyIp => "tray_copy_ip",
            TrayMenu::ToggleProxy => "tray_toggle_proxy",
            TrayMenu::ToggleWindow => "tray_toggle_window",
            TrayMenu::Restart => "tray_restart",
            TrayMenu::Quit => "tray_quit",
        };
        MenuId::new(id)
    }
}

impl TrayMenu {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "tray_copy_profile" => Some(Self::CopyProfile),
            "tray_copy_ip" => Some(Self::CopyIp),
            "tray_toggle_proxy" => Some(Self::ToggleProxy),
            "tray_toggle_window" => Some(Self::ToggleWindow),
            "tray_restart" => Some(Self::Restart),
            "tray_quit" => Some(Self::Quit),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct TrayItems {
    profile: MenuItem,
    ip: MenuItem,
    toggle_proxy: MenuItem,
    toggle_window: MenuItem,
    restart: MenuItem,
    quit: MenuItem,
}

impl TrayItems {
    /// Создает элементы и собирает их в меню
    pub(crate) fn build() -> (Self, DioxusTrayMenu) {
        let items = Self {
            profile: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::CopyProfile.into(),
                "Профиль:",
                true,
                None,
            ),
            ip: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::CopyIp.into(),
                "IP:",
                true,
                None,
            ),
            toggle_proxy: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::ToggleProxy.into(),
                "Вкл. прокси",
                true,
                None,
            ),
            toggle_window: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::ToggleWindow.into(),
                "Развернуть окно",
                true,
                None,
            ),
            restart: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::Restart.into(),
                "Перезапустить прокси",
                true,
                None,
            ),
            quit: MenuItem::with_id::<MenuId, &str>(
                TrayMenu::Quit.into(),
                "Закрыть приложение",
                true,
                None,
            ),
        };

        let menu = DioxusTrayMenu::new();
        let _ = menu.append_items(&[
            &items.profile,
            &items.ip,
            &PredefinedMenuItem::separator(),
            &items.toggle_proxy,
            &items.toggle_window,
            &PredefinedMenuItem::separator(),
            &items.restart,
            &items.quit,
        ]);

        (items, menu)
    }
}

/// Синхронизирует состояние приложения (Dioxus) с текстом в меню трея
pub(crate) fn sync_tray_state(state: AppState, items: &TrayItems) {
    use_effect({
        let item = items.profile.clone();
        let active_profile = state.active_profile;
        move || item.set_text(format!("Профиль: {}", active_profile()))
    });

    use_effect({
        let item = items.ip.clone();
        let active_ip = state.active_ip;
        move || item.set_text(format!("IP: {}", active_ip()))
    });

    use_effect({
        let item = items.toggle_window.clone();
        let is_visible = state.is_visible;
        move || {
            let text = if is_visible() {
                "Свернуть окно"
            } else {
                "Развернуть окно"
            };
            item.set_text(text);
        }
    });

    use_effect({
        let item = items.toggle_proxy.clone();
        let is_connected = state.is_connected;
        move || {
            let text = if is_connected() {
                "Откл. прокси"
            } else {
                "Вкл. прокси"
            };
            item.set_text(text);
        }
    });
}

/// Обрабатывает клики по меню трея
pub(crate) fn setup_tray_events(
    arch: Arc<Orchestrator>,
    window: DesktopContext,
    items: TrayItems,
) {
    use_muda_event_handler(move |evt| {
        let Some(tray_menu) = TrayMenu::from_str(evt.id().as_ref()) else {
            return;
        };

        match tray_menu {
            TrayMenu::CopyProfile => {
                let text = arch.state.active_profile_rx.borrow().clone();
                if let Err(e) = to_clipboard(&text) {
                    tracing::warn!(error = %e, "Failed to Clipboard: {}", text);
                }
            },
            TrayMenu::CopyIp => {
                let ip = arch.state.ip_rx.borrow().clone();
                if !ip.is_empty()
                    && ip != "Ожидание.."
                    && let Err(e) = to_clipboard(&ip)
                {
                    tracing::warn!(error = %e, "Failed to Clipboard: {}", ip);
                }
            },
            TrayMenu::ToggleProxy => {
                let arch = arch.clone();
                let item = items.toggle_proxy.clone();
                spawn(async move {
                    let active = !arch.is_connected();
                    if arch.toggle_connection(active).await.is_ok() {
                        consume_context::<AppState>()
                            .is_connected
                            .set(active);
                        item.set_text(if active {
                            "Откл. прокси"
                        } else {
                            "Вкл. прокси"
                        });
                    }
                });
            },
            TrayMenu::ToggleWindow => {
                let visible = window.is_visible();
                window.set_visible(!visible);
                arch.set_active(!visible);

                if !visible {
                    window.set_minimized(false);
                    window.set_focus();
                }
            },
            TrayMenu::Restart => arch.restart_core(),
            TrayMenu::Quit => {
                Orchestrator::closing();
                window.close();
                std::thread::sleep(Duration::from_secs(2));
                tracing::info!("Bye..");
                std::process::exit(0);
            },
        }
    });
}
