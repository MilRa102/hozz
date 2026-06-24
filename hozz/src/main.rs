mod components;
mod pages;
mod route;
mod utils;
mod widgets;

use std::sync::{Arc, Mutex};

use dioxus::{
    desktop::{
        Config, WindowBuilder, WindowCloseBehaviour,
        tao::window::Theme,
        wry::dpi::{LogicalPosition, LogicalSize, Position, Size},
    },
    prelude::*,
};

use crate::pages::app::LoaderApp;

pub(crate) const MAIN_CSS: &str = include_str!("../assets/main.css");
pub(crate) const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
pub(crate) const ZERO_EMPTY_BYTES: &[u8] = include_bytes!("../assets/zero_empty.webp");
pub(crate) const ZERO_MASKED_BYTES: &[u8] = include_bytes!("../assets/zero_masked.webp");
pub(crate) const ZERO_ERROR_BYTES: &[u8] = include_bytes!("../assets/zero_error.webp");

fn main() {
    #[allow(clippy::unwrap_used)]
    let _telemetry = telemetry::init_telemetry().unwrap();

    let listener = match machine::sock::enforce_socket() {
        Ok(Some(l)) => l,
        _ => return,
    };

    let args: Vec<String> = std::env::args().collect();
    let is_minimized = args.contains(&"--minimized".to_string());

    let wb = WindowBuilder::new()
        .with_title("Hozz")
        .with_inner_size(Size::Logical(LogicalSize::new(1200.0, 700.0)))
        .with_position(Position::Logical(LogicalPosition::new(
            100.0, 100.0,
        )))
        .with_visible(!is_minimized)
        .with_decorations(false)
        .with_theme(Some(Theme::Dark));

    let config = Config::new()
        .with_window(wb)
        .with_menu(None)
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        .with_tray_icon_show_window_on_click(true);

    let ipc_ctx = machine::sock::IpcListener(Arc::new(Mutex::new(listener)));

    LaunchBuilder::new()
        .with_cfg(desktop! { config })
        .with_context(ipc_ctx)
        .launch(LoaderApp);
}
