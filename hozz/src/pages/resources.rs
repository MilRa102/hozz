use std::time::Duration;

use dioxus::prelude::*;
use machine::{SystemData, SystemMonitor};

use crate::widgets::system::{CpuWidget, DiskWidget, GpuWidget, MemoryWidget};

// Вспомогательная функция для Uptime
fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;

    if days > 0 {
        format!("{}д {}ч {}м", days, hours, mins)
    } else if hours > 0 {
        format!("{}ч {}м", hours, mins)
    } else {
        format!("{}м", mins)
    }
}

#[component]
pub fn SystemResources() -> Element {
    let mut sys_state = use_signal(|| None::<SystemData>);

    use_effect(move || {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn(move || {
            let mut monitor = SystemMonitor::new();

            loop {
                let data = monitor.fetch_data();

                if tx.send(data).is_err() {
                    break;
                }

                std::thread::sleep(Duration::from_secs(1));
            }
        });

        spawn(async move {
            while let Some(data) = rx.recv().await {
                sys_state.set(Some(data));
            }
        });
    });

    rsx! {
        div { class: "h-full bg-zinc-950 p-6 overflow-y-auto flex flex-col gap-6 text-zinc-100",

            if let Some(res) = sys_state() {
                // Заголовок страницы с Uptime
                div { class: "flex justify-between items-end shrink-0",
                    div { class: "flex flex-col gap-1.5",
                        h1 { class: "text-2xl font-semibold text-zinc-50 tracking-tight", "Системные ресурсы" }
                        p { class: "text-sm text-zinc-400", "Мониторинг аппаратных показателей в реальном времени" }
                    }
                    div { class: "text-right",
                        span { class: "text-[10px] font-semibold text-zinc-500 uppercase tracking-widest block mb-1", "В работе" }
                        span { class: "text-sm font-mono text-zinc-300 bg-zinc-900 border border-zinc-800 px-3 py-1.5 rounded-md",
                            "{format_uptime(res.uptime_seconds)}"
                        }
                    }
                }

                // Основные метрики (CPU, RAM, Swap, Видеокарты)
                div { class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 shrink-0",
                    // НОВЫЙ ВИДЖЕТ: CPU
                    CpuWidget {
                        name: res.cpu_name.clone(),
                        usage: res.cpu_usage,
                        cores: res.cpu_count,
                    }
                    MemoryWidget {
                        total: res.memory_total,
                        used: res.memory_used,
                        label: "Оперативная память".to_string(),
                    }
                    MemoryWidget {
                        total: res.swap_total,
                        used: res.swap_used,
                        label: "Файл подкачки (Swap)".to_string(),
                    }
                    for gpu in &res.gpus {
                        GpuWidget { gpu: gpu.clone() }
                    }
                }

                // Панель хранилищ
                div { class: "bg-zinc-900/30 border border-zinc-800/80 backdrop-blur-sm rounded-xl shadow-sm flex flex-col shrink-0 overflow-hidden",
                    div { class: "px-5 py-4 border-b border-zinc-800/50 bg-zinc-900/50 flex justify-between items-center",
                        h2 { class: "text-xs font-semibold text-zinc-300 uppercase tracking-widest", "Локальные накопители" }
                        span { class: "text-[10px] font-semibold text-zinc-400 bg-zinc-950 border border-zinc-800 px-2 py-1 rounded-md tracking-wider",
                            "Дисков: {res.disks.len()}"
                        }
                    }
                    div { class: "p-5 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4",
                        for disk in &res.disks {
                            DiskWidget { disk: disk.clone() }
                        }
                    }
                }
            } else {
                // Скелетон загрузки
                div { class: "flex-1 flex items-center justify-center text-zinc-500 text-sm gap-3",
                    div { class: "w-4 h-4 border-2 border-zinc-800 border-t-zinc-400 rounded-full animate-spin" }
                    "Опрос датчиков системы..."
                }
            }
        }
    }
}
