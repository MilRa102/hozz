use std::{io::Error, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::time::sleep;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

use crate::{
    app::{
        nodes::Traffic,
        orchestrator::{ORCH, Orchestrator},
    },
    core::models::rule::{Rule, Target},
    db::SledManager,
    infra::{CoreController, log::LoggingLayer, storage::group::GroupManager},
};

#[async_trait]
pub(crate) trait BackgroundTasks {
    /// Runs background tasks: traffic monitoring, group updating
    fn launch_background();

    /// Run background task: traffic monitoring
    async fn traffic_monitor(&self);

    /// Run background task: group updating
    async fn groups_monitor(&self);

    /// Run background task: connections monitoring
    async fn connections_monitor(&self);

    /// Run background task: TUN interface health check
    async fn tun_monitor(self: &Arc<Self>);
}

#[async_trait]
impl BackgroundTasks for Orchestrator {
    fn launch_background() {
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

    async fn traffic_monitor(&self) {
        let client = &self.dispatch.api;

        loop {
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

    async fn groups_monitor(&self) {
        loop {
            if !self.is_active() {
                sleep(Duration::from_mins(1)).await;
                continue;
            }

            self.sync_groups().await;
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connections_monitor(&self) {
        let client = &self.dispatch.api;

        loop {
            if !self.is_active() && !self.is_connected() {
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            if let Ok(c) = client.connections().await {
                for conn in c.connections {
                    if !conn.rule.is_empty() && conn.rule != "Match" {
                        continue;
                    }

                    let (name, is_app) = if !conn.metadata.process.is_empty() {
                        (conn.metadata.process.clone(), true)
                    } else if !conn.metadata.host.is_empty() {
                        (conn.metadata.host.clone(), false)
                    } else {
                        continue;
                    };

                    let mut forward = if is_app {
                        Rule::builder(&name).target(Target::App).build()
                    } else {
                        Rule::builder(&name).target(Target::Host).build()
                    };

                    match self.rules.get(&forward.id) {
                        Ok(Some(mut existing)) => {
                            if existing.is_active() {
                                continue;
                            }

                            existing.inc_amt();
                            let amt = existing.amt;
                            if let Err(e) = self.rules.save(&existing.id, &existing) {
                                tracing::warn!(error = %e, "Failed to save rule")
                            }

                            if existing.is_ignored() {
                                continue;
                            }

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

    async fn tun_monitor(self: &Arc<Self>) {
        loop {
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
