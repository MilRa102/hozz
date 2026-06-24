use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_free_icons::{Icon as DxIcon, IconShape};
use image::GenericImageView;

// Material Design Icon Component
#[component]
pub fn Icon<T: IconShape + Clone + PartialEq + 'static>(
    icon: T,
    #[props(default = 24)] size: u32,
    #[props(default = "white")] color: &'static str,
    #[props(default = "")] class: &'static str,
) -> Element {
    let fill = color.to_string();
    rsx! {DxIcon {
        class: class,
        width: size,
        height: size,
        fill: "{fill}",
        icon: icon,
    }}
}

// App Icon
pub(crate) struct AppIcon {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

impl AppIcon {
    pub(crate) fn new() -> Option<Arc<Self>> {
        let bytes = include_bytes!("../../assets/zero.png");
        let image = image::load_from_memory(bytes)
            .inspect_err(|error| error!(%error, "Failed to load icon"))
            .ok()?;
        let (width, height) = image.dimensions();
        let rgba = image.to_rgba8().into_raw();
        Some(Arc::new(Self {
            rgba,
            width,
            height,
        }))
    }

    pub(crate) fn build<T, E, F>(&self, factory: F) -> Option<T>
    where
        F: FnOnce(Vec<u8>, u32, u32) -> anyhow::Result<T, E>,
        E: std::fmt::Display,
    {
        factory(self.rgba.clone(), self.width, self.height)
            .inspect_err(|error| error!(%error, "Failed to load icon"))
            .ok()
    }
}
