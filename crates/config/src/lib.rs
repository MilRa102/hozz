mod app;
mod conf;
mod mihomo;
mod workspace;

pub use app::AppConfig;
pub use conf::Config;
pub use mihomo::MihomoConfig;
pub use workspace::WorkspaceConfig;

pub static CONF: std::sync::LazyLock<Config> = std::sync::LazyLock::new(|| {
    Config::load().expect("Failed to read environment variables")
});

pub fn default_true() -> bool {
    true
}
