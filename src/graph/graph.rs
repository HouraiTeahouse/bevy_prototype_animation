use crate::AnimationClip;
use std::collections::{HashMap, VecDeque};

// The ID of a node within the graph.
// The root
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u16);

impl NodeId {
    const ROOT: NodeId = NodeId(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(u16);

struct GraphNodes {
    nodes: Vec<Node>,
}

impl GraphNodes {
    pub fn get(&self, node: NodeId) -> Option<&Node> {
        self.nodes.get(node.0 as usize)
    }

    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node.0 as usize)
    }
}

struct GraphState {
    // Individually weighted influences from each active clip.
    influences: HashMap<ClipId, f32>,
}

impl GraphState {
    fn reset(&mut self) {
        self.influences.clear();
    }

    fn add_influence(&mut self, clip: ClipId, delta_weight: f32) {
        if let Some(weight) = self.influences.get_mut(&clip) {
            *weight += delta_weight;
        } else {
            self.influences.insert(clip, delta_weight);
        }
    }
}

pub enum AnimationGraphError {
    NodeNotFound(NodeId),
    InputAlreadyExists(NodeId),
}

pub struct AnimationGraph {
    nodes: GraphNodes,
    state: GraphState,
}

impl AnimationGraph {
    pub fn add_input(
        &mut self,
        target: NodeId,
        input: NodeId,
    ) -> Result<&mut NodeInput, AnimationGraphError> {
        // TODO: Check for cycles before adding edge.

        if self.nodes.get(input).is_none() {
            return Err(AnimationGraphError::NodeNotFound(input));
        }
        let target = if let Some(node) = self.nodes.get_mut(target) {
            node
        } else {
            return Err(AnimationGraphError::NodeNotFound(target));
        };

        if target.get_input_mut(input).is_some() {
            Err(AnimationGraphError::InputAlreadyExists(input))
        } else {
            target.inputs.push(NodeInput::new(input));
            Ok(target.inputs.last_mut().unwrap())
        }
    }

    //     pub fn add_clip(&mut self, target: AnimationClip) -> Option<NodeId> {}

    /// Sets the time for a given node. If the node is set to propagate its
    /// time, all of it's currently connected inputs will also have the time
    /// propagated to them as well.
    pub fn set_time(&mut self, node_id: NodeId, time: f32) -> Result<(), AnimationGraphError> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.local_time = time;
            if !node.propogate_time {
                return Ok(());
            }
        } else {
            return Err(AnimationGraphError::NodeNotFound(node_id));
        }

        //
        // TODO: Cache this to avoid allocations in the future.
        let mut pending = VecDeque::new();
        pending.push_back(node_id);
        while let Some(node_id) = pending.pop_front() {
            if let Some(node) = self.nodes.get(node_id) {
                let node = if let Some(node) = self.nodes.get_mut(node_id) {
                    node.local_time = time;
                    node
                } else {
                    continue;
                };
                if node.propogate_time {
                    pending.extend(node.connected_inputs().map(|input| input.node_id));
                }
            }
        }

        Ok(())
    }

    /// Evaluates the graph, computing the influences individual results.
    pub fn evaluate(&mut self) {
        self.state.reset();
        if let Some(root) = self.nodes.get(NodeId::ROOT) {
            root.compute_influences(1.0, &self.nodes, &mut self.state);
        }
    }
}

struct Node {
    local_time: f32,
    duration: f32,
    inputs: Vec<NodeInput>,
    // whether or not to propogate the time metric downstream
    propogate_time: bool,
    clip: Option<ClipId>,
}

impl Node {
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

    fn compute_influences(&self, weight: f32, nodes: &GraphNodes, state: &mut GraphState) {
        if let Some(clip) = self.clip {
            state.add_influence(clip, weight);
        }
        for input in self.connected_inputs() {
            if input.weight == 0.0 {
                continue;
            }
            if let Some(node) = nodes.get(input.node_id) {
                node.compute_influences(input.weight * weight, nodes, state);
            }
        }
    }
}

pub struct NodeInput {
    node_id: NodeId,
    connected: bool,
    weight: f32,
}

impl NodeInput {
    fn new(node_id: NodeId) -> Self {
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

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight
    }
}
