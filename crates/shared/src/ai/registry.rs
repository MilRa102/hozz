use std::sync::Arc;

use ai::AiPrefsReader;
use rig::tool::server::{ToolServer, ToolServerHandle};

use crate::{
    ai::tools::{
        DockerListTool, ProfileListTool, ProxyStatusTool, ProxyToggleTool,
        SystemResourcesTool, TavilyExtractTool,
    },
    apps::Orchestrator,
};

pub trait AiRegistry {
    fn tool_server(self: &Arc<Self>) -> ToolServerHandle;
    fn refresh_tool_server(self: &Arc<Self>) -> ToolServerHandle;
}

impl AiRegistry for Orchestrator {
    fn tool_server(self: &Arc<Self>) -> ToolServerHandle {
        let guard = self.ai_tool_server.read().unwrap();
        if let Some(server) = guard.clone() {
            return server;
        }

        drop(guard);
        self.refresh_tool_server()
    }

    fn refresh_tool_server(self: &Arc<Self>) -> ToolServerHandle {
        let server = {
            let mut builder = ToolServer::new()
                .tool(ProxyStatusTool::new(self.clone()))
                .tool(ProxyToggleTool::new(self.clone()))
                .tool(ProfileListTool::new(self.clone()))
                .tool(DockerListTool)
                .tool(SystemResourcesTool);

            if let Some(apikey) = AiPrefsReader.tavily_api_key() {
                builder = builder
                    .tool(crate::ai::tools::TavilySearchTool::new(apikey.clone()))
                    .tool(TavilyExtractTool::new(apikey));
            }

            builder.run()
        };

        let mut guard = self.ai_tool_server.write().unwrap();
        *guard = Some(server.clone());
        server
    }
}
