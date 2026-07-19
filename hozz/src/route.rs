use dioxus::prelude::*;

use crate::{
    components::nav::Navbar,
    pages::{
        chat::ChatPage, docker::DockerContainers, home::Home, proxy::ProxyDashboard,
        resources::SystemResources, setting::SettingsView, vault::VaultPage,
    },
    widgets::docker::DockerContainer,
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},

    #[route("/system")]
    SystemResources { },

    #[route("/docker")]
    DockerContainers { },

    #[route("/docker/:id")]
    DockerContainer { id: String },

    #[route("/proxy")]
    ProxyDashboard {},

    #[route("/chat")]
    ChatPage {},

    #[route("/vault")]
    VaultPage {},

    #[route("/settings")]
    SettingsView {}
}
