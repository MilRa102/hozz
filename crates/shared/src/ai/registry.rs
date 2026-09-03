use std::sync::Arc;

use rig::tool::server::{ToolServer, ToolServerHandle};

use crate::{
    ai::tools::{
        DockerListTool, ProfileListTool, ProxyStatusTool, ProxyToggleTool,
        SystemResourcesTool,
    },
    apps::Orchestrator,
};

pub trait AiRegistry {
    fn tool_server(self: &Arc<Self>) -> ToolServerHandle;
}

impl AiRegistry for Orchestrator {
    fn tool_server(self: &Arc<Self>) -> ToolServerHandle {
        self.ai_tool_server
            .get_or_init(|| {
                ToolServer::new()
                    .tool(ProxyStatusTool::new(self.clone()))
                    .tool(ProxyToggleTool::new(self.clone()))
                    .tool(ProfileListTool::new(self.clone()))
                    .tool(DockerListTool)
                    .tool(SystemResourcesTool)
                    .run()
            })
            .clone()
    }
}
