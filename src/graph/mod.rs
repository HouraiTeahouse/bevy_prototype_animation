mod node;

use node::*;

use crate::AnimationClip;
use bevy_asset::Handle;
use std::collections::{HashMap, VecDeque};

pub(self) struct GraphState {
    // Individually weighted influences from each active clip.
    influences: HashMap<Handle<AnimationClip>, f32>,
}

impl GraphState {
    fn reset(&mut self) {
        self.influences.clear();
    }

    fn add_influence(&mut self, clip: Handle<AnimationClip>, delta_weight: f32) {
        if let Some(weight) = self.influences.get_mut(&clip) {
            *weight += delta_weight;
        } else {
            self.influences.insert(clip, delta_weight);
        }
    }
}

struct GraphTraversalNode {
    node_id: NodeId,
    cumulative_weight: f32,
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

        self.nodes
            .get(input)
            .ok_or(AnimationGraphError::NodeNotFound(input))?;

        let target = self
            .nodes
            .get_mut(target)
            .ok_or(AnimationGraphError::NodeNotFound(target))?;

        if target.get_input_mut(input).is_some() {
            Err(AnimationGraphError::InputAlreadyExists(input))
        } else {
            target.inputs.push(NodeInput::new(input));
            Ok(target.inputs.last_mut().unwrap())
        }
    }

    pub fn add_clip(&mut self, clip: Handle<AnimationClip>) -> NodeId {
        // TODO: Find a way to check for duplicate leaf clip nodes
        self.nodes.add(Node::create_leaf(clip))
    }

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
                    pending.extend(node.connected_inputs().map(|input| input.node_id()));
                }
            }
        }

        Ok(())
    }

    /// Evaluates the graph, computing the influences individual results.
    pub fn evaluate(&mut self) {
        self.state.reset();

        // TODO: Use smallvec to avoid allocation here.
        let stack = vec![GraphTraversalNode {
            node_id: NodeId::ROOT,
            cumulative_weight: 1.0,
        }];

        // Conduct a depth-first traversal of the graph multiplying the weights
        // as it gets deeper into the tree.
        while let Some(current) = stack.pop() {
            let current_node = if let Some(node) = self.nodes.get(current.node_id) {
                node
            } else {
                continue;
            };

            if let Some(clip) = current.clip {
                self.state.add_influence(clip, current_node.weight);
            }

            for input in self.connected_inputs() {
                let cumulative_weight = input.weight * current_node.cumulative_weight;
                if cumulative_weight != 0.0 {
                    stack.push(GraphTraversalNode {
                        node_id: input.node_id,
                        cumulative_weight,
                    });
                }
            }
        }
    }
}
