pub mod clipboard;
pub mod icon;
pub mod manage;
pub mod tray;

pub(crate) use clipboard::to_clipboard;
pub(crate) use icon::{AppIcon, Icon};
pub(crate) use manage::{proxy_management, tray_management};
