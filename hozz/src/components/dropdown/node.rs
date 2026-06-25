use dioxus::prelude::*;
use shared::apps::node::Node;

use crate::components::item::NodeItem;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct NodeDropdownProps {
    #[props(default)]
    nodes: Vec<Node>,
    select: EventHandler<String>,
}

#[component]
pub(crate) fn NodeDropdownItems(props: NodeDropdownProps) -> Element {
    rsx! {{
        props.nodes
        .iter()
        .map(|node| {
            let node = node.clone();
            let node_name: String = node.name.clone();
            rsx! {
                NodeItem {
                    key: "{node_name}",
                    node,
                    onclick: move |()| props.select.call(node_name.clone()),
                }
            }
        })
    }}
}
