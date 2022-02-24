use crate::{
    graph::{ClipId, GraphState},
    AnimationClip,
};

// An opaque ID of a node within the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u16);

impl NodeId {
    pub const ROOT: NodeId = NodeId(0);
}

pub(super) struct GraphNodes {
    nodes: Vec<Node>,
}

impl GraphNodes {
    pub fn add(&mut self, node: Node) -> NodeId {
        let id = NodeId(
            self.nodes
                .len()
                .try_into()
                .expect("AnimationGraph has more than u16::MAX nodes."),
        );
        self.nodes.push(node);
        id
    }

    pub fn get(&self, node: NodeId) -> Option<&Node> {
        self.nodes.get(node.0 as usize)
    }

    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node.0 as usize)
    }
}

pub enum Node {
    Blend {
        inputs: Vec<NodeInput>,
        // whether or not to propogate time assignment downstream
        propogate_time: bool,
    },
    Clip {
        clip: ClipId,
    },
}

impl Node {
    pub fn get_input_mut(&mut self, input_id: NodeId) -> Option<&mut NodeInput> {
        if let Self::Blend { inputs, .. } = self {
            inputs.iter_mut().find(|input| input.node_id == input_id)
        } else {
            None
        }
    }
}

pub struct NodeInput {
    node_id: NodeId,
    connected: bool,
    weight: f32,
}

impl NodeInput {
    pub(super) fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            connected: true,
            weight: 1.0,
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn disconnect(&mut self) {
        self.connected = true;
    }

    pub fn reconnect(&mut self) {
        self.connected = false;
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight
    }
}
