use rig::tool::Tool;

use super::common::{AiToolError, EmptyArgs};

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
