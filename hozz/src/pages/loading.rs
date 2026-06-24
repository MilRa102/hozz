use dioxus::prelude::*;

#[component]
pub(crate) fn Skeleton() -> Element {
    rsx! {
        // Главный контейнер экрана
        // Задаем глобальный черный фон для App Shell
        div { class: "flex flex-col h-screen w-full bg-black font-sans cursor-wait overflow-hidden",

            // Основная рабочая область (Сайдбар + Контент)
            div { class: "flex flex-1 overflow-hidden",

                // 1. Сайдбар
                aside { class: "w-64 flex flex-col shrink-0 border-r border-white/10",
                    // Логотип приложения
                    div { class: "h-14 flex items-center px-4 gap-3 shrink-0",
                        div { class: "w-8 h-8 rounded-md bg-white/10 animate-pulse shrink-0" }
                        div { class: "h-3 w-24 bg-white/10 rounded animate-pulse" }
                    }

                    // Блоки навигации
                    div { class: "p-4 space-y-8 mt-2 flex-1",
                        // Группа 1
                        div { class: "space-y-3",
                            div { class: "h-2 w-12 bg-white/10 rounded animate-pulse mb-4" }
                            div { class: "h-8 w-full bg-white/5 rounded-md animate-pulse" }
                            div { class: "h-8 w-5/6 bg-white/5 rounded-md animate-pulse" }
                        }
                        // Группа 2
                        div { class: "space-y-3",
                            div { class: "h-2 w-16 bg-white/10 rounded animate-pulse mb-4" }
                            div { class: "h-8 w-4/5 bg-white/5 rounded-md animate-pulse" }
                            div { class: "h-8 w-full bg-white/5 rounded-md animate-pulse" }
                        }
                    }
                }

                // 2. Правая часть (Шапка + Рабочая область Canvas)
                div { class: "flex-1 flex flex-col min-w-0",

                    // Шапка приложения (Header)
                    header { class: "h-14 flex items-center justify-between px-4 shrink-0",
                        div { class: "flex items-center gap-3",
                            div { class: "h-8 w-8 bg-white/5 rounded-md animate-pulse" }
                            div { class: "h-3 w-32 bg-white/5 rounded animate-pulse hidden sm:block" }
                        }
                        div { class: "flex items-center gap-3",
                            div { class: "h-2 w-2 rounded-full bg-white/10 animate-pulse" }
                            div { class: "h-3 w-20 bg-white/5 rounded animate-pulse" }
                        }
                    }

                    // Область контента (Карточка с отступами)
                    main { class: "flex-1 overflow-hidden pb-4 pr-4 pl-2",
                        // Сам контейнер карточки
                        div { class: "w-full h-full bg-zinc-950 border border-white/10 rounded-xl flex flex-col p-6",

                            // Заголовок страницы
                            div { class: "h-6 w-48 bg-white/10 rounded animate-pulse mb-6 shrink-0" }

                            // Контейнер "таблицы"
                            div { class: "border border-white/10 rounded-lg flex-1 flex flex-col overflow-hidden",
                                // Заголовок таблицы (шапка)
                                div { class: "h-11 bg-white/5 border-b border-white/10 flex items-center px-4 gap-6 shrink-0",
                                    div { class: "h-3 w-32 bg-white/10 rounded animate-pulse" }
                                    div { class: "h-3 w-24 bg-white/10 rounded animate-pulse" }
                                    div { class: "h-3 w-40 bg-white/10 rounded animate-pulse" }
                                }

                                // Строки таблицы
                                for _ in 0..7 {
                                    div { class: "h-12 border-b border-white/5 flex items-center px-4 gap-6",
                                        div { class: "h-3 w-48 bg-white/5 rounded animate-pulse" }
                                        div { class: "h-3 w-20 bg-white/5 rounded animate-pulse" }
                                        div { class: "h-3 flex-1 bg-white/5 rounded animate-pulse" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 3. Строка состояния (Status bar)
            div { class: "h-8 bg-black border-t border-white/10 flex items-center px-4 justify-between shrink-0",
                div { class: "flex items-center gap-3",
                    div { class: "h-2 w-2 rounded-full bg-white/20 animate-pulse" }
                    div { class: "h-2.5 w-24 bg-white/10 rounded animate-pulse" }
                }
                div { class: "flex items-center gap-6",
                    div { class: "h-2.5 w-16 bg-white/10 rounded animate-pulse" }
                    div { class: "h-2.5 w-12 bg-white/10 rounded animate-pulse" }
                }
            }
        }
    }
}
