use base64::{Engine, engine::general_purpose};
use dioxus::prelude::*;

use crate::{ZERO_EMPTY_BYTES, ZERO_ERROR_BYTES, ZERO_MASKED_BYTES};

#[component]
pub(crate) fn ZeroEmpty(title: Option<String>, description: Option<String>) -> Element {
    let img_src = use_memo(move || {
        let b64 = general_purpose::STANDARD.encode(ZERO_EMPTY_BYTES);
        format!("data:image/webp;base64,{b64}")
    });

    let display_title = title.unwrap_or_else(|| "ДАННЫЕ НЕ НАЙДЕНЫ".to_string());
    let display_desc = description.unwrap_or_else(|| {
        "Зеро не смог найти информацию. Попробуй изменить параметры.".to_string()
    });

    rsx! {
        div { class: "flex flex-row items-center justify-center gap-6 p-8 w-full",

            img {
                src: "{img_src}",
                class: "w-32 h-32 object-contain shrink-0 drop-shadow-[0_0_15px_rgba(6,182,212,0.3)] opacity-80 hover:opacity-100 transition-opacity duration-300",
                alt: "Empty State"
            }

            div { class: "flex flex-col items-start max-w-sm text-left",
                h3 { class: "text-zinc-300 font-mono text-sm uppercase tracking-widest",
                    "{display_title}"
                }
                p { class: "text-zinc-500 text-xs mt-2 leading-relaxed",
                    "{display_desc}"
                }
            }
        }
    }
}

#[component]
pub(crate) fn ZeroMasked(title: Option<String>, description: Option<String>) -> Element {
    let img_src = use_memo(move || {
        let b64 = general_purpose::STANDARD.encode(ZERO_MASKED_BYTES);
        format!("data:image/png;base64,{}", b64)
    });

    let display_title = title.unwrap_or_else(|| "СОЕДИНЕНИЕ ЗАЩИЩЕНО".to_string());
    let display_desc = description.unwrap_or_else(|| {
        "Зеро успешно применил профиль. Трафик зашифрован и скрыт от посторонних глаз.".to_string()
    });

    rsx! {
        div { class: "flex flex-row items-center justify-center gap-6 p-8 w-full",
            img {
                src: "{img_src}",
                // Свечение цвета Индиго
                class: "w-32 h-32 object-contain shrink-0 drop-shadow-[0_0_15px_rgba(99,102,241,0.3)] opacity-80 hover:opacity-100 transition-opacity duration-300",
                alt: "Masked State"
            }

            div { class: "flex flex-col items-start max-w-sm text-left",
                h3 { class: "text-zinc-300 font-mono text-sm uppercase tracking-widest",
                    "{display_title}"
                }
                p { class: "text-zinc-500 text-xs mt-2 leading-relaxed",
                    "{display_desc}"
                }
            }
        }
    }
}

#[component]
pub(crate) fn ZeroError(title: Option<String>, description: Option<String>) -> Element {
    let img_src = use_memo(move || {
        let b64 = general_purpose::STANDARD.encode(ZERO_ERROR_BYTES);
        format!("data:image/webp;base64,{b64}")
    });

    let display_title = title.unwrap_or_else(|| "СИСТЕМНАЯ ОШИБКА".to_string());
    let display_desc = description.unwrap_or_else(|| {
        "Зеро не может обработать эти данные. Проверь конфигурацию или загляни в консоль.".to_string()
    });

    rsx! {
        div { class: "flex flex-row items-center justify-center gap-6 p-8 w-full",
            img {
                src: "{img_src}",
                // Красное свечение (Error)
                class: "w-32 h-32 object-contain shrink-0 drop-shadow-[0_0_15px_rgba(239,68,68,0.3)] opacity-80 hover:opacity-100 transition-opacity duration-300",
                alt: "Error State"
            }

            div { class: "flex flex-col items-start max-w-sm text-left",
                // Заголовок окрашен в красный для акцента
                h3 { class: "text-red-400 font-mono text-sm uppercase tracking-widest",
                    "{display_title}"
                }
                p { class: "text-zinc-500 text-xs mt-2 leading-relaxed",
                    "{display_desc}"
                }
            }
        }
    }
}
