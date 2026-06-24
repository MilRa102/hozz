use dioxus::prelude::*;
use dioxus_free_icons::icons::md_alert_icons::MdWarning;

use crate::utils::Icon;

#[component]
pub(crate) fn ErrorScreen(err: String, on_retry: EventHandler<()>) -> Element {
    rsx! {
        // Глобальный фон: радикально черный
        div { class: "min-h-screen bg-black flex items-center justify-center p-6 font-sans",

            // Карточка: темно-цинковая с тонкой прозрачной рамкой
            div { class: "max-w-md w-full bg-zinc-950 border border-white/10 rounded-xl shadow-2xl p-6 text-left",

                // Заголовок
                div { class: "flex items-center gap-4 mb-5",
                    // Иконка: полупрозрачный красный фон, яркий текст/бордер
                    div { class: "w-10 h-10 bg-rose-500/10 rounded-lg flex items-center justify-center border border-rose-500/20 shrink-0",
                        span { class: "text-2xl text-rose-400 flex items-center",
                            // В Dioxus лучше управлять цветом через классы обертки (text-rose-400), если иконка это наследует
                            Icon { icon: MdWarning }
                        }
                    }
                    h1 { class: "text-zinc-50 text-lg font-semibold tracking-wide",
                        "Ошибка инициализации"
                    }
                }

                // Очень тонкий разделитель
                hr { class: "border-white/5 mb-5" }

                // Блок с техническим текстом ошибки
                div { class: "bg-zinc-900/50 border border-white/5 rounded-md p-3 mb-6",
                    p { class: "text-zinc-400 text-xs font-mono break-all leading-relaxed",
                        "{err}"
                    }
                }

                // Панель действий
                div { class: "flex justify-end",
                    button {
                        // Главная кнопка в стиле Vercel: светло-серый фон, темный текст
                        class: "px-4 py-2 bg-zinc-100 text-zinc-900 text-sm font-semibold rounded-md hover:bg-white transition-colors focus:outline-none focus:ring-2 focus:ring-zinc-500 focus:ring-offset-2 focus:ring-offset-zinc-950 active:scale-95",
                        onclick: move |_| on_retry.call(()),
                        "Повторить попытку"
                    }
                }
            }
        }
    }
}
