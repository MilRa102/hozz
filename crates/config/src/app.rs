use serde::Deserialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    #[default]
    Error,
}

impl From<LogLevel> for String {
    fn from(level: LogLevel) -> Self {
        let level_str = match level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        level_str.to_string()
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_myip_url")]
    pub myip_url: String,
    #[serde(default)]
    pub level: LogLevel,
    #[serde(default)]
    pub is_admin: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            myip_url: default_myip_url(),
            level: LogLevel::default(),
            is_admin: false,
        }
    }
}

fn default_myip_url() -> String {
    "https://api64.ipify.org".to_string()
}
