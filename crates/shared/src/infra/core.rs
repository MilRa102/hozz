use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use config::CONF;
use tokio::{spawn, time::sleep};

use crate::{
    app::orchestrator::{ORCH, Orchestrator},
    apps::prefs::SystemProxyCapability,
    infra::{
        PrefsManager,
        log::LoggingLayer,
        storage::{profile::ProfileManager, rule::RuleManager},
    },
};

#[async_trait]
pub trait CoreController {
    /// Initial loading of the application
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// let _ = orch.bootstrap().await;
    /// ```
    async fn bootstrap(self: &Arc<Self>);

    /// Gracefully shuts down the application.
    ///
    /// This method is responsible for stopping any running services or tasks
    /// and cleaning up resources before the application exits.
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// Orchestrator::closing();
    /// ```
    fn closing();

    /// Grant privileges to run a child service
    ///
    /// # Returns
    /// Successfully granted privileges or an error occurred while attempting to grant privileges
    async fn ensure_privileges(&self) -> Result<()>;

    /// Toggle app connection
    ///
    /// # Arguments
    /// * `active` - The new connection status
    ///
    /// # Returns
    /// Switched state or execution error
    ///
    /// # Example
    /// ```
    /// use shared::app::orchestrator::Orchestrator;
    /// let orch = Orchestrator::init().await.unwrap();
    /// let _ = orch.toggle_connection(true).await;
    /// ```
    async fn toggle_connection(self: &Arc<Self>, active: bool) -> Result<()>;

    /// Retrieves the current real external IP address
    async fn fetch_real_ip(&self) -> Result<()>;

    fn restart_core(&self);
}

#[async_trait]
impl CoreController for Orchestrator {
    async fn bootstrap(self: &Arc<Self>) {
        let app = self.apps.fetch();

        if !app.is_privileged
            && let Err(e) = self.ensure_privileges().await
        {
            tracing::error!(error = %e, "Failed to ensure privileges");
        }

        if let Err(e) = self.sync_preferences().await {
            tracing::error!(error = %e, "Failed to sync preferences");
        }
        if let Err(e) = self.sync_profiles().await {
            tracing::error!(error = %e, "Failed to sync profiles");
        }
        if let Err(e) = self.sync_rules().await {
            tracing::error!(error = %e, "Failed to sync rules");
        }

        if app.is_connected {
            tokio::time::sleep(Duration::from_millis(200)).await;

            match self.toggle_connection(true).await {
                Ok(_) => tracing::info!("Restoring previous connection state.."),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to restore the last connection");
                    self.warning("Не удалось восстановить последнее соединение");
                },
            }
        }
    }

    fn closing() {
        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                let _ = machine::SysProxyController.toggle(false);
                arch.dispatch.core.stop().await;
            }
        });
    }

    async fn ensure_privileges(&self) -> Result<()> {
        self.dispatch.core.ensure_capabilities().await?;
        let mut app = self.apps.fetch();
        app.is_privileged = true;
        self.apps.update(&app)?;
        self.ok("Привилегии подтверждены");
        Ok(())
    }

    async fn toggle_connection(self: &Arc<Self>, active: bool) -> Result<()> {
        let mut ok = false;

        for attempt in 1..=3 {
            match self.dispatch.api.toggle_tun(active).await {
                Ok(()) => {
                    ok = true;
                    break;
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to toggle connection. Retrying...");

                    if attempt < 3 {
                        self.dispatch.api.restart().await.ok();
                        sleep(Duration::from_secs(1)).await;
                    }
                },
            }
        }

        if !ok {
            tracing::error!("Failed to toggle TUN after 3 attempts");
            return Err(anyhow::anyhow!(
                "Failed to toggle TUN after 3 attempts"
            ));
        }

        let mut app = self.apps.fetch();

        let controller = machine::SysProxyController;
        if active && self.preference_is_active::<SystemProxyCapability>() {
            if let Err(e) = controller.toggle(true) {
                self.warning(e.to_string());
            };
        } else {
            if let Err(e) = controller.toggle(false) {
                tracing::warn!(error = %e, "Failed to toggle System Proxy..");
            }
        };

        app.is_connected = active;
        self.apps.update(&app)?;

        self.set_active(active);
        self.set_connected(active);

        tracing::info!(state = %active, "TUN toggled successfully");
        Ok(())
    }

    async fn fetch_real_ip(&self) -> Result<()> {
        let max_retries = 5;
        let mut attempts = 0;
        let url = &CONF.app.myip_url;

        loop {
            attempts += 1;

            let request = reqwest::get(url).await;

            match request {
                Ok(response) if response.status().is_success() => {
                    let ip = response.text().await?;
                    self.state.update_ip(ip.trim());
                    return Ok(());
                },
                _ if attempts < max_retries => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                },
                Err(e) => {
                    tracing::error!(error = %e, "Unexpected error");
                    return Err(e.into());
                },
                Ok(response) => {
                    self.error(format!(
                        "Не удалось получить адрес: {}",
                        response.status()
                    ));
                    tracing::error!(status = %response.status(), "Failed to retrieve the address");
                    return Err(anyhow::anyhow!("Не удалось получить адрес."));
                },
            }
        }
    }

    fn restart_core(&self) {
        let dispatch = self.dispatch.clone();
        spawn(async move {
            if let Err(e) = dispatch.api.restart().await {
                tracing::error!(error = %e, "Failed to restart the kernel")
            }
        });
    }
}
