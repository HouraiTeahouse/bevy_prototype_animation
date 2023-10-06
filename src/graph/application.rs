use crate::graph::{track::BoneId, AnimationGraph};
use bevy_ecs::{prelude::*, system::SystemState, world::unsafe_world_cell::UnsafeWorldCell};
use bevy_log::warn;
use bevy_reflect::{TypeRegistry, TypeRegistryArc};
use bevy_tasks::ComputeTaskPool;
use dashmap::DashSet;
use std::ops::Deref;

const BINDING_BATCCH_SIZE: usize = 8;

#[derive(Component)]
pub(crate) struct BoneBinding {
    pub(super) graph: Entity,
    pub(super) bone_id: BoneId,
}

// This MUST be used as an exclusive system for aliasing safety.
// The immutable reference to the a World is used mutably in an unsafe
// manner if simultaneous World access is allowed.
pub(crate) fn animate_entities_system(
    world: &mut World,
    state: &mut SystemState<(
        Query<(Entity, &BoneBinding)>,
        Query<Ref<AnimationGraph>>,
        Res<AppTypeRegistry>,
        Commands,
    )>,
    // entities: Query<(Entity, &BoneBinding)>,
    // graphs: Query<Ref<AnimationGraph>>,
    // type_registry: Res<AppTypeRegistry>,
    // TODO: I'm not sure if this even works.
    dead: Local<DashSet<Entity>>,
    // mut commands: Commands,
) {
    let worldcell = world.as_unsafe_world_cell();

    // TODO: Should this be cached for perf reasons?
    let (entities, graphs, type_registry, mut commands) =
        unsafe { state.get_unchecked_manual(worldcell) };
    debug_assert!(dead.is_empty());

    if graphs.is_empty() {
        for (entity, _) in entities.iter() {
            commands.entity(entity).remove::<BoneBinding>();
        }
        return;
    }

    let type_registry = type_registry.read();
    entities.par_iter().for_each(|(entity, binding)| {
        if animate_entity(entity, binding, &graphs, &type_registry, &worldcell).is_ok() {
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
    graphs: &Query<Ref<AnimationGraph>>,
    type_registry: &TypeRegistry,
    world: &UnsafeWorldCell,
) -> Result<(), AnimatePropertyError> {
    let graph = graphs
        .get(binding.graph)
        .map_err(|_| AnimatePropertyError::InvalidAnimationGraph)?;
    let bone = graph
        .get_bone(binding.bone_id)
        .ok_or(AnimatePropertyError::InvalidBoundBone)?;
    if bone.entity() != Some(entity) {
        return Err(AnimatePropertyError::BoneNoLongerBound);
    } else if !graph.is_changed() {
        // No need to update the components if the upstream graph hasn't changed.
        return Ok(());
    }
    // TODO: Can we pass in EntityWorldMut or an UnsafeEntityCell instead of Entity?
    let entitymut = world.get_entity(entity).unwrap();

    let mut success = false;
    // TODO: Is there a way to make this more efficient? see https://github.com/bevyengine/bevy/issues/4985
    for track in bone.tracks() {
        let property = track.property;
        let component = type_registry
            .get(property.component_type_id())
            .and_then(|registration| registration.data::<ReflectComponent>())
            // SAFE: Each entity is only accessed by one thread at a given time in
            // an exclusive system. Only one component on every is accessed at a
            // given time.
            //
            // The blend_via_reflect call below will cause simultaneous read-only
            // access of Resources in a read-only fashion. There are no aliasing
            // issues as this mutation only affects components.
            .and_then(|reflect| unsafe { reflect.reflect_unchecked_mut(entitymut) });

        if let Some(mut comp) = component {
            if let Ok(field) = property.field_path().field_mut(comp.as_reflect_mut()) {
                // SAFE: This access is read-only and is required to only access
                // resources. This cannot cause race conditions as only non-Resource
                // components are mutated.
                success |= unsafe {
                    track
                        .track
                        .blend_via_reflect(&graph.state, field, world.world())
                        .is_ok()
                };
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
