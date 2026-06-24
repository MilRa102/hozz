use serde::{Deserialize, Serialize};

// Disk data obtained via sysinfo.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiskData {
    pub name: String,
    pub mount_point: String,
    pub available_space: u64,
    pub total_space: u64,
    pub kind: String,
}

impl std::fmt::Display for DiskData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Диск: {}, Точка монтирования: {}, Доступно: {} GB, Всего: {} GB",
            self.name,
            self.mount_point,
            self.available_space / 1024 / 1024 / 1024,
            self.total_space / 1024 / 1024 / 1024
        )
    }
}

impl From<&sysinfo::Disk> for DiskData {
    fn from(disk: &sysinfo::Disk) -> Self {
        DiskData {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            available_space: disk.available_space(),
            total_space: disk.total_space(),
            kind: format!("{:?}", disk.kind()),
        }
    }
}

impl PartialEq for DiskData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.mount_point == other.mount_point
    }
}
