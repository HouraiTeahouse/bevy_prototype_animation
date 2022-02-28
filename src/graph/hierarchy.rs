use crate::graph::{track::BoneId, AnimationGraph};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_reflect::{GetPath, ReflectMut, TypeRegistry, TypeRegistryArc};
use bevy_tasks::ComputeTaskPool;
use std::ops::Deref;

const BINDING_BATCCH_SIZE: usize = 8;

#[derive(Component)]
pub(crate) struct BoneBinding {
    graph: Entity,
    bone_id: BoneId,
}

// This MUST be used as an exclusive system for aliasing safety.
fn animate_entities_system(
    world: &mut World,
    entities: Query<(Entity, &BoneBinding)>,
    graphs: Query<&AnimationGraph>,
    type_registry: Res<TypeRegistryArc>,
    task_pool: Res<ComputeTaskPool>,
    mut commands: Commands,
) {
    let world: &World = &*world;
    let type_registry = type_registry.read();
    entities.par_for_each(&*task_pool, BINDING_BATCCH_SIZE, |(entity, binding)| {
        animate_entity(entity, binding, &graphs, &type_registry, world);
        // Properly parallelize remove command generation
        // if result.is_none() {
        //     commands.entity(entity).remove::<BoneBinding>();
        // }
    });
}

fn animate_entity(
    entity: Entity,
    binding: &BoneBinding,
    graphs: &Query<&AnimationGraph>,
    type_registry: &TypeRegistry,
    world: &World,
) -> Option<()> {
    let graph = graphs.get(binding.graph).ok()?;
    let bone = graph.get_bone(binding.bone_id)?;
    if bone.entity() != Some(entity) {
        return None;
    }

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
                track.track.blend_via_reflect(&graph.state, field);
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
    Some(())
}
