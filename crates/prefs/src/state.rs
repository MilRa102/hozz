use rkyv::{Archive, Deserialize, Serialize};

/// Глобальные настройки пользователя
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct AppPrefs {
    pub key: String,
    pub value: String,
}

impl AppPrefs {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub fn as_bool(&self) -> bool {
        self.value.parse().unwrap_or(false)
    }

    pub fn as_u16(&self) -> u16 {
        self.value.parse().unwrap_or(0)
    }
}
