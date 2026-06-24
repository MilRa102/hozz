mod autostart;
mod gpu;
mod net;
mod ops;
mod proxy;
pub mod sock;
mod ssd;

pub use autostart::AutostartApp;
pub use gpu::GpuData;
pub use net::NetworksData;
pub use ops::{SystemData, SystemMonitor};
pub use proxy::SysProxyController;
pub use ssd::DiskData;
