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
    pub id: String,                      // container ID or name
    pub image: String,                   // container image name
    pub command: String,                 // command string (commandline)
    pub created: DateTime<Utc>,          // when the container was started
    pub state: ContainerStatus,          // current operational state
    pub status: String,                  // overall system status (e.g., "active")
    pub ports: String,                   // public interfaces (tcp: 0-65535)
    pub names: String,                   // all container names
    pub size_rw: i64,                    // read/write disk space used
    pub size_root_fs: i64,               // read-only filesystem extent
    pub labels: HashMap<String, String>, // metadata tags
}

impl Container {
    /// List all running containers from Docker.
    ///
    /// # Arguments:
    /// * None - No additional options
    ///   Returns a vector of Container summaries.
    ///
    /// # Examples:
    /// ```rust
    /// let containers = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(containers, vec![Some(Container { ... })]);
    /// ```
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

    /// Inspect detailed information about a specific container by ID.
    ///
    /// # Arguments:
    /// * `id` - The container's unique identifier.
    ///   Returns the Container struct or None if not found.
    ///
    /// # Examples:
    /// ```rust
    /// let container = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(container.inspect("abc123").unwrap().id, "test-id");
    /// ```
    pub async fn inspect(id: String) -> Option<Self> {
        let containers = Self::list().await;
        if let Some(updated) = containers.into_iter().find(|c| c.id == id) {
            return Some(updated);
        }
        None
    }

    /// Stop a running container.
    ///
    /// # Arguments:
    /// * `self` - The container object to stop.
    ///   Returns true if the operation succeeded, false otherwise.
    ///
    /// # Examples:
    /// ```rust
    /// let stopped = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(stopped.stop(), true);
    /// assert!(!docker::Docker::connect_with_local_defaults()
    ///     .await()
    ///     .stop("xyz789".to_string())
    ///     .is_ok()); // False for non-running containers
    /// ```
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

    /// Start a running container from its stop state.
    ///
    /// # Arguments:
    /// * `self` - The container object to start.
    ///   Returns true if the operation succeeded, false otherwise.
    ///
    /// # Examples:
    /// ```rust
    /// let started = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(started.start(), true);
    /// assert!(!docker::Docker::connect_with_local_defaults()
    ///     .await()
    ///     .start("xyz789".to_string())
    ///     .is_ok()); // False for containers still stopped
    /// ```
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

    /// Restart a container to restart it if stopped.
    ///
    /// # Arguments:
    /// * `self` - The container object to restart.
    ///   Returns true if the operation succeeded, false otherwise.
    ///
    /// # Examples:
    /// ```rust
    /// let restarted = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(restarted.restart(), true);
    /// assert!(!docker::Docker::connect_with_local_defaults()
    ///     .await()
    ///     .restart("xyz789".to_string())
    ///     .is_ok()); // False for containers in stopped state
    /// ```
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

    /// Fetch log messages from a container.
    ///
    /// # Arguments:
    /// * `self` - The container object to log.
    ///   Returns the Stream of log entries or Err if unable to retrieve logs.
    ///
    /// # Examples:
    /// ```rust
    /// let streams = docker::Docker::connect_with_local_defaults()
    ///     .await;
    /// assert_eq!(streams.logs(Some(LogsOptions {
    ///     stdout: true,
    ///     stderr: true,
    ///     tail: "100".to_string(),
    ///     follow: true,
    /// })).map(|log| log.to_string()));
    /// ```
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

    /// Check if a container is currently running.
    ///
    /// # Arguments:
    /// * `self` - The container object to check.
    ///   Returns true if the container is running, false otherwise.
    ///
    /// # Examples:
    /// ```rust
    /// assert!(docker::Docker::connect_with_local_defaults()
    ///     .await()
    ///     .is_running("abc123".to_string())
    ///     .unwrap()); // Should be true for containers in RUNNING state
    /// ```
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
