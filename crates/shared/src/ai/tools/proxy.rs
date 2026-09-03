use std::sync::Arc;

use rig::tool::Tool;

use super::common::{AiToolError, EmptyArgs, ProxyStatusOutput, ToggleProxyArgs};
use crate::apps::{Orchestrator, proxy::CoreController};

#[derive(Clone)]
pub struct ProxyStatusTool {
    orch: Arc<Orchestrator>,
}

impl ProxyStatusTool {
    pub(crate) fn new(orch: Arc<Orchestrator>) -> Self {
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
    pub(crate) fn new(orch: Arc<Orchestrator>) -> Self {
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
