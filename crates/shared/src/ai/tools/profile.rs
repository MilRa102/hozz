use std::sync::Arc;

use db::SledManager;
use rig::tool::Tool;

use crate::apps::{
    Orchestrator,
    proxy::Source,
};

use super::common::{AiToolError, EmptyArgs, ProxyProfileOutput};

#[derive(Clone)]
pub struct ProfileListTool {
    orch: Arc<Orchestrator>,
}

impl ProfileListTool {
    pub(crate) fn new(orch: Arc<Orchestrator>) -> Self {
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
