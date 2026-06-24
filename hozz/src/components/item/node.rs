use dioxus::prelude::*;
use shared::app::nodes::Node;

/// Node item
///
/// # Arguments
/// * `node` - Node data
/// * `onclick` - Click handler
#[component]
pub(crate) fn NodeItem(node: Node, onclick: EventHandler<()>) -> Element {
    // В темной теме выделенный элемент подсвечиваем белым бордером и светлым фоном
    let (active_bg, active_border, active_text) = if node.activated {
        (
            "bg-white/10",
            "border-l-6",
            "text-zinc-100 font-semibold",
        )
    } else {
        (
            "hover:bg-white/5",
            "border-transparent",
            "text-zinc-400 font-medium",
        )
    };

    let opacity_class = if node.available {
        "opacity-100"
    } else {
        "opacity-40"
    };

    let trimmed = node.name.trim();
    let mut icon = None;
    let mut display_name = trimmed;

    if let Some(emoji_char) = trimmed.chars().next()
        && !emoji_char.is_ascii_alphanumeric()
        && emoji_char != '['
        && emoji_char != '('
    {
        if let Some(idx) =
            trimmed.find(|c: char| c.is_ascii_alphanumeric() || c == '[' || c == '(')
        {
            let (i, rest) = trimmed.split_at(idx);
            let i = i.trim();
            if !i.is_empty() {
                let last_icon = i.split_whitespace().last().unwrap_or(i);

                icon = Some(last_icon.to_string());
                display_name = rest.trim();
            }
        } else {
            let last_icon = trimmed
                .split_whitespace()
                .last()
                .unwrap_or(trimmed);
            icon = Some(last_icon.to_string());
            display_name = "";
        }
    }

    rsx! {
        div {
            class: "flex items-center justify-between px-4 py-3 cursor-pointer transition-colors {active_bg} {active_border} {opacity_class}",
            onclick: move |_| onclick.call(()),

            div { class: "flex items-center gap-3",
                if let Some(emoji) = icon {
                    div { class: "flex-shrink-0 w-8 h-8 flex items-center justify-center bg-white/5 rounded-md border border-white/5 text-lg shadow-sm",
                        "{emoji}"
                    }
                }


                div { class: "flex flex-col",
                    p { class: "text-sm {active_text} truncate", "{display_name}" }
                    p { class: "text-[10px] text-zinc-500 font-mono mt-0.5 uppercase tracking-wider",
                        "{node.protocol} · {node.transport}"
                    }
                }
            }

            div { class: "text-right",
                if node.available {
                    p { class: "text-xs font-mono text-zinc-500", "{node.latency} мс" }
                } else {
                    p { class: "text-xs font-mono text-rose-500/70", "Недоступен" }
                }
            }
        }
    }
}
