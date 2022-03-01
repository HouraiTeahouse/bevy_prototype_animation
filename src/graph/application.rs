use crate::graph::{track::BoneId, AnimationGraph};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_reflect::{GetPath, ReflectMut, TypeRegistry, TypeRegistryArc};
use bevy_tasks::ComputeTaskPool;
use dashmap::DashSet;
use std::ops::Deref;

const BINDING_BATCCH_SIZE: usize = 8;

#[derive(Component)]
pub(super) struct BoneBinding {
    pub(super) graph: Entity,
    pub(super) bone_id: BoneId,
}

// This MUST be used as an exclusive system for aliasing safety.
fn animate_entities_system(
    world: &mut World,
    entities: Query<(Entity, &BoneBinding)>,
    graphs: Query<(&AnimationGraph, ChangeTrackers<AnimationGraph>)>,
    type_registry: Res<TypeRegistryArc>,
    task_pool: Res<ComputeTaskPool>,
    dead: Local<DashSet<Entity>>,
    mut commands: Commands,
) {
    debug_assert!(dead.is_empty());

    if graphs.is_empty() {
        for (entity, _) in entities.iter() {
            commands.entity(entity).remove::<BoneBinding>();
        }
        return;
    }

    let world: &World = &*world;
    let type_registry = type_registry.read();
    entities.par_for_each(&*task_pool, BINDING_BATCCH_SIZE, |(entity, binding)| {
        if animate_entity(entity, binding, &graphs, &type_registry, world).is_ok() {
            dead.insert(entity);
        }
    });

    if !dead.is_empty() {
        for entity in dead.iter() {
            commands.entity(*entity).remove::<BoneBinding>();
        }

        dead.clear();
    }
}

enum AnimatePropertyError {
    /// The graph entity no longer has a AnimationGraph or was despawned.
    InvalidAnimationGraph,
    /// The animation graph no longer has corresponding bone.
    InvalidBoundBone,
    /// The binding is no longer valid as the graph has bound to another bone.
    BoneNoLongerBound,
    /// None of the properties that were animated
    NoValidProperties,
}

fn animate_entity(
    entity: Entity,
    binding: &BoneBinding,
    graphs: &Query<(&AnimationGraph, ChangeTrackers<AnimationGraph>)>,
    type_registry: &TypeRegistry,
    world: &World,
) -> Result<(), AnimatePropertyError> {
    let (graph, tracker) = graphs
        .get(binding.graph)
        .map_err(|_| AnimatePropertyError::InvalidAnimationGraph)?;
    let bone = graph
        .get_bone(binding.bone_id)
        .ok_or(AnimatePropertyError::InvalidBoundBone)?;
    if bone.entity() != Some(entity) {
        return Err(AnimatePropertyError::BoneNoLongerBound);
    } else if !tracker.is_changed() {
        // No need to update the components if the upstream graph hasn't changed.
        return Ok(());
    }

    let mut success = false;
    for track in bone.tracks() {
        let property = track.property;
        let component_name = property.component_name();
        let component = type_registry
            .get_with_name(property.component_name())
            .and_then(|registration| registration.data::<ReflectComponent>())
            // SAFE: Each entity is only accessed by one thread at a given time in
            // an exclusive system. Only one component on every is accessed at a
            // given time.
            .and_then(|reflect| unsafe { reflect.reflect_component_unchecked_mut(world, entity) });

        if let Some(mut comp) = component {
            if let Ok(field) = comp.as_mut().path_mut(&property.field_path()) {
                success |= track.track.blend_via_reflect(&graph.state, field).is_ok();
            }
        } else {
            warn!(
                "Failed to animate '{}'. Struct '{}' has no field {}.",
                property.deref(),
                property.component_name(),
                property.field_path(),
            );
        }
    }

    if success {
        Ok(())
    } else {
        Err(AnimatePropertyError::NoValidProperties)
    }
}
