use db::SledManager;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct SecretVisit {
    pub mount: String,
    pub path: String,
    pub count: usize,
}

#[derive(Debug, Default, Clone, Archive, Serialize, Deserialize, PartialEq)]
pub struct VaultConfig {
    pub url: String,
    pub token: String,
    pub visited: Vec<SecretVisit>,
}

impl SecretVisit {
    pub fn new(mount: &str, path: &str) -> Self {
        Self {
            mount: mount.to_string(),
            path: path.to_string(),
            count: 1,
        }
    }
}

impl VaultConfig {
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            ..Default::default()
        }
    }
}

pub struct VaultStore;

impl SledManager<VaultConfig> for VaultStore {
    const TREE_NAME: &'static str = "vault";
}

impl VaultStore {
    const GLOBAL_KEY: &'static str = "global";

    pub fn fetch(&self) -> Option<VaultConfig> {
        self.get(Self::GLOBAL_KEY)
            .inspect_err(|e| tracing::error!(error = %e, "Failed to fetch Vault config"))
            .ok()
            .flatten()
    }

    pub fn update(&self, config: &VaultConfig) -> anyhow::Result<()> {
        self.save(Self::GLOBAL_KEY, config)
    }

    pub fn cleanup(&self) -> anyhow::Result<()> {
        self.delete(Self::GLOBAL_KEY)
    }
}
