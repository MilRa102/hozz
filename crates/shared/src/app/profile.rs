use config::default_true;
use serde::{Deserialize, Serialize};

use crate::utils::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProfileV1 {
    pub id: String,
    pub source: Source,
    pub last_update: u64,
    pub update_interval: u64,
}

/// Proxy node subscription profile
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
    pub source: Source,
    pub update_interval: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Listing sources for subscription profiles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Source {
    Remote(String),
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: None,
            source: Source::Remote(String::new()),
            update_interval: 3600,
            enabled: default_true(),
        }
    }
}

/// Fetches proxies from a subscription profile
impl Profile {
    pub fn new(url: &str) -> Self {
        let source = Source::Remote(url.to_string());
        let id = Self::generate_id(&source);

        Self {
            id,
            source,
            ..Default::default()
        }
    }

    fn generate_id(source: &Source) -> String {
        let Source::Remote(url) = source;
        generate_id(url)
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled
    }
}
