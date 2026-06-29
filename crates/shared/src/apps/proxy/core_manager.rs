use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use config::CONF;
use tokio::{spawn, time::sleep};

use crate::apps::{
    LoggingLayer, ORCH, Orchestrator, PrefsManager, ProfileManager, RuleManager,
    prefs::SystemProxyCapability,
};

/// A trait defining the interface for controlling the core proxy functionality.
///
/// This trait provides methods to manage the lifecycle and state of the system proxy,
/// including privilege management, connection toggling, IP address retrieval, and core restart.
/// It is implemented by the `Orchestrator` struct to provide centralized control over proxy operations.
///
/// # Methods
/// * `bootstrap()` - Initializes the core controller by syncing all application data and restoring connection state.
/// * `closing()` - Performs cleanup actions when the application is shutting down.
/// * `ensure_privileges()` - Verifies and grants necessary system privileges for proxy operation.
/// * `toggle_connection()` - Toggles the TUN device and system proxy on/off with retry logic.
/// * `fetch_real_ip()` - Retrieves the current external IP address from a configured URL.
/// * `restart_core()` - Triggers a kernel/system restart to apply changes.
#[async_trait]
pub trait CoreController {
    /// Initializes the core controller by syncing all application data and restoring connection state.
    ///
    /// This method performs several initialization tasks:
    /// 1. Ensures system privileges are granted if not already present.
    /// 2. Synchronizes preferences, profiles, and rules from storage.
    /// 3. Restores the previous connection state if the application was previously connected.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    async fn bootstrap(self: &Arc<Self>);

    /// Performs cleanup actions when the application is shutting down.
    ///
    /// This method stops the system proxy and halts the core dispatch loop. It runs asynchronously
    /// in a spawned task to ensure graceful shutdown without blocking the main thread.
    fn closing();

    /// Toggles the TUN device and system proxy on/off with retry logic.
    ///
    /// This method attempts to toggle the connection state up to 3 times. For each attempt, it:
    /// 1. Calls the API to toggle the TUN device.
    /// 2. If successful, updates the application state and notifies the user.
    /// 3. If unsuccessful, restarts the kernel and retries after a 1-second delay.
    ///
    /// After toggling, it also controls the system proxy based on the active preference setting.
    /// Finally, it updates the connection state in the application store and sets the active status.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    /// * `active` - The desired connection state (true for on, false for off).
    ///
    /// # Returns
    /// * `Result<()>` - Success if the toggle completes within 3 attempts, or an error if all attempts fail.
    async fn toggle_connection(self: &Arc<Self>, active: bool) -> Result<()>;

    /// Retrieves the current external IP address from a configured URL.
    ///
    /// This method attempts to fetch the real external IP address by making HTTP GET requests
    /// to a URL specified in the configuration. It retries up to 5 times with a 500ms delay
    /// between attempts to handle transient network issues. Upon success, it updates the application
    /// state with the retrieved IP address and returns.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the IP is retrieved, or an error if all attempts fail or the response is invalid.
    async fn fetch_real_ip(&self) -> Result<()>;

    /// Triggers a kernel/system restart to apply changes.
    ///
    /// This method initiates a system restart via the API dispatch layer. It spawns an asynchronous task
    /// to perform the restart, ensuring the main thread is not blocked. The restart process will be handled
    /// by the operating system's reboot mechanism.
    fn restart_core(&self);
}

#[async_trait]
impl CoreController for Orchestrator {
    /// Initializes the core controller by syncing all application data and restoring connection state.
    ///
    /// This method performs several initialization tasks:
    /// 1. If the application was previously connected (`app.is_connected`), it waits briefly (200ms)
    ///    before attempting to restore the previous connection state via `toggle_connection(true)`.
    /// 2. Synchronizes preferences, profiles, and rules from storage using their respective sync methods.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    async fn bootstrap(self: &Arc<Self>) {
        let app = self.apps.fetch();

        // Restore previous connection state if applicable
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

        // Sync preferences, profiles, and rules
        let (pref_res, prof_res, rules_res) = tokio::join!(
            self.sync_preferences(),
            self.sync_profiles(),
            self.sync_rules()
        );

        if let Err(e) = pref_res {
            tracing::error!(error = %e, "Failed to sync preferences");
        }
        if let Err(e) = prof_res {
            tracing::error!(error = %e, "Failed to sync profiles");
        }
        if let Err(e) = rules_res {
            tracing::error!(error = %e, "Failed to sync rules");
        }
    }

    /// Performs cleanup actions when the application is shutting down.
    ///
    /// This method stops the system proxy and halts the core dispatch loop. It runs asynchronously
    /// in a spawned task to ensure graceful shutdown without blocking the main thread.
    fn closing() {
        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                // Stop the system proxy controller
                let _ = machine::SysProxyController.toggle(false);
                // Stop the core dispatch loop
                arch.dispatch.core.stop().await;
            }
        });
    }

    /// Toggles the TUN device and system proxy on/off with retry logic.
    ///
    /// This method attempts to toggle the connection state up to 3 times. For each attempt, it:
    /// 1. Calls the API to toggle the TUN device.
    /// 2. If successful, updates the application state and notifies the user.
    /// 3. If unsuccessful, restarts the kernel and retries after a 1-second delay.
    ///
    /// After toggling, it also controls the system proxy based on the active preference setting.
    /// Finally, it updates the connection state in the application store and sets the active status.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    /// * `active` - The desired connection state (true for on, false for off).
    ///
    /// # Returns
    /// * `Result<()>` - Success if the toggle completes within 3 attempts, or an error if all attempts fail.
    async fn toggle_connection(self: &Arc<Self>, active: bool) -> Result<()> {
        let mut ok = false;

        // Retry loop up to 3 times
        for attempt in 1..=3 {
            match self.dispatch.api.toggle_tun(active).await {
                Ok(()) => {
                    ok = true;
                    break;
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to toggle connection. Retrying...");

                    if attempt < 3 {
                        // Restart kernel on failure and wait before retrying
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

        // Control system proxy based on preference
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

    /// Retrieves the current external IP address from a configured URL.
    ///
    /// This method attempts to fetch the real external IP address by making HTTP GET requests
    /// to a URL specified in the configuration. It retries up to 5 times with a 500ms delay
    /// between attempts to handle transient network issues. Upon success, it updates the application
    /// state with the retrieved IP address and returns.
    ///
    /// # Arguments
    /// * `self` - A reference to the core controller instance.
    ///
    /// # Returns
    /// * `Result<()>` - Success if the IP is retrieved, or an error if all attempts fail or the response is invalid.
    async fn fetch_real_ip(&self) -> Result<()> {
        let max_retries = 5;
        let mut attempts = 0;
        let url = &CONF.app.myip_url;

        loop {
            attempts += 1;

            let request = reqwest::get(url).await;

            match request {
                // Success case: Valid IP received
                Ok(response) if response.status().is_success() => {
                    let ip = response.text().await?;
                    self.state.update_ip(ip.trim());
                    return Ok(());
                },
                // Retry case: Request failed but not max retries reached
                _ if attempts < max_retries => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                },
                Err(e) => {
                    tracing::error!(error = %e, "Unexpected error");
                    return Err(e.into());
                },
                // Error case: Invalid response status
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

    /// Triggers a kernel/system restart to apply changes.
    ///
    /// This method initiates a system restart via the API dispatch layer. It spawns an asynchronous task
    /// to perform the restart, ensuring the main thread is not blocked. The restart process will be handled
    /// by the operating system's reboot mechanism.
    fn restart_core(&self) {
        let dispatch = self.dispatch.clone();
        spawn(async move {
            if let Err(e) = dispatch.api.restart().await {
                tracing::error!(error = %e, "Failed to restart the kernel")
            }
        });
    }
}
