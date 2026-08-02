use rig::tool::Tool;

use crate::apps::docker::Container;

use super::common::{AiToolError, EmptyArgs};

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
