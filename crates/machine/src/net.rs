use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworksData {
    pub iface: String,
    pub received: u64,
    pub transmitted: u64,
}
