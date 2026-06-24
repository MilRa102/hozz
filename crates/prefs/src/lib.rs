mod hook;
mod meta;
mod registry;
mod state;

pub use hook::{PreferenceHook, PreferenceKey};
pub use meta::*;
pub use registry::SettingsRegistry;
pub use state::AppPrefs;
