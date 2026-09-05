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

#[derive(Debug, Deserialize)]
pub struct TavilySearchArgs {
    pub query: String,
    #[serde(default)]
    pub search_depth: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub max_results: Option<u8>,
    #[serde(default)]
    pub include_domains: Option<Vec<String>>,
    #[serde(default)]
    pub exclude_domains: Option<Vec<String>>,
    #[serde(default)]
    pub time_range: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TavilySearchResultOutput {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
    pub published_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TavilySearchOutput {
    pub query: String,
    pub results: Vec<TavilySearchResultOutput>,
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

impl From<rest::RestError> for AiToolError {
    fn from(value: rest::RestError) -> Self {
        Self(value.to_string())
    }
}
