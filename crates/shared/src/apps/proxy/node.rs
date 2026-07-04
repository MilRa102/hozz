use serde::Deserialize;

use crate::core::models::api;

/// Group of available proxies
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Proxy {
    pub groups: Vec<GroupNode>,
}

/// Proxy node group
#[derive(Debug, Clone, PartialEq)]
pub struct GroupNode {
    pub name: String,
    pub nodes: Vec<Node>,
    pub selector: String,
    pub selected: String,
}

/// Proxy node details
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub latency: u32,
    pub protocol: String,
    pub transport: String,
    pub available: bool,
    pub activated: bool,
}

/// Incoming and outgoing delays
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Traffic {
    pub up: u64,
    pub down: u64,
}

/// Proxy node transform details
impl From<api::Proxy> for Node {
    fn from(value: api::Proxy) -> Self {
        Self {
            name: value.name(),
            latency: value.latency(),
            protocol: value.protocol(),
            transport: value.transport(),
            available: value.alive,
            activated: false,
        }
    }
}

/// Proxy group nodes transform details
impl From<api::Proxies> for Vec<GroupNode> {
    fn from(value: api::Proxies) -> Self {
        let mut groups = Vec::new();

        for (name, proxy) in &value.proxies {
            if let Some(member_names) = &proxy.all {
                if proxy.name == "GLOBAL" {
                    continue;
                }

                let mut nodes = Vec::new();
                let selected = proxy.now.clone().unwrap_or_default();

                for node_name in member_names {
                    if let Some(raw) = value.proxies.get(node_name) {
                        let mut node = Node::from(raw.clone());
                        if node_name == &selected {
                            node.activated = true;
                        }
                        nodes.push(node);
                    }
                }
                nodes.sort_by_key(|n| n.latency);

                groups.push(GroupNode {
                    name: name.clone(),
                    selector: proxy.group_type.clone(),
                    selected,
                    nodes,
                });
            }
        }

        groups.sort_by_key(|b| b.name.clone());
        groups
    }
}
