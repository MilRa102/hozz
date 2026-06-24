use dioxus::prelude::*;
use machine::{DiskData, GpuData};

#[component]
pub fn CpuWidget(name: String, usage: f32, cores: usize) -> Element {
    let color = if usage > 90.0 {
        "bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.5)]"
    } else if usage > 70.0 {
        "bg-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.5)]"
    } else {
        "bg-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.4)]" // CPU сделаем в стиле Cyan!
    };

    rsx! {
        div { class: "bg-zinc-900/30 border border-zinc-800/80 backdrop-blur-sm rounded-xl shadow-sm p-5 flex flex-col hover:border-zinc-700 hover:bg-zinc-900/50 transition-all duration-200",
            div { class: "flex justify-between items-start mb-4 gap-2",
                span { class: "text-sm font-semibold text-zinc-200 truncate tracking-tight", title: "{name}", "{name}" }
                span { class: "px-2 py-0.5 rounded-md text-[10px] font-mono font-bold text-zinc-400 bg-zinc-800/50 border border-zinc-700/50 shrink-0",
                    "{cores} ЯДЕР"
                }
            }

            div { class: "flex justify-between items-end mb-2.5 mt-auto",
                span { class: "text-[11px] text-zinc-500 font-medium uppercase tracking-widest", "Утилизация CPU" }
                span { class: "text-sm font-mono font-bold text-zinc-300", "{usage:.1}%" }
            }

            div { class: "w-full bg-zinc-950/50 rounded-full h-1.5 overflow-hidden border border-zinc-800/50",
                div {
                    class: "h-full rounded-full transition-all duration-500 {color}",
                    style: "width: {usage}%",
                }
            }
        }
    }
}

#[component]
pub fn GpuWidget(gpu: GpuData) -> Element {
    // Полупрозрачные бейджи температуры с легким свечением
    let temp_style = if gpu.temperature < 70 {
        "text-emerald-400 bg-emerald-500/10 border-emerald-500/20 shadow-[0_0_10px_rgba(16,185,129,0.1)]"
    } else if gpu.temperature < 85 {
        "text-amber-400 bg-amber-500/10 border-amber-500/20 shadow-[0_0_10px_rgba(245,158,11,0.1)]"
    } else {
        "text-red-400 bg-red-500/10 border-red-500/20 shadow-[0_0_10px_rgba(239,68,68,0.1)]"
    };

    let memory_usage = gpu
        .used_memory
        .checked_mul(100)
        .and_then(|value| value.checked_div(gpu.total_memory))
        .unwrap_or(0);

    // Цвет полосы прогресса со свечением
    let mem_color = if memory_usage > 90 {
        "bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.5)]"
    } else if memory_usage > 75 {
        "bg-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.5)]"
    } else {
        "bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]"
    };

    rsx! {
        div { class: "bg-zinc-900/30 border border-zinc-800/80 backdrop-blur-sm rounded-xl shadow-sm p-5 flex flex-col hover:border-zinc-700 hover:bg-zinc-900/50 transition-all duration-200",
            div { class: "flex justify-between items-start mb-4 gap-2",
                span { class: "text-sm font-semibold text-zinc-200 truncate tracking-tight", title: "{gpu.model}", "{gpu.model}" }

                span { class: "px-2 py-0.5 rounded-md text-[10px] font-mono font-bold uppercase tracking-widest border {temp_style} shrink-0",
                    "{gpu.temperature}°C"
                }
            }

            div { class: "flex justify-between items-end mb-2.5 mt-auto",
                span { class: "text-[11px] text-zinc-500 font-medium uppercase tracking-widest", "VRAM Утилизация" }
                span { class: "text-sm font-mono font-bold text-zinc-300", "{memory_usage}%" }
            }

            div { class: "w-full bg-zinc-950/50 rounded-full h-1.5 overflow-hidden border border-zinc-800/50",
                div {
                    class: "h-full rounded-full transition-all duration-500 {mem_color}",
                    style: "width: {memory_usage}%",
                }
            }
        }
    }
}

#[component]
pub fn MemoryWidget(total: u64, used: u64, label: String) -> Element {
    let usage_pct = used
        .checked_mul(100)
        .and_then(|value| value.checked_div(total))
        .unwrap_or(0);
    let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_gb = total as f64 / 1024.0 / 1024.0 / 1024.0;

    // В спокойном состоянии используем нейтральный белый/серый цвет
    let color = if usage_pct > 90 {
        "bg-red-500 shadow-[0_0_10px_rgba(239,68,68,0.5)]"
    } else if usage_pct > 75 {
        "bg-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.5)]"
    } else {
        "bg-zinc-300 shadow-[0_0_10px_rgba(228,228,231,0.4)]"
    };

    rsx! {
        div { class: "bg-zinc-900/30 border border-zinc-800/80 backdrop-blur-sm rounded-xl shadow-sm p-5 flex flex-col hover:border-zinc-700 hover:bg-zinc-900/50 transition-all duration-200",
            div { class: "flex justify-between items-start mb-4",
                span { class: "text-[11px] font-semibold text-zinc-400 uppercase tracking-widest", "{label}" }
                span { class: "text-xs font-mono font-bold text-zinc-500", "{usage_pct}%" }
            }

            div { class: "flex items-baseline gap-1.5 mb-4",
                h3 { class: "text-3xl font-bold text-zinc-100 tracking-tighter", "{used_gb:.1}" }
                span { class: "text-xs text-zinc-500 font-medium", "/ {total_gb:.1} GB" }
            }

            div { class: "w-full bg-zinc-950/50 rounded-full h-1.5 mt-auto overflow-hidden border border-zinc-800/50",
                div {
                    class: "h-full rounded-full transition-all duration-700 {color}",
                    style: "width: {usage_pct}%",
                }
            }
        }
    }
}

#[component]
pub fn DiskWidget(disk: DiskData) -> Element {
    let used = disk
        .total_space
        .saturating_sub(disk.available_space);
    let usage_pct = used
        .checked_mul(100)
        .and_then(|value| value.checked_div(disk.total_space))
        .unwrap_or(0);

    let color = if usage_pct > 90 {
        "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.5)]"
    } else if usage_pct > 75 {
        "bg-amber-400 shadow-[0_0_8px_rgba(245,158,11,0.5)]"
    } else {
        "bg-zinc-400 shadow-[0_0_8px_rgba(161,161,170,0.3)]"
    };

    // Очищаем kind от лишних кавычек или мусора, если они есть из debug формата
    let clean_kind = disk.kind.replace("\"", "");

    rsx! {
        div { class: "border border-zinc-800/80 rounded-lg p-3.5 bg-zinc-950/30 hover:bg-zinc-800/40 hover:border-zinc-700 transition-all flex flex-col group",
            div { class: "flex justify-between items-center mb-1.5",
                div { class: "flex items-center gap-2 overflow-hidden pr-2",
                    // Бейджик SSD / HDD
                    span { class: "text-[9px] px-1.5 py-0.5 rounded text-zinc-400 bg-zinc-800 border border-zinc-700 font-bold tracking-wider shrink-0",
                        "{clean_kind}"
                    }
                    span { class: "text-sm font-medium text-zinc-300 group-hover:text-zinc-100 transition-colors truncate tracking-tight", title: "{disk.mount_point}",
                        "{disk.mount_point}"
                    }
                }
                span { class: "text-[11px] font-mono font-semibold text-zinc-500 group-hover:text-zinc-400 shrink-0", "{usage_pct}%" }
            }

            p { class: "text-[10px] text-zinc-600 font-mono mb-3.5 truncate uppercase tracking-wider", title: "{disk.name}",
                "{disk.name}"
            }

            div { class: "w-full bg-zinc-950 h-1 rounded-full overflow-hidden mt-auto",
                div {
                    class: "h-full rounded-full transition-all duration-500 {color}",
                    style: "width: {usage_pct}%",
                }
            }
        }
    }
}
