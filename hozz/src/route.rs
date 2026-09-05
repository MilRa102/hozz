use dioxus::prelude::*;

use crate::{
    components::nav::Navbar,
    pages::{
        chat::ChatPage, home::Home, proxy::ProxyDashboard, resources::SystemResources,
        setting::SettingsView, vault::VaultPage,
    },
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},

    #[route("/system")]
    SystemResources { },

    #[route("/proxy")]
    ProxyDashboard {},

    #[route("/chat")]
    ChatPage {},

    #[route("/vault")]
    VaultPage {},

    #[route("/settings")]
    SettingsView {}
}
