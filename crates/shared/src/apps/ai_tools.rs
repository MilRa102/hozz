use std::sync::Arc;

use db::SledManager;
use rig_core::tool::{Tool, ToolDyn};
use serde::{Deserialize, Serialize};

use crate::apps::{
    Orchestrator,
    docker::Container,
    proxy::{CoreController, Source},
};

#[derive(Debug, Serialize)]
pub struct ProxyStatusOutput {
    connected: bool,
    active_profile: String,
    ip: String,
}

#[derive(Debug, Deserialize)]
pub struct EmptyArgs;

#[derive(Debug, Deserialize)]
pub struct ToggleProxyArgs {
    active: bool,
}

#[derive(Debug, Serialize)]
pub struct ProxyProfileOutput {
    id: String,
    name: Option<String>,
    source_url: String,
    update_interval: u64,
    enabled: bool,
}

#[derive(Debug)]
pub struct AiToolError(String);

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

#[derive(Clone)]
pub struct ProxyStatusTool {
    orch: Arc<Orchestrator>,
}

impl ProxyStatusTool {
    fn new(orch: Arc<Orchestrator>) -> Self {
        Self { orch }
    }
}

impl Tool for ProxyStatusTool {
    const NAME: &'static str = "proxy_status";

    type Error = AiToolError;
    type Args = EmptyArgs;
    type Output = ProxyStatusOutput;

    fn description(&self) -> String {
        "Get current proxy status, active profile, and external IP".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(ProxyStatusOutput {
            connected: self.orch.is_connected(),
            active_profile: self.orch.state.active_profile_rx.borrow().clone(),
            ip: self.orch.state.ip_rx.borrow().clone(),
        })
    }
}

#[derive(Clone)]
pub struct ProxyToggleTool {
    orch: Arc<Orchestrator>,
}

impl ProxyToggleTool {
    fn new(orch: Arc<Orchestrator>) -> Self {
        Self { orch }
    }
}

impl Tool for ProxyToggleTool {
    const NAME: &'static str = "proxy_toggle";

    type Error = AiToolError;
    type Args = ToggleProxyArgs;
    type Output = ProxyStatusOutput;

    fn description(&self) -> String {
        "Enable or disable proxy connection".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "active": {
                    "type": "boolean",
                    "description": "Target proxy state (true=on, false=off)"
                }
            },
            "required": ["active"],
            "additionalProperties": false,
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.orch
            .toggle_connection(args.active)
            .await
            .map_err(AiToolError::from)?;

        if let Err(error) = self.orch.fetch_real_ip().await {
            tracing::warn!(error = %error, "Failed to refresh external IP after proxy toggle");
        }

        Ok(ProxyStatusOutput {
            connected: self.orch.is_connected(),
            active_profile: self.orch.state.active_profile_rx.borrow().clone(),
            ip: self.orch.state.ip_rx.borrow().clone(),
        })
    }
}

#[derive(Clone)]
pub struct DockerListTool;

impl Tool for DockerListTool {
    const NAME: &'static str = "docker_list_containers";

    type Error = AiToolError;
    type Args = EmptyArgs;
    type Output = Vec<Container>;

    fn description(&self) -> String {
        "List Docker containers with state and metadata".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(Container::list().await)
    }
}

#[derive(Clone)]
pub struct SystemResourcesTool;

impl Tool for SystemResourcesTool {
    const NAME: &'static str = "system_resources";

    type Error = AiToolError;
    type Args = EmptyArgs;
    type Output = machine::SystemData;

    fn description(&self) -> String {
        "Read current CPU, memory, disk, network and GPU metrics".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut monitor = machine::SystemMonitor::new();
        Ok(monitor.fetch_data())
    }
}

#[derive(Clone)]
pub struct ProfileListTool {
    orch: Arc<Orchestrator>,
}

impl ProfileListTool {
    fn new(orch: Arc<Orchestrator>) -> Self {
        Self { orch }
    }
}

impl Tool for ProfileListTool {
    const NAME: &'static str = "proxy_profiles";

    type Error = AiToolError;
    type Args = EmptyArgs;
    type Output = Vec<ProxyProfileOutput>;

    fn description(&self) -> String {
        "List proxy subscription profiles".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let profiles = self
            .orch
            .profiles
            .all()
            .map_err(AiToolError::from)?;

        Ok(profiles
            .into_iter()
            .map(|profile| {
                let Source::Remote(source_url) = profile.source;
                ProxyProfileOutput {
                    id: profile.id,
                    name: profile.name,
                    source_url,
                    update_interval: profile.update_interval,
                    enabled: profile.enabled,
                }
            })
            .collect())
    }
}

impl Orchestrator {
    pub fn ai_tools(self: &Arc<Self>) -> Vec<Box<dyn ToolDyn>> {
        vec![
            Box::new(ProxyStatusTool::new(self.clone())),
            Box::new(ProxyToggleTool::new(self.clone())),
            Box::new(ProfileListTool::new(self.clone())),
            Box::new(DockerListTool),
            Box::new(SystemResourcesTool),
        ]
    }
}
