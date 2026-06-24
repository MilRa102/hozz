use nvml_wrapper::Device;
use serde::{Deserialize, Serialize};

// Graphics card data obtained via NVML.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GpuData {
    pub model: String,
    pub used_memory: u64,
    pub total_memory: u64,
    pub temperature: u32,
}

impl std::fmt::Display for GpuData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Модель: {}, Память: {}/{} MB, Температура: {}°C",
            self.model,
            self.used_memory / 1024 / 1024,
            self.total_memory / 1024 / 1024,
            self.temperature
        )
    }
}

impl TryFrom<Device<'_>> for GpuData {
    type Error = anyhow::Error;

    fn try_from(device: Device) -> Result<Self, Self::Error> {
        let model = device.name()?;
        let mem_info = device.memory_info()?;
        let temperature = device
            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)?;

        Ok(GpuData {
            model,
            used_memory: mem_info.used,
            total_memory: mem_info.total,
            temperature,
        })
    }
}

impl PartialEq for GpuData {
    fn eq(&self, other: &Self) -> bool {
        self.model == other.model
    }
}
