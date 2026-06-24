use std::{collections::HashMap, time::Duration};

use config::CONF;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use super::conf::Tun;

/// Clash client, used for requests to Sing-Box
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) client: reqwest::Client,
}

/// Returned response with a list of "Proxies"
#[derive(Debug, Clone, Deserialize)]
pub struct Proxies {
    pub proxies: HashMap<String, Proxy>,
}

/// Details of one of the "Proxies"
#[derive(Debug, Clone, Deserialize)]
pub struct Proxy {
    #[serde(rename = "type")]
    pub(crate) group_type: String,
    pub(crate) alive: bool,
    pub(crate) name: String,
    pub(crate) now: Option<String>,
    pub(crate) all: Option<Vec<String>>,
    pub(crate) history: Vec<LatencyHistory>,
    pub(crate) udp: bool,
}

/// History of one proxy server delay
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LatencyHistory {
    pub(crate) delay: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub(crate) tun: Tun,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    pub(crate) connections: Vec<ConnectionDetail>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionDetail {
    pub(crate) metadata: ConnectionMetadata,
    pub(crate) rule: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionMetadata {
    pub(crate) host: String,
    pub(crate) process: String,
}

impl Default for Client {
    fn default() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", CONF.mihomo.token.expose_secret())
                .parse()
                .expect("Failed to parse token as header value"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self { client }
    }
}
