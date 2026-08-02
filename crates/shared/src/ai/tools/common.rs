use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ProxyStatusOutput {
    pub connected: bool,
    pub active_profile: String,
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct EmptyArgs;

#[derive(Debug, Deserialize)]
pub struct ToggleProxyArgs {
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct ProxyProfileOutput {
    pub id: String,
    pub name: Option<String>,
    pub source_url: String,
    pub update_interval: u64,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct AiToolError(pub String);

impl std::fmt::Display for AiToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AiToolError {}

impl From<anyhow::Error> for AiToolError {
    fn from(value: anyhow::Error) -> Self {
        Self(value.to_string())
    }
}
