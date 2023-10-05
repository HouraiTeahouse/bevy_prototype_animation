#[cfg(test)]
#[macro_use]
extern crate static_assertions;

mod animatable;
pub mod clip;
pub mod curve;
pub mod graph;
pub mod path;
mod util;

pub mod prelude {
    pub use crate::{clip::AnimationClip, curve::Curve, graph::AnimationGraph};
}

use crate::prelude::*;
pub use animatable::*;
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_transform::TransformSystem;

#[derive(Clone, Debug, SystemSet, PartialEq, Eq, Hash)]
pub enum AnimationSystem {
    GraphEvaluation,
    GraphHierarchyBind,
    GraphSamplingSkeletal,
    GraphSamplingGeneric,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        // TODO: I think this is correct?
        app.init_asset::<clip::AnimationClip>()
            .add_systems(
                PostUpdate,
                (evaluate_graph_system).in_set(AnimationSystem::GraphEvaluation),
            )
            .add_systems(
                PostUpdate,
                (graph::hierarchy::bind_hierarchy_system)
                    .in_set(AnimationSystem::GraphHierarchyBind)
            )
            .add_systems(
                PostUpdate,
                (graph::application::animate_entities_system)
                    .in_set(AnimationSystem::GraphSamplingGeneric)
                    .after(AnimationSystem::GraphHierarchyBind)
                    .after(AnimationSystem::GraphEvaluation)
                    .before(TransformSystem::TransformPropagate),
            );

        // .add_systems(evaluate_graph_system.label(AnimationSystem::GraphEvaluation))
        // .add_systems(
        //     graph::hierarchy::bind_hierarchy_system
        //         .label(AnimationSystem::GraphHierarchyBind)
        //         .after(AnimationSystem::TransformSystem::ParentUpdate),
        // )
        // .add_systems(
        //     graph::application::animate_entities_system
        //         .exclusive_system()
        //         .label(AnimationSystem::GraphSamplingGeneric)
        //         .after(AnimationSystem::GraphHierarchyBind)
        //         .after(AnimationSystem::GraphEvaluation)
        //         .before(TransformSystem::TransformPropagate),
        // );
    }
}

/// Evaluates all altered [`AnimationGraph`]s and updates it's internal state.
pub fn evaluate_graph_system(mut graphs: Query<&mut AnimationGraph, Changed<AnimationGraph>>) {
    for mut graph in graphs.iter_mut() {
        graph.evaluate();
    }
}
