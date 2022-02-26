#[cfg(test)]
#[macro_use]
extern crate static_assertions;

mod animatable;
pub mod clip;
pub mod curve;
pub mod graph;
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
    GraphSamplingSkeletal,
    GraphSamplingGeneric,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<clip::AnimationClip>()
            .add_system(evaluate_graph_system.label(AnimationSystem::GraphEvaluation))
            .add_system(
                sample_graphs_skeletal_system
                    .label(AnimationSystem::GraphSamplingSkeletal)
                    .after(AnimationSystem::GraphEvaluation)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_system(
                sample_graphs_generic_system
                    .exclusive_system()
                    .label(AnimationSystem::GraphSamplingGeneric)
                    .after(AnimationSystem::GraphEvaluation)
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

/// Evaluates all altered [`AnimationGraph`]s and updates it's internal state.
pub fn evaluate_graph_system(mut graphs: Query<&mut AnimationGraph, Changed<AnimationGraph>>) {
    for mut graph in graphs.iter_mut() {
        graph.evaluate();
    }
}

pub fn sample_graphs_skeletal_system(
    graphs: Query<&AnimationGraph, Changed<AnimationGraph>>,
    transforms: Query<&mut Transform>,
) {
}

/// Samples the current state of all updated [`AnimationGraph`]s and applies the sampled values
/// to the applicable
///
/// This must be used as an exclusive system due to
pub fn sample_graphs_generic_system(world: &mut World) {}
