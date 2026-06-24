use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Danger,
    Primary,
}

#[component]
pub fn ActionButton(
    children: Element,
    onclick: EventHandler<MouseEvent>,
    #[props(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[props(default = String::new())] class: String,
) -> Element {
    let variant_style = match variant {
        ButtonVariant::Danger => {
            "bg-zinc-900/50 border-zinc-800 text-red-500 hover:bg-red-950/40 hover:border-red-900/50 hover:text-red-400"
        },
        ButtonVariant::Primary => {
            "bg-zinc-100 border-transparent text-zinc-900 hover:bg-white shadow-[0_0_12px_rgba(255,255,255,0.05)]"
        },
    };

    rsx! {
        button {
            // Улучшили анимацию (duration-200) и добавили focus-состояние для доступности
            class: "border px-3 py-1.5 rounded-md transition-all duration-200 cursor-pointer text-xs font-medium flex items-center justify-center gap-1.5 focus:outline-none focus:ring-2 focus:ring-zinc-600/50 focus:ring-offset-1 focus:ring-offset-zinc-950 active:scale-[0.98] {variant_style} {class}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}
