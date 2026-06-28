use std::{io::Error, sync::Arc, time::Duration};

use async_trait::async_trait;
use db::SledManager;
use futures::StreamExt;
use tokio::time::sleep;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

use crate::{
    apps::{
        LoggingLayer, NodeManager, ORCH, Orchestrator, node::Traffic,
        proxy::CoreController,
    },
    core::models::rule::{Rule, Target},
};

/// Trait defining the interface for background monitoring tasks.
///
/// This trait provides methods to monitor traffic statistics, group synchronization,
/// active connections, and TUN device status in real-time.
#[async_trait]
pub(crate) trait BackgroundTasks {
    /// Launches all background monitoring tasks as asynchronous jobs.
    ///
    /// Starts four separate `tokio::spawn` tasks:
    /// - `traffic_monitor`: Monitors incoming traffic data streams.
    /// - `groups_monitor`: Synchronizes local groups with the server.
    /// - `connections_monitor`: Tracks and manages connection rules based on activity.
    /// - `tun_monitor`: Handles TUN device lifecycle (Windows) and IP fetching (Linux).
    fn launch_background();

    /// Monitors real-time traffic data from the API.
    ///
    /// Continuously polls the server for traffic statistics, parses JSON responses,
    /// and updates the local state with received metrics. If the orchestrator is
    /// inactive or disconnected, it pauses monitoring to conserve resources.
    async fn traffic_monitor(&self);

    /// Synchronizes local group configurations with the server.
    ///
    /// Periodically fetches the latest group data from the API and updates the
    /// local storage. Pauses if the orchestrator is not active.
    async fn groups_monitor(&self);

    /// Monitors active connections and manages connection rules.
    ///
    /// Scans through all active connections, identifies those matching specific criteria
    /// (non-empty rules excluding "Match"), and creates/updates rules in the database.
    /// Triggers warnings for high-frequency requests to help users configure rules.
    async fn connections_monitor(&self);

    /// Monitors TUN device status and performs OS-specific maintenance tasks.
    ///
    /// - On Windows: Checks if the TUN device is enabled; if disconnected, attempts
    ///   to restart the connection.
    /// - On Linux: Periodically fetches the real IP address of the node.
    async fn tun_monitor(self: &Arc<Self>);
}

/// Implementation of `BackgroundTasks` for the `Orchestrator`.
#[async_trait]
impl BackgroundTasks for Orchestrator {
    /// Launches all background monitoring tasks as asynchronous jobs.
    ///
    /// Starts four separate `tokio::spawn` tasks:
    /// - `traffic_monitor`: Monitors incoming traffic data streams.
    /// - `groups_monitor`: Synchronizes local groups with the server.
    /// - `connections_monitor`: Tracks and manages connection rules based on activity.
    /// - `tun_monitor`: Handles TUN device lifecycle (Windows) and IP fetching (Linux).
    fn launch_background() {
        std::thread::sleep(Duration::from_millis(300));

        // Regular tasks
        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                arch.traffic_monitor().await;
            }
        });

        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                arch.groups_monitor().await;
            }
        });

        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                arch.connections_monitor().await;
            }
        });

        tokio::spawn(async move {
            if let Some(arch) = ORCH.get() {
                arch.tun_monitor().await;
            }
        });
    }

    /// Monitors real-time traffic data from the API.
    ///
    /// Continuously polls the server for traffic statistics, parses JSON responses,
    /// and updates the local state with received metrics. If the orchestrator is
    /// inactive or disconnected, it pauses monitoring to conserve resources.
    async fn traffic_monitor(&self) {
        let client = &self.dispatch.api;

        loop {
            // Check if we should pause monitoring due to inactivity or disconnection
            if !self.is_active() && !self.is_connected() {
                self.state.update_traffic(Traffic::default());
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            match client.traffic().await {
                Ok(response) => {
                    let stream = response
                        .bytes_stream()
                        .map(|r| r.map_err(Error::other));
                    let mut lines =
                        FramedRead::new(StreamReader::new(stream), LinesCodec::new());

                    while let Some(Ok(line)) = lines.next().await {
                        // Exit the loop if orchestrator becomes inactive
                        if !self.is_active() {
                            break;
                        }

                        match serde_json::from_str::<Traffic>(&line) {
                            Ok(stats) => self.state.update_traffic(stats),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to parse traffic data")
                            },
                        }
                    }
                },
                Err(e) => tracing::warn!(error = %e, "Failed to fetch traffic data"),
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Synchronizes local group configurations with the server.
    ///
    /// Periodically fetches the latest group data from the API and updates the
    /// local storage. Pauses if the orchestrator is not active.
    async fn groups_monitor(&self) {
        loop {
            // Check if we should pause monitoring due to inactivity
            if !self.is_active() {
                sleep(Duration::from_mins(1)).await;
                continue;
            }

            self.sync_groups().await;
            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Monitors active connections and manages connection rules.
    ///
    /// Scans through all active connections, identifies those matching specific criteria
    /// (non-empty rules excluding "Match"), and creates/updates rules in the database.
    /// Triggers warnings for high-frequency requests to help users configure rules.
    async fn connections_monitor(&self) {
        let client = &self.dispatch.api;

        loop {
            // Check if we should pause monitoring due to inactivity or disconnection
            if !self.is_active() && !self.is_connected() {
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            if let Ok(c) = client.connections().await {
                for conn in c.connections {
                    // Skip connections with empty rules or "Match" rule
                    if !conn.rule.is_empty() && conn.rule != "Match" {
                        continue;
                    }

                    // Determine process name and whether it's an application
                    let (name, is_app) = if !conn.metadata.process.is_empty() {
                        (conn.metadata.process.clone(), true)
                    } else if !conn.metadata.host.is_empty() {
                        (conn.metadata.host.clone(), false)
                    } else {
                        continue;
                    };

                    // Build the rule based on whether it's an app or host
                    let mut forward = if is_app {
                        Rule::builder(&name).target(Target::App).build()
                    } else {
                        Rule::builder(&name).target(Target::Host).build()
                    };

                    match self.rules.get(&forward.id) {
                        Ok(Some(mut existing)) => {
                            // If rule already exists and is active, skip updates
                            if existing.is_active() {
                                continue;
                            }

                            // Increment connection count for this rule
                            existing.inc_amt();
                            let amt = existing.amt;

                            // Save the updated rule to database
                            if let Err(e) = self.rules.save(&existing.id, &existing) {
                                tracing::warn!(error = %e, "Failed to save rule")
                            }

                            // Check if the rule is ignored
                            if existing.is_ignored() {
                                continue;
                            }

                            // Determine if a warning should be triggered based on connection count
                            let should_notify = match amt {
                                100 | 500 => true,
                                amt if amt > 500 && amt % 1000 == 0 => true,
                                _ => false,
                            };

                            if should_notify {
                                self.warning(format!(
                                    "Частые запросы к {}. Настроить правило?",
                                    existing.name
                                ));
                            }
                        },
                        Ok(None) => {
                            // Create a new rule for this connection
                            forward.inc_amt();
                            forward.deactivate();
                            if let Err(e) = self.rules.upsert(&forward) {
                                tracing::error!(error = %e, "Failed to save forward")
                            }
                        },
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to access forwards store")
                        },
                    }
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Monitors TUN device status and performs OS-specific maintenance tasks.
    ///
    /// - On Windows: Checks if the TUN device is enabled; if disconnected, attempts
    ///   to restart the connection.
    /// - On Linux: Periodically fetches the real IP address of the node.
    async fn tun_monitor(self: &Arc<Self>) {
        loop {
            // Check if we should pause monitoring due to disconnection
            if !self.is_connected() {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            #[cfg(target_os = "windows")]
            {
                let cfg = self
                    .dispatch
                    .api
                    .configs()
                    .await
                    .unwrap_or_default();

                let is_tun_alive = cfg.tun.enable == true;

                if !is_tun_alive {
                    tracing::warn!("TUN has disconnected. Restarting fuel...");
                    if let Err(e) = self.toggle_connection(true).await {
                        tracing::error!(error = %e, "Failed to toggle connection");
                    }

                    sleep(Duration::from_secs(15)).await;
                    continue;
                }
            }

            #[cfg(target_os = "linux")]
            {
                if let Err(e) = self.fetch_real_ip().await {
                    tracing::error!(error = %e, "Failed to fetch real IP");
                }
            }
            sleep(Duration::from_secs(40)).await;
        }
    }
}
