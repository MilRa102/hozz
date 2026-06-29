use config::default_true;
use rkyv::{Archive, Deserialize, Serialize};

use crate::utils::generate_id;

/// Proxy node subscription profile
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
    pub source: Source,
    pub update_interval: u64,
    pub enabled: bool,
}

/// Listing sources for subscription profiles
#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
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
