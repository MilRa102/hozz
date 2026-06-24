use std::collections::HashMap;

pub use bollard::models::ContainerSummaryStateEnum as ContainerStatus;
use bollard::{
    Docker,
    plugin::ContainerSummary,
    query_parameters::{ListContainersOptionsBuilder, LogsOptions},
};
use chrono::{DateTime, TimeZone, Utc};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Container {
    pub id: String,
    pub image: String,
    pub command: String,
    pub created: DateTime<Utc>,
    pub state: ContainerStatus,
    pub status: String,
    pub ports: String,
    pub names: String,
    pub size_rw: i64,
    pub size_root_fs: i64,
    pub labels: HashMap<String, String>,
}

impl Container {
    pub async fn list() -> Vec<Self> {
        let docker = Docker::connect_with_local_defaults().ok();

        if let Some(docker) = docker {
            let opts = ListContainersOptionsBuilder::new()
                .all(true)
                .build();
            let containers = docker.list_containers(Some(opts)).await.ok();
            if let Some(containers) = containers {
                return containers
                    .into_iter()
                    .map(Container::from)
                    .collect();
            }
        }
        Vec::new()
    }

    pub async fn inspect(id: String) -> Option<Self> {
        let containers = Self::list().await;
        if let Some(updated) = containers.into_iter().find(|c| c.id == id) {
            return Some(updated);
        }
        None
    }

    pub async fn stop(&self) -> bool {
        let docker = Docker::connect_with_local_defaults().ok();
        if let Some(docker) = docker {
            return docker
                .stop_container(&self.id, None)
                .await
                .is_ok();
        }
        false
    }

    pub async fn start(&self) -> bool {
        let docker = Docker::connect_with_local_defaults().ok();
        if let Some(docker) = docker {
            return docker
                .start_container(&self.id, None)
                .await
                .is_ok();
        }
        false
    }

    pub async fn restart(&self) -> bool {
        let docker = Docker::connect_with_local_defaults().ok();
        if let Some(docker) = docker {
            return docker
                .restart_container(&self.id, None)
                .await
                .is_ok();
        }
        false
    }

    #[allow(clippy::unwrap_used)]
    pub fn logs(&self) -> impl Stream<Item = String> {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let opts = Some(LogsOptions {
            stdout: true,
            stderr: true,
            tail: "100".to_string(),
            follow: true,
            timestamps: true,
            ..Default::default()
        });

        docker.logs(&self.id, opts).map(|log| match log {
            Ok(output) => output.to_string(),
            Err(e) => {
                tracing::warn!(error = %e, container_id = %self.id, "Error fetching logs for container");
                format!("Error fetching logs for container {}: {}", self.id, e)
            },
        })
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.state == ContainerStatus::RUNNING
    }
}

impl From<ContainerSummary> for Container {
    fn from(c: ContainerSummary) -> Self {
        Self {
            id: c.id.unwrap_or_default(),
            image: c.image.unwrap_or_default(),
            command: c.command.unwrap_or_default(),
            created: Utc
                .timestamp_opt(c.created.unwrap_or_default(), 0)
                .latest()
                .unwrap_or(Utc::now()),
            state: c.state.unwrap_or(ContainerStatus::CREATED),
            status: c.status.unwrap_or_default(),
            ports: c
                .ports
                .unwrap_or_default()
                .iter()
                .map(|p| {
                    format!(
                        "{} → {}",
                        p.private_port,
                        p.public_port.unwrap_or(0)
                    )
                })
                .collect::<Vec<String>>()
                .join(", "),
            names: c.names.unwrap_or_default().join(", "),
            size_rw: c.size_rw.unwrap_or_default(),
            size_root_fs: c.size_root_fs.unwrap_or_default(),
            labels: c.labels.unwrap_or_default(),
        }
    }
}
