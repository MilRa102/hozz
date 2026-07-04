use std::time::Duration;

use config::CONF;
use reqwest::Response;
use serde_json::{json, to_value};

use super::models::{
    api::{Client, Config, Proxies, Proxy},
    conf::Mihomo,
};
use crate::core::models::api::Connection;

impl Client {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn api_url() -> String {
        format!("http://{}", CONF.mihomo.url)
    }

    /// Get all proxies from Clash API
    pub async fn all(&self) -> anyhow::Result<Proxies> {
        let url = format!("{}/proxies", Self::api_url());
        let response: Proxies = self.client.get(url).send().await?.json().await?;
        Ok(response)
    }

    /// Select proxy by name from Clash API
    pub async fn select(&self, selector: &str, name: &str) -> anyhow::Result<()> {
        let url = format!("{}/proxies/{}", Self::api_url(), selector);
        self.client
            .put(url)
            .json(&json!({ "name": name }))
            .send()
            .await?;
        Ok(())
    }

    /// Get traffic stats from Clash API
    pub async fn traffic(&self) -> anyhow::Result<Response> {
        let url = format!("{}/traffic", Self::api_url());
        let response = self.client.get(url).send().await?;
        Ok(response)
    }

    /// Get configs from Clash API
    pub(crate) async fn configs(&self) -> anyhow::Result<Config> {
        let url = format!("{}/configs", Self::api_url());
        let response: Config = self.client.get(url).send().await?.json().await?;
        Ok(response)
    }

    /// Toggle TUN mode from Clash API
    pub async fn toggle_tun(&self, enable: bool) -> anyhow::Result<()> {
        let url = format!("{}/configs", Self::api_url());
        let mut config = Config::default();
        let mut duration = Duration::from_millis(200);

        if enable {
            config.tun.enable = enable;
        }

        let payload = to_value(config)?;
        for _ in 1..=CONF.mihomo.retry {
            match self
                .client
                .patch(&url)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    if let Ok(extrn_cfg) = self.configs().await
                        && extrn_cfg.tun.enable == enable
                    {
                        return Ok(());
                    }

                    tracing::warn!(%enable, "Core hasn't applied settings yet.");
                },
                Ok(response) => {
                    tracing::warn!(
                        status = %response.status(),
                        "Core returned non-success status"
                    );
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Network error while toggling tun. Retrying..");
                },
            }

            tokio::time::sleep(duration).await;
            duration *= 2;
        }

        Err(anyhow::anyhow!(
            "Failed after retries: {}",
            CONF.mihomo.retry
        ))
    }

    /// Update configs from Clash API
    pub(crate) async fn update_config(&self, config: &Mihomo) -> anyhow::Result<()> {
        let url = format!("{}/configs?force=true", Self::api_url());

        let response = self.client.put(&url).json(&config).send().await?;
        if !response.status().is_success() {
            let error = response.text().await?;
            tracing::error!(action = "updateConfig", %error);
            anyhow::bail!("Failed to update configuration");
        }

        Ok(())
    }

    /// Update a specific proxy provider (Profile)
    pub(crate) async fn update_provider(&self, name: &str) -> anyhow::Result<()> {
        let url = format!("{}/providers/proxies/{}", Self::api_url(), name);

        let response = self.client.put(&url).send().await?;
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            tracing::error!(action = "updateProvider", provider = name, %error);
            anyhow::bail!("Failed to update provider `{name}`");
        }

        Ok(())
    }

    /// Perform a health check (delay test) for a specific proxy provider
    pub(crate) async fn healthcheck_provider(&self, name: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/providers/proxies/{}/healthcheck",
            Self::api_url(),
            name
        );

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            tracing::error!(action = "healthcheckProvider", provider = name, %error);
            anyhow::bail!("Failed to run healthcheck for provider `{name}`");
        }

        Ok(())
    }

    /// Get connections from Clash API
    pub(crate) async fn connections(&self) -> anyhow::Result<Connection> {
        let url = format!("{}/connections?interval=5000", Self::api_url());
        let response: Connection = self.client.get(url).send().await?.json().await?;
        Ok(response)
    }

    /// Restart Clash API
    pub(crate) async fn restart(&self) -> anyhow::Result<()> {
        let url = format!("{}/restart", Self::api_url());
        self.client.post(url).send().await?;
        Ok(())
    }
}

impl Proxy {
    /// Get proxy name from Clash API
    #[must_use]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get proxy transport from Clash API
    #[must_use]
    pub fn transport(&self) -> String {
        let transport = if self.udp { "udp" } else { "tcp" };
        transport.to_string()
    }

    /// Get proxy protocol from Clash API
    #[must_use]
    pub fn protocol(&self) -> String {
        self.group_type.clone()
    }

    /// Get proxy latency from Clash API
    #[must_use]
    pub fn latency(&self) -> u32 {
        let latency = self.history.last().cloned().unwrap_or_default();
        latency.delay
    }
}
