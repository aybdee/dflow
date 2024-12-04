use petgraph::graph::NodeIndex;

use crate::cfg::{CfGraph, CfgNode};

pub fn create_and_connect(graph: &mut CfGraph, root: NodeIndex, new_node: CfgNode) -> NodeIndex {
    let node = graph.add_node(new_node);
    graph.add_edge(root, node, ());
    node
}

pub fn connect_with_merge(
    graph: &mut CfGraph,
    source: Vec<NodeIndex>,
    destination: NodeIndex,
) -> NodeIndex {
    if source.len() == 0 {
        return destination;
    } else if source.len() == 1 {
        graph.add_edge(*source.first().unwrap(), destination, ());
        destination
    } else {
        let merge = graph.add_node(CfgNode::merge());
        connect_children(graph, merge, source, false);
        graph.add_edge(merge, destination, ());
        merge
    }
}

pub fn connect_children(
    graph: &mut CfGraph,
    parent: NodeIndex,
    children: Vec<NodeIndex>,
    rev: bool,
) {
    for child in children {
        if rev {
            graph.add_edge(parent, child, ());
        } else {
            graph.add_edge(child, parent, ());
        }
    }
}
