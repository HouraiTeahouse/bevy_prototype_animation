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
use bevy_transform::{prelude::*, TransformSystem};

#[derive(Clone, Debug, SystemLabel, PartialEq, Eq, Hash)]
pub enum AnimationSystem {
    GraphEvaluation,
    GraphHierarchyDirtyCheck,
    GraphHierarchyBind,
    GraphSamplingSkeletal,
    GraphSamplingGeneric,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<clip::AnimationClip>()
            .add_system(evaluate_graph_system.label(AnimationSystem::GraphEvaluation))
            .add_system(
                graph::hierarchy::dirty_hierarchy_system
                    .label(AnimationSystem::GraphHierarchyDirtyCheck),
            )
            .add_system(
                graph::hierarchy::bind_hierarchy_system.label(AnimationSystem::GraphHierarchyBind),
            )
            .add_system(
                graph::application::animate_entities_system
                    .exclusive_system()
                    .label(AnimationSystem::GraphSamplingGeneric)
                    .after(AnimationSystem::GraphHierarchyBind)
                    .after(AnimationSystem::GraphEvaluation),
            );
    }
}

/// Evaluates all altered [`AnimationGraph`]s and updates it's internal state.
pub fn evaluate_graph_system(mut graphs: Query<&mut AnimationGraph, Changed<AnimationGraph>>) {
    for mut graph in graphs.iter_mut() {
        graph.evaluate();
    }
}
