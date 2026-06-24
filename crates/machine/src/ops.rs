use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use sysinfo::{Disks, Networks, System};

use crate::{DiskData, GpuData, NetworksData};

pub struct SystemMonitor {
    sys: System,
    nvml: Option<Nvml>,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(), // Вызываем тяжелую инициализацию ТОЛЬКО ОДИН РАЗ
            nvml: Nvml::init().ok(),
        }
    }

    /// Быстрое обновление данных (вызывается каждую секунду)
    pub fn fetch_data(&mut self) -> SystemData {
        // Обновляем метрики (это работает в 100 раз быстрее, чем new_all)
        self.sys.refresh_all();

        let disks_info = Disks::new_with_refreshed_list();
        let disks: Vec<DiskData> = disks_info.iter().map(DiskData::from).collect();

        let networks_info = Networks::new_with_refreshed_list();
        let networks: Vec<NetworksData> = networks_info
            .iter()
            .map(|(iface, data)| NetworksData {
                iface: iface.clone(),
                received: data.received(),
                transmitted: data.transmitted(),
            })
            .collect();

        let mut gpus = Vec::new();
        if let Some(nvml) = &self.nvml
            && let Ok(device_count) = nvml.device_count()
        {
            for i in 0..device_count {
                if let Ok(device) = nvml.device_by_index(i)
                    && let Ok(gpu_data) = GpuData::try_from(device)
                {
                    gpus.push(gpu_data);
                }
            }
        }

        let uptime_seconds = System::uptime();
        let cpu_name = self
            .sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(System::cpu_arch);

        let cpu_usage = self.sys.global_cpu_usage();

        SystemData {
            name: System::name(),
            kernel_version: System::kernel_version(),
            os_version: System::os_version(),
            hostname: System::host_name(),
            cpu_name,
            cpu_usage,
            cpu_count: self.sys.cpus().len(),
            memory_total: self.sys.total_memory(),
            memory_used: self.sys.used_memory(),
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            disks,
            gpus,
            networks,
            uptime_seconds,
        }
    }
}

// Collected system information, including CPU, memory, swap, disks, and GPU.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemData {
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub hostname: Option<String>,
    pub cpu_name: String,
    pub cpu_usage: f32,
    pub cpu_count: usize,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disks: Vec<DiskData>,
    pub gpus: Vec<GpuData>,
    pub networks: Vec<NetworksData>,
    pub uptime_seconds: u64,
}
