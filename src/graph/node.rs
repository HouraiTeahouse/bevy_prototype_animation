use crate::{AnimationClip, graph::GraphState};
use bevy_asset::Handle;

// The ID of a node within the graph.
// The root
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct NodeId(u16);

impl NodeId {
    pub const ROOT: NodeId = NodeId(0);
}

pub(super) struct GraphNodes {
    nodes: Vec<Node>,
}

impl GraphNodes {
    pub fn add(&mut self, node: Node) -> NodeId {
        self.nodes.push(node);
        let id = self.nodes.len() - 1;
        NodeId(
            id.try_into()
                .expect("AnimationGraph has more than u16::MAX nodes."),
        )
    }

    pub fn get(&self, node: NodeId) -> Option<&Node> {
        self.nodes.get(node.0 as usize)
    }

    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node.0 as usize)
    }
}

pub struct Node {
    pub(crate) local_time: f32,
    pub(crate) inputs: Vec<NodeInput>,
    // whether or not to propogate the time metric downstream
    pub(crate) propogate_time: bool,
    pub(crate) clip: Option<Handle<AnimationClip>>,
}

impl Node {
    pub(super) fn create_leaf(clip: Handle<AnimationClip>) -> Self {
        Self {
            local_time: 0.0,
            inputs: Vec::new(),
            propogate_time: false,
            clip: Some(clip),
        }
    }

    pub fn get_input(&self, input_id: NodeId) -> Option<&NodeInput> {
        self.inputs.iter().find(|input| input.node_id == input_id)
    }

    pub fn get_input_mut(&mut self, input_id: NodeId) -> Option<&mut NodeInput> {
        self.inputs
            .iter_mut()
            .find(|input| input.node_id == input_id)
    }

    pub fn connected_inputs(&self) -> impl Iterator<Item = &NodeInput> {
        self.inputs.iter().filter(|input| input.connected)
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
