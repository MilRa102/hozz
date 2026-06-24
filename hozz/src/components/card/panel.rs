use dioxus::prelude::*;

#[component]
pub fn PanelCard(
    children: Element,
    #[props(default = String::new())] class: String,
) -> Element {
    rsx! {
        // Тонкая прозрачная граница вместо серой
        div { class: "bg-black/20 border border-white/10 rounded-xl shadow-sm backdrop-blur-sm {class}",
            {children}
        }
    }
}
