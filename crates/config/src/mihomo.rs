use secrecy::SecretString;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize)]
pub struct MihomoConfig {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_url")]
    pub url: String,
    #[serde(default = "default_token")]
    pub token: SecretString,
    #[serde(default = "default_retry")]
    pub retry: i8,
    #[serde(default = "default_mixed_port")]
    pub mixed_port: u16,
}

impl Default for MihomoConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            url: default_url(),
            token: default_token(),
            retry: default_retry(),
            mixed_port: default_mixed_port(),
        }
    }
}

fn default_version() -> String {
    "v1.19.27".to_string()
}

fn default_url() -> String {
    "127.0.0.1:7090".to_string()
}

fn default_token() -> SecretString {
    // Исправлена опечатка machin_id -> machine_id
    let machine_id = machine_uid::get().unwrap_or_else(|_| "fallback-id".to_string());
    let salt = "com.Nodes.hozz-Salt";

    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(salt.as_bytes());

    SecretString::from(hex::encode(hasher.finalize()))
}

fn default_retry() -> i8 {
    5
}

fn default_mixed_port() -> u16 {
    7089
}
